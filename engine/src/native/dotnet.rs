//! Sole production .NET process boundary. It receives generated C# and Rust's
//! validated typed values; it never reads YAML or publishes canonical artifacts.
use crate::{Error, Result, delivery::BuildPlan};
use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{Read, Write},
    path::Path,
    process::{Command, Stdio},
    time::{Duration, Instant},
};

pub struct Built {
    pub binary: Vec<u8>,
    pub native_evidence: String,
}
fn io(error: std::io::Error) -> Error {
    Error::new("E-DOTNET-IO", error.to_string())
}
const PROJECT: &str = r#"<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <TargetFramework>net8.0</TargetFramework><OutputType>Exe</OutputType>
    <Nullable>enable</Nullable><LangVersion>11</LangVersion>
    <RootNamespace>Masterdata.Generated</RootNamespace>
    <Deterministic>true</Deterministic><AssemblyName>NativeBuilder</AssemblyName>
    <RestorePackagesWithLockFile>true</RestorePackagesWithLockFile>
  </PropertyGroup>
  <ItemGroup>
    <PackageReference Include="MasterMemory" Version="3.0.4" />
    <PackageReference Include="MessagePack" Version="$MESSAGEPACK$" />
  </ItemGroup>
</Project>
"#;
fn stage(
    root: &Path,
    csharp: &BTreeMap<String, String>,
    program: &str,
    messagepack: &str,
) -> Result<()> {
    let generated = root.join("Generated");
    fs::create_dir(&generated).map_err(io)?;
    for (path, bytes) in csharp {
        let path = crate::project::relative_safe(path)?;
        if path.components().count() != 1 {
            return Err(Error::new("E-CODEGEN", "flat generated file required"));
        }
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(generated.join(path))
            .map_err(io)?;
        file.write_all(bytes.as_bytes()).map_err(io)?;
    }
    fs::write(
        root.join("NativeBuilder.csproj"),
        PROJECT.replace("$MESSAGEPACK$", messagepack),
    )
    .map_err(io)?;
    fs::write(root.join("NuGet.Config"),"<configuration><packageSources><clear/><add key=\"nuget.org\" value=\"https://api.nuget.org/v3/index.json\"/></packageSources></configuration>").map_err(io)?;
    fs::write(root.join("Program.cs"), program).map_err(io)?;
    Ok(())
}
fn invoke(root: &Path, arguments: &[&std::ffi::OsStr], label: &str) -> Result<String> {
    let log = root.join(format!("{label}.log"));
    let file = File::create(&log).map_err(io)?;
    let mut child = Command::new("dotnet")
        .args(arguments)
        .current_dir(root)
        .env("DOTNET_CLI_TELEMETRY_OPTOUT", "1")
        .stdin(Stdio::null())
        .stdout(Stdio::from(file.try_clone().map_err(io)?))
        .stderr(Stdio::from(file))
        .spawn()
        .map_err(|error| Error::new("E-DOTNET-UNAVAILABLE", error.to_string()))?;
    let start = Instant::now();
    let status = loop {
        if let Some(status) = child.try_wait().map_err(io)? {
            break status;
        }
        if start.elapsed() > Duration::from_secs(300) {
            let _ = child.kill();
            let _ = child.wait();
            return Err(Error::new(
                "E-DOTNET-TIMEOUT",
                format!(
                    "{label}: native process did not complete within 300s; staged output was not published"
                ),
            ));
        }
        std::thread::sleep(Duration::from_millis(20));
    };
    let mut bytes = vec![];
    File::open(log)
        .map_err(io)?
        .take(64 * 1024)
        .read_to_end(&mut bytes)
        .map_err(io)?;
    let output = String::from_utf8_lossy(&bytes).into_owned();
    if !status.success() {
        return Err(Error::new(
            "E-DOTNET-FAILURE",
            format!("{label}: {status}\n{output}"),
        ));
    }
    Ok(output)
}
fn compile(root: &Path) -> Result<String> {
    invoke(
        root,
        &[
            "build".as_ref(),
            "NativeBuilder.csproj".as_ref(),
            "--configuration".as_ref(),
            "Release".as_ref(),
            "--output".as_ref(),
            "bin".as_ref(),
            "--nologo".as_ref(),
        ],
        "compile",
    )
}
pub fn build(plan: &BuildPlan) -> Result<Built> {
    let staged = tempfile::Builder::new()
        .prefix("masterdata-native-build-")
        .tempdir()
        .map_err(io)?;
    stage(staged.path(), plan.csharp(), &plan.builder(), "3.1.11")?;
    let input = staged.path().join("request.json");
    plan.write_request(File::create(&input).map_err(io)?)?;
    let compile = compile(staged.path())?;
    let output = staged.path().join("masterdata.bytes");
    let evidence = invoke(
        staged.path(),
        &[
            "bin/NativeBuilder.dll".as_ref(),
            input.as_os_str(),
            output.as_os_str(),
        ],
        "build-reload",
    )?;
    if !evidence.contains("PASS native MasterMemory Build / actual reload / every selected row") {
        return Err(Error::new(
            "E-DOTNET-RELOAD",
            "native reload evidence missing",
        ));
    }
    let binary = fs::read(output).map_err(io)?;
    if binary.is_empty() {
        return Err(Error::new(
            "E-DOTNET-BINARY",
            "native builder returned an empty artifact",
        ));
    }
    Ok(Built {
        binary,
        native_evidence: format!("{compile}\n{evidence}"),
    })
}

#[cfg(feature = "native-consumer")]
pub fn verify_consumer(
    csharp: &BTreeMap<String, String>,
    binary: &[u8],
    consumer: &Path,
    api_checks: &str,
) -> Result<String> {
    let staged = tempfile::Builder::new()
        .prefix("masterdata-independent-consumer-")
        .tempdir()
        .map_err(io)?;
    // Canonical Unity consumer version is tested independently of the builder's
    // current MessagePack dependency. Neither consumes source YAML or Rust APIs.
    stage(
        staged.path(),
        csharp,
        &format!(
            "RewriteOracle.Consumer.Check(global::System.IO.File.ReadAllBytes(args[0]));\n{api_checks}"
        ),
        "3.1.3",
    )?;
    fs::copy(consumer, staged.path().join("Consumer.cs")).map_err(io)?;
    let output = staged.path().join("masterdata.bytes");
    fs::write(&output, binary).map_err(io)?;
    let compiled = compile(staged.path())?;
    let result = invoke(
        staged.path(),
        &["bin/NativeBuilder.dll".as_ref(), output.as_os_str()],
        "consumer",
    )?;
    Ok(format!("{compiled}\n{result}"))
}
