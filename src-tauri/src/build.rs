use crate::files::{digest, optional_read, optional_read_bytes, safe_path, Workspace};
use crate::model::{
    fields_of, tables, type_kind, type_source, type_sources, uid, BuildResult, Field,
    PublishConfig, Snapshot, TableInfo, PRIMITIVES,
};
use crate::schema::in_profile;
use crate::validation::{public_name, validate};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use indexmap::IndexMap;
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub fn dotnet_executable() -> Result<PathBuf, String> {
    dotnet_executable_with_env(
        std::env::var("PATH").ok().as_deref(),
        std::env::var("DOTNET_ROOT").ok().as_deref(),
        std::env::var("ProgramFiles").ok().as_deref(),
    )
}

pub fn dotnet_executable_with_env(
    path_var: Option<&str>,
    dotnet_root: Option<&str>,
    program_files: Option<&str>,
) -> Result<PathBuf, String> {
    let filename = if cfg!(windows) {
        "dotnet.exe"
    } else {
        "dotnet"
    };
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(p) = path_var {
        for dir in std::env::split_paths(p) {
            if !dir.as_os_str().is_empty() {
                candidates.push(dir.join(filename));
            }
        }
    }
    if let Some(root) = dotnet_root {
        if !root.is_empty() {
            candidates.push(PathBuf::from(root).join(filename));
        }
    }
    if cfg!(windows) {
        let pf = program_files.unwrap_or("C:\\Program Files");
        candidates.push(PathBuf::from(pf).join("dotnet").join(filename));
    } else {
        for fixed in [
            "/usr/local/share/dotnet/dotnet",
            "/opt/homebrew/bin/dotnet",
            "/usr/share/dotnet/dotnet",
            "/usr/local/bin/dotnet",
            "/usr/bin/dotnet",
        ] {
            candidates.push(PathBuf::from(fixed));
        }
    }
    for candidate in candidates {
        if fs::metadata(&candidate).map(|m| m.is_file()).unwrap_or(false) {
            return Ok(candidate);
        }
    }
    Err(".NET SDKが見つかりません。.NET 8以降のSDKをインストールしてください。編集と保存はそのまま利用できます。".to_string())
}

fn quote_cs(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| format!("\"{s}\""))
}

fn class_name(table: &TableInfo) -> String {
    table
        .source
        .data
        .get("csharpName")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| public_name(&table.name))
}

fn base_type(t: &str) -> String {
    if PRIMITIVES.contains(&t) {
        t.to_string()
    } else {
        format!("global::Masterdata.Generated.{t}")
    }
}

fn field_type(f: &Field) -> String {
    format!(
        "{}{}{}",
        base_type(&f.r#type),
        if f.array { "[]" } else { "" },
        if f.nullable { "?" } else { "" }
    )
}

fn props_cs(fields: &[Field], table: Option<&TableInfo>) -> String {
    fields
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let mut annotations = vec![format!("[Key({})]", f.key.unwrap_or(i as i64))];
            if let Some(tbl) = table {
                if let Some(pk_arr) = tbl
                    .source
                    .data
                    .get("primaryKey")
                    .and_then(|p| p.get("fields"))
                    .and_then(|v| v.as_array())
                {
                    if let Some(pk_order) =
                        pk_arr.iter().position(|x| x.as_str() == Some(f.name.as_str()))
                    {
                        annotations.push(format!("[PrimaryKey(keyOrder: {pk_order})]"));
                    }
                }
                if let Some(sk_arr) = tbl
                    .source
                    .data
                    .get("secondaryKeys")
                    .and_then(|v| v.as_array())
                {
                    for (sk_index, sk) in sk_arr.iter().enumerate() {
                        if let Some(f_arr) = sk.get("fields").and_then(|v| v.as_array()) {
                            if let Some(order) = f_arr
                                .iter()
                                .position(|x| x.as_str() == Some(f.name.as_str()))
                            {
                                let non_unique = sk
                                    .get("nonUnique")
                                    .and_then(|v| v.as_bool())
                                    .unwrap_or(false);
                                annotations.push(format!(
                                    "[SecondaryKey({sk_index}, keyOrder: {order}){}]",
                                    if non_unique { ", NonUnique" } else { "" }
                                ));
                            }
                        }
                    }
                }
            }
            let init = if (f.r#type == "string" || f.array) && !f.nullable {
                " = null!;"
            } else {
                ""
            };
            format!(
                "  {}\n  public {} {} {{ get; set; }}{}",
                annotations.join("\n  "),
                field_type(f),
                public_name(&f.name),
                init
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub struct GeneratedBuild {
    pub files: IndexMap<String, String>,
    pub program: String,
    pub input: String,
}

pub fn generate(snapshot: &Snapshot) -> GeneratedBuild {
    let types = type_sources(snapshot);
    let all_tables = tables(snapshot);
    let mut files = IndexMap::new();
    let prefix = "// Generated by MasterData. Do not edit.\n#nullable enable\nusing MasterMemory;\nusing MessagePack;\nnamespace Masterdata.Generated;\n";

    for source in &types {
        let Some(kind) = type_kind(Some(source)) else {
            continue;
        };
        let name = source.data.get("name").map(|v| v.text()).unwrap_or_default();
        let d = source.data.get(kind).cloned().unwrap_or(crate::model::Data::Null);
        let mut code = prefix.to_string();
        if kind == "enum" || kind == "flags" {
            let underlying = d
                .get("underlying")
                .and_then(|v| v.as_str())
                .unwrap_or("int");
            let suffix = match underlying {
                "ulong" => "UL",
                "uint" => "U",
                "long" => "L",
                _ => "",
            };
            let members = d
                .get("members")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            let member_lines: Vec<String> = members
                .iter()
                .map(|m| {
                    let m_name = m.get("name").map(|v| v.text()).unwrap_or_default();
                    let m_val = m.get("value").map(|v| v.text()).unwrap_or_else(|| "0".to_string());
                    format!("  {m_name} = {m_val}{suffix}")
                })
                .collect();
            code.push_str(&format!(
                "{}public enum {name} : {underlying} {{\n{}\n}}\n",
                if kind == "flags" {
                    "[System.Flags]\n"
                } else {
                    ""
                },
                member_lines.join(",\n")
            ));
        } else if kind == "custom" {
            code.push_str(&format!(
                "[MessagePackObject]\npublic partial struct {name} {{\n  public {name}() {{}}\n{}\n}}\n",
                props_cs(&fields_of(source), None)
            ));
        } else {
            let underlying = d
                .get("underlying")
                .and_then(|v| v.as_str())
                .unwrap_or("int");
            let bt = base_type(underlying);
            code.push_str(&format!(
                "[MessagePackObject]\npublic readonly partial struct {name} : System.IEquatable<{name}>, System.IComparable<{name}> {{\n  [Key(0)] public {bt} Value {{ get; }}\n  [SerializationConstructor] public {name}({bt} value) {{ Value = value; }}\n  public bool Equals({name} other) => System.Collections.Generic.EqualityComparer<{bt}>.Default.Equals(Value, other.Value);\n  public int CompareTo({name} other) => System.Collections.Generic.Comparer<{bt}>.Default.Compare(Value, other.Value);\n  public override bool Equals(object? obj) => obj is {name} other && Equals(other);\n  public override int GetHashCode() => System.Collections.Generic.EqualityComparer<{bt}>.Default.GetHashCode(Value);\n  public override string ToString() => Value.ToString() ?? \"\";\n}}\n"
            ));
        }
        files.insert(format!("{name}.g.cs"), code);
    }

    for table in &all_tables {
        let name = class_name(table);
        let mut helpers = String::new();
        if let Some(refs) = table
            .source
            .data
            .get("references")
            .and_then(|v| v.as_array())
        {
            for reference in refs {
                let target_table_name = reference
                    .get("target")
                    .and_then(|t| t.get("table"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let Some(target) = all_tables.iter().find(|t| t.name == target_table_name) else {
                    continue;
                };
                let target_fields: Vec<String> = reference
                    .get("target")
                    .and_then(|t| t.get("fields"))
                    .and_then(|v| v.as_array())
                    .map(|a| a.iter().map(|x| x.text()).collect())
                    .unwrap_or_default();
                let own_fields: Vec<String> = reference
                    .get("fields")
                    .and_then(|v| v.as_array())
                    .map(|a| a.iter().map(|x| x.text()).collect())
                    .unwrap_or_default();

                let mut candidate_keys = Vec::new();
                if let Some(pk) = target.source.data.get("primaryKey") {
                    candidate_keys.push(pk);
                }
                if let Some(sks) = target
                    .source
                    .data
                    .get("secondaryKeys")
                    .and_then(|v| v.as_array())
                {
                    for sk in sks {
                        candidate_keys.push(sk);
                    }
                }
                let target_key = candidate_keys.into_iter().find(|k| {
                    let kf: Vec<String> = k
                        .get("fields")
                        .and_then(|v| v.as_array())
                        .map(|a| a.iter().map(|x| x.text()).collect())
                        .unwrap_or_default();
                    kf == target_fields
                });
                let non_unique = target_key
                    .and_then(|k| k.get("nonUnique"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let optional = own_fields.iter().all(|n| {
                    table
                        .fields
                        .iter()
                        .find(|f| &f.name == n)
                        .map(|f| f.nullable)
                        .unwrap_or(false)
                });
                let target_type = format!("global::Masterdata.Generated.{}", class_name(target));
                let ret = if non_unique {
                    format!("global::MasterMemory.RangeView<{target_type}>")
                } else {
                    format!("{target_type}{}", if optional { "?" } else { "" })
                };
                let args: Vec<String> = own_fields
                    .iter()
                    .map(|n| {
                        let f = table.fields.iter().find(|f| &f.name == n).unwrap();
                        let prop = public_name(n);
                        if optional && f.r#type != "string" && !f.array {
                            format!("{prop}.Value")
                        } else {
                            prop
                        }
                    })
                    .collect();
                let tuple = if args.len() == 1 {
                    args[0].clone()
                } else {
                    format!("({})", args.join(", "))
                };
                let ref_name = reference
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let method_name = reference
                    .get("csharpName")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .unwrap_or_else(|| format!("Get{}", public_name(ref_name)));
                let null_guard = if optional {
                    let cond = own_fields
                        .iter()
                        .map(|n| format!("{} is null", public_name(n)))
                        .collect::<Vec<_>>()
                        .join(" && ");
                    let fallback = if non_unique {
                        format!("{ret}.Empty")
                    } else {
                        "null".to_string()
                    };
                    format!("    if ({cond}) return {fallback};\n")
                } else {
                    String::new()
                };
                let find_by = target_fields
                    .iter()
                    .map(|s| public_name(s))
                    .collect::<Vec<_>>()
                    .join("And");
                helpers.push_str(&format!(
                    "\n  public {ret} {method_name}(MemoryDatabase database) {{\n{null_guard}    return database.{}Table.FindBy{find_by}({tuple});\n  }}\n",
                    class_name(target)
                ));
            }
        }
        files.insert(
            format!("{name}.g.cs"),
            format!(
                "{prefix}[MemoryTable({}), MessagePackObject]\npublic sealed partial class {name} {{\n{}{helpers}\n}}\n",
                quote_cs(&table.name),
                props_cs(&table.fields, Some(table))
            ),
        );
    }

    let mut variable = 0usize;
    fn read_expr(snapshot: &Snapshot, f: &Field, e: &str, variable: &mut usize) -> String {
        if f.nullable {
            let mut non_null = f.clone();
            non_null.nullable = false;
            return format!(
                "({e}.ValueKind == JsonValueKind.Null ? ({})null : {})",
                field_type(f),
                read_expr(snapshot, &non_null, e, variable)
            );
        }
        if f.array {
            let v = format!("element{}", *variable);
            *variable += 1;
            let mut elem = f.clone();
            elem.array = false;
            return format!(
                "{e}.EnumerateArray().Select({v} => {}).ToArray()",
                read_expr(snapshot, &elem, &v, variable)
            );
        }
        let t_src = type_source(snapshot, &f.r#type);
        let kind = type_kind(t_src.as_ref());
        if kind == Some("valueObject") {
            let underlying = t_src
                .as_ref()
                .and_then(|s| s.data.get("valueObject"))
                .and_then(|v| v.get("underlying"))
                .and_then(|v| v.as_str())
                .unwrap_or("int")
                .to_string();
            let inner = Field {
                name: String::new(),
                r#type: underlying,
                key: None,
                array: false,
                nullable: false,
            };
            return format!(
                "new {}({})",
                base_type(&f.r#type),
                read_expr(snapshot, &inner, e, variable)
            );
        }
        if kind == Some("custom") {
            let props = fields_of(t_src.as_ref().unwrap())
                .iter()
                .map(|child| {
                    format!(
                        "{} = {}",
                        public_name(&child.name),
                        read_expr(
                            snapshot,
                            child,
                            &format!("{e}.GetProperty({})", quote_cs(&child.name)),
                            variable
                        )
                    )
                })
                .collect::<Vec<_>>()
                .join(", ");
            return format!("new {} {{ {props} }}", base_type(&f.r#type));
        }
        if kind == Some("enum") {
            return format!("Enum.Parse<{}>({e}.GetString()!)", base_type(&f.r#type));
        }
        if kind == Some("flags") {
            return format!(
                "Enum.Parse<{}>(string.Join(\",\", {e}.EnumerateArray().Select(v => v.GetString())))",
                base_type(&f.r#type)
            );
        }
        if f.r#type == "string" {
            return format!("{e}.GetString()!");
        }
        if f.r#type == "bool" {
            return format!("{e}.GetBoolean()");
        }
        format!(
            "{}.Parse({e}.ToString(), System.Globalization.CultureInfo.InvariantCulture)",
            f.r#type
        )
    }

    let mut body = String::new();
    for table in &all_tables {
        let name = class_name(table);
        let inits = table
            .fields
            .iter()
            .map(|f| {
                format!(
                    "{} = {}",
                    public_name(&f.name),
                    read_expr(
                        snapshot,
                        f,
                        &format!("input.GetProperty({})", quote_cs(&f.name)),
                        &mut variable
                    )
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        body.push_str(&format!(
            "var rows{name} = new List<{name}>();\nforeach(var input in document.RootElement.GetProperty({}).EnumerateArray()) rows{name}.Add(new {name} {{ {inits} }});\nbuilder.Append(rows{name});\n",
            quote_cs(&table.name)
        ));
    }
    body.push_str("var bytes = builder.Build();\nvar database = new MemoryDatabase(bytes);\n");
    for table in &all_tables {
        let name = class_name(table);
        let keys: Vec<String> = table
            .source
            .data
            .get("primaryKey")
            .and_then(|p| p.get("fields"))
            .and_then(|v| v.as_array())
            .map(|a| a.iter().map(|x| x.text()).collect())
            .unwrap_or_default();
        let tuple = if keys.len() == 1 {
            format!("expected.{}", public_name(&keys[0]))
        } else {
            format!(
                "({})",
                keys.iter()
                    .map(|k| format!("expected.{}", public_name(k)))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        let find_by = keys
            .iter()
            .map(|k| public_name(k))
            .collect::<Vec<_>>()
            .join("And");
        body.push_str(&format!(
            "if (database.{name}Table.Count != rows{name}.Count) throw new Exception(\"Reload count differs\");\nforeach (var expected in rows{name}) {{ var actual = database.{name}Table.FindBy{find_by}({tuple}); if (!MessagePackSerializer.Serialize(expected).SequenceEqual(MessagePackSerializer.Serialize(actual))) throw new Exception(\"Reload values differ\"); }}\n"
        ));
    }
    body.push_str("File.WriteAllBytes(args[1], bytes);\nConsole.WriteLine(\"PASS MasterMemory build / reload / every selected row\");\n");

    let program = format!(
        "using System;\nusing System.IO;\nusing System.Linq;\nusing System.Collections.Generic;\nusing System.Text.Json;\nusing MasterMemory;\nusing MessagePack;\nusing MessagePack.Resolvers;\nusing Masterdata.Generated;\n[assembly: MasterMemoryGeneratorOptions(Namespace = \"Masterdata.Generated\")]\nMessagePackSerializer.DefaultOptions = MessagePackSerializerOptions.Standard.WithResolver(CompositeResolver.Create(MasterMemoryResolver.Instance, StandardResolver.Instance));\nusing var document = JsonDocument.Parse(File.ReadAllText(args[0]));\nvar builder = new DatabaseBuilder();\n{body}"
    );

    let mut dataset = serde_json::Map::new();
    for t in &all_tables {
        let mut rows: Vec<Value> = Vec::new();
        for s in &t.sources {
            if let Some(arr) = s.data.get("records").and_then(|v| v.as_array()) {
                for r in arr {
                    rows.push(r.to_dotnet_json_value());
                }
            }
        }
        dataset.insert(t.name.clone(), Value::Array(rows));
    }

    GeneratedBuild {
        files,
        program,
        input: Value::Object(dataset).to_string(),
    }
}

fn run_dotnet(cwd: &Path, args: &[&str]) -> Result<String, String> {
    let executable = dotnet_executable()?;
    let mut cmd = Command::new(executable);
    cmd.current_dir(cwd)
        .args(args)
        .env("DOTNET_CLI_TELEMETRY_OPTOUT", "1")
        .env("MSBUILDDISABLENODEREUSE", "1")
        .env("DOTNET_CLI_USE_MSBUILD_SERVER", "0")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x08000000);
    }
    let output = cmd
        .output()
        .map_err(|e| format!(".NET SDKを確認してください: {e}"))?;
    let mut combined = String::from_utf8_lossy(&output.stdout).to_string();
    combined.push_str(&String::from_utf8_lossy(&output.stderr));
    if output.status.success() {
        Ok(combined)
    } else {
        let tail: String = combined
            .chars()
            .rev()
            .take(16000)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        Err(tail)
    }
}

pub fn build_workspace(
    workspace: &Workspace,
    input: &Snapshot,
    profile: &str,
    revision: i64,
) -> Result<BuildResult, String> {
    if !profile.is_empty() && !input.config.profiles.contains_key(profile) {
        return Err("選択したプロファイルが見つかりません。".to_string());
    }
    let snapshot = if !profile.is_empty() {
        in_profile(input, profile)
    } else {
        input.clone()
    };
    let problems = validate(&snapshot, "");
    if !problems.is_empty() {
        let preview = problems
            .iter()
            .take(12)
            .map(|p| p.message.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        return Err(format!(
            "Buildできません（{}件）\n{}",
            problems.len(),
            preview
        ));
    }

    let generated = generate(&snapshot);
    let id = uid();
    let directory = safe_path(&workspace.root, &format!(".masterdata/builds/{id}"))?;
    fs::create_dir_all(directory.join("Generated")).map_err(|e| e.to_string())?;
    for (file, text) in &generated.files {
        fs::write(directory.join("Generated").join(file), text).map_err(|e| e.to_string())?;
    }
    fs::write(directory.join("Program.cs"), &generated.program).map_err(|e| e.to_string())?;
    fs::write(directory.join("input.json"), &generated.input).map_err(|e| e.to_string())?;
    fs::write(
        directory.join("Builder.csproj"),
        r#"<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup><OutputType>Exe</OutputType><TargetFramework>net8.0</TargetFramework><Nullable>enable</Nullable><LangVersion>12</LangVersion><RootNamespace>Masterdata.Generated</RootNamespace></PropertyGroup><ItemGroup><PackageReference Include="MasterMemory" Version="3.0.4"/><PackageReference Include="MessagePack" Version="3.1.11"/></ItemGroup></Project>"#,
    )
    .map_err(|e| e.to_string())?;

    let mut log = run_dotnet(
        &directory,
        &[
            "build",
            "Builder.csproj",
            "-c",
            "Release",
            "-o",
            "bin",
            "--nologo",
            "--disable-build-servers",
        ],
    )?;
    log.push_str(&run_dotnet(
        &directory,
        &["bin/Builder.dll", "input.json", "masterdata.bytes"],
    )?);
    if !log.contains("PASS MasterMemory build / reload / every selected row") {
        return Err("成果物のreload確認を取得できませんでした。".to_string());
    }

    let all_tables = tables(&snapshot);
    let total_records: usize = all_tables
        .iter()
        .map(|t| {
            t.sources
                .iter()
                .map(|s| {
                    s.data
                        .get("records")
                        .and_then(|v| v.as_array())
                        .map(|a| a.len())
                        .unwrap_or(0)
                })
                .sum::<usize>()
        })
        .sum();

    let mut files: Vec<String> = generated.files.keys().cloned().collect();
    files.push("masterdata.bytes".to_string());

    let mut hashes = IndexMap::new();
    for file in &files {
        let p = if file.ends_with(".cs") {
            directory.join("Generated").join(file)
        } else {
            directory.join(file)
        };
        let bytes = fs::read(&p).map_err(|e| e.to_string())?;
        hashes.insert(file.clone(), digest(&BASE64.encode(&bytes)));
    }

    let result = BuildResult {
        directory: directory.to_string_lossy().to_string(),
        revision,
        profile: profile.to_string(),
        tables: all_tables.len(),
        records: total_records,
        log,
        files,
        hashes,
    };

    fs::write(
        directory.join("build.json"),
        serde_json::to_string_pretty(&result).unwrap_or_default(),
    )
    .map_err(|e| e.to_string())?;

    Ok(result)
}

#[derive(Debug, Clone)]
struct PublishTask {
    target: PathBuf,
    bytes: Option<Vec<u8>>,
    before: Option<Vec<u8>>,
}

fn check_no_symlinks_on_absolute(abs: &Path) -> Result<(), String> {
    let mut cur = PathBuf::new();
    for comp in abs.components() {
        cur.push(comp);
        match fs::symlink_metadata(&cur) {
            Ok(meta) => {
                if meta.file_type().is_symlink() {
                    return Err("シンボリックリンクは変更しません。".to_string());
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(())
}

fn normalize_lexical(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in path.components() {
        match comp {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}

pub fn publish_build(
    result: &BuildResult,
    root: &Path,
    targets: &PublishConfig,
) -> Result<String, String> {
    let root = fs::canonicalize(root).map_err(|e| e.to_string())?;
    let mut tasks: Vec<PublishTask> = Vec::new();
    let mut manifests: Vec<(PathBuf, String)> = Vec::new();
    let build_dir = PathBuf::from(&result.directory);

    for (kind, target) in [("csharp", &targets.csharp), ("binary", &targets.binary)] {
        if target.is_empty() {
            continue;
        }
        let destination = normalize_lexical(&root.join(target));
        let sources_dir = root.join("sources");
        if destination == root.join("masterdata.yaml")
            || destination == root.join("masterdata.toml")
            || destination == sources_dir
            || destination.starts_with(&sources_dir)
        {
            return Err("編集元を出力先に指定できません。".to_string());
        }
        check_no_symlinks_on_absolute(&destination)?;

        let directory = if kind == "csharp" {
            destination.clone()
        } else {
            destination
                .parent()
                .ok_or_else(|| "出力先パスが不正です。".to_string())?
                .to_path_buf()
        };
        let manifest_file = directory.join(format!(".masterdata-{kind}-manifest.json"));
        let previous: IndexMap<String, String> = match optional_read(&manifest_file)? {
            Some(text) => serde_json::from_str(&text).unwrap_or_default(),
            None => IndexMap::new(),
        };
        let mut next: IndexMap<String, String> = IndexMap::new();

        let names: Vec<String> = if kind == "csharp" {
            result
                .files
                .iter()
                .filter(|f| f.ends_with(".cs"))
                .cloned()
                .collect()
        } else {
            vec!["masterdata.bytes".to_string()]
        };

        for name in &names {
            let file = if kind == "csharp" {
                destination.join(name)
            } else {
                destination.clone()
            };
            let artifact_path = if kind == "csharp" {
                build_dir.join("Generated").join(name)
            } else {
                build_dir.join(name)
            };
            let bytes = fs::read(&artifact_path).map_err(|e| e.to_string())?;
            check_no_symlinks_on_absolute(&file)?;
            let actual_hash = digest(&BASE64.encode(&bytes));
            if result.hashes.get(name) != Some(&actual_hash) {
                return Err(format!("Build後に成果物が変更されています: {name}"));
            }
            let before = optional_read_bytes(&file)?;
            let key = file
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or(name)
                .to_string();
            if let Some(before_bytes) = &before {
                let before_hash = digest(&BASE64.encode(before_bytes));
                if previous.get(&key) != Some(&before_hash) {
                    return Err(format!("配布先の既存ファイルを保護しました: {}", file.display()));
                }
            }
            tasks.push(PublishTask {
                target: file,
                bytes: Some(bytes),
                before,
            });
            next.insert(key, actual_hash);
        }

        for (name, prev_hash) in &previous {
            if next.contains_key(name)
                || Path::new(name).file_name().and_then(|n| n.to_str()) != Some(name.as_str())
                || !name.ends_with(".g.cs")
            {
                continue;
            }
            let file = directory.join(name);
            let Ok(Some(before_bytes)) = optional_read_bytes(&file) else {
                continue;
            };
            if &digest(&BASE64.encode(&before_bytes)) != prev_hash {
                return Err(format!("変更された生成ファイルを保護しました: {}", file.display()));
            }
            tasks.push(PublishTask {
                target: file,
                bytes: None,
                before: Some(before_bytes),
            });
        }

        manifests.push((
            manifest_file,
            serde_json::to_string_pretty(&next).unwrap_or_default(),
        ));
    }

    if tasks.is_empty() {
        return Err("出力先を設定してください。".to_string());
    }

    for (m_file, m_text) in manifests {
        let before = optional_read_bytes(&m_file)?;
        tasks.push(PublishTask {
            target: m_file,
            bytes: Some(m_text.into_bytes()),
            before,
        });
    }

    let journal = build_dir.join(format!("publish-{}", uid()));
    fs::create_dir_all(&journal).map_err(|e| e.to_string())?;
    let journal_manifest: Vec<Value> = tasks
        .iter()
        .enumerate()
        .map(|(i, t)| {
            json!({
                "target": t.target.to_string_lossy(),
                "before": t.before.as_ref().map(|_| format!("{i}.before")),
            })
        })
        .collect();
    fs::write(
        journal.join("manifest.json"),
        serde_json::to_string_pretty(&journal_manifest).unwrap_or_default(),
    )
    .map_err(|e| e.to_string())?;
    for (i, t) in tasks.iter().enumerate() {
        if let Some(before) = &t.before {
            fs::write(journal.join(format!("{i}.before")), before).map_err(|e| e.to_string())?;
        }
    }

    let mut completed: Vec<PublishTask> = Vec::new();
    let apply_res = (|| -> Result<(), String> {
        for task in &tasks {
            let current = optional_read_bytes(&task.target)?;
            if current != task.before {
                return Err(format!("配布中の外部変更: {}", task.target.display()));
            }
            if let Some(parent) = task.target.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            match &task.bytes {
                None => {
                    fs::remove_file(&task.target).map_err(|e| e.to_string())?;
                }
                Some(bytes) => {
                    let temp = PathBuf::from(format!("{}.{}.tmp", task.target.display(), uid()));
                    fs::write(&temp, bytes).map_err(|e| e.to_string())?;
                    fs::rename(&temp, &task.target).map_err(|e| e.to_string())?;
                }
            }
            completed.push(task.clone());
        }
        Ok(())
    })();

    if let Err(err) = apply_res {
        for task in completed.into_iter().rev() {
            let actual = optional_read_bytes(&task.target).ok().flatten();
            if actual != task.bytes {
                return Err(format!(
                    "配布中に外部変更があります。元の内容を {} に保持しています。",
                    journal.display()
                ));
            }
            match &task.before {
                None => {
                    let _ = fs::remove_file(&task.target);
                }
                Some(b) => {
                    let _ = fs::write(&task.target, b);
                }
            }
        }
        return Err(err);
    }

    Ok(format!(
        "{}ファイルを配置しました。Unity側のコンパイル・読込みは未確認です。",
        tasks.len()
    ))
}
