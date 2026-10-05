#![cfg(feature = "native-consumer")]
use masterdata_engine::{
    delivery::{BuildPlan, SavedConfig},
    native::{artifact::ArtifactSet, dotnet},
};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn run(root: &Path, args: &[&str], passed: bool) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_masterdata"))
        .current_dir(root)
        .args(args)
        .output()
        .unwrap();
    assert_eq!(
        output.status.success(),
        passed,
        "{}: {}\n{}",
        args.join(" "),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}
fn artifact(root: &Path) -> ArtifactSet {
    ArtifactSet::load(&SavedConfig::load(root).unwrap()).unwrap()
}
fn fixture() -> tempfile::TempDir {
    let repository = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    let minimal = repository.join("fixtures/rewrite-oracle/v1/consumer/minimal");
    let temp = tempfile::tempdir().unwrap();
    fs::copy(
        minimal.join("masterdata.toml"),
        temp.path().join("masterdata.toml"),
    )
    .unwrap();
    fs::create_dir(temp.path().join("sources")).unwrap();
    for entry in fs::read_dir(minimal.join("sources")).unwrap() {
        let entry = entry.unwrap();
        fs::copy(
            entry.path(),
            temp.path().join("sources").join(entry.file_name()),
        )
        .unwrap();
    }
    temp
}
#[test]
fn cli_build_and_publish_preserve_their_independent_result_and_source_boundaries() {
    let temp = fixture();
    let root = temp.path();
    let config_path = root.join("masterdata.toml");
    let config = fs::read_to_string(&config_path).unwrap();
    let targets = "\n[[publish.targets]]\nkind=\"csharp\"\npath=\"delivery/generated\"\n[[publish.targets]]\nkind=\"binary\"\npath=\"delivery/binary/masterdata.bytes\"\n";
    fs::write(&config_path, config.clone() + targets).unwrap();
    let dry = run(root, &["build", "--dry-run", "--json"], true);
    assert_eq!(dry["outcome"], "Success");
    assert_eq!(dry["dryRun"], true);
    assert!(!root.join(".masterdata").exists());
    assert!(!root.join("delivery").exists());
    let built = run(root, &["build", "--json"], true);
    assert_eq!(built["outcome"], "Success");
    assert!(!root.join("delivery").exists());
    let set = artifact(root);
    let original_receipt = set.receipt.clone();
    let plan = BuildPlan::capture(root, None).unwrap();
    let consumer = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../fixtures/rewrite-oracle/v1/consumer/minimal/Consumer.cs");
    dotnet::verify_consumer(plan.csharp(), &set.binary, &consumer, "").unwrap();
    let preview = run(root, &["publish", "--dry-run"], true);
    assert_eq!(preview["outcome"], "NotAttempted");
    assert_eq!(preview["preview"]["sourceFreshness"], "not_checked");
    assert_eq!(preview["preview"]["targets"].as_array().unwrap().len(), 2);
    assert!(!root.join("delivery").exists());
    let data = root.join("sources/data.yaml");
    let saved_data = fs::read(&data).unwrap();
    fs::write(&data, b"kind: [unfinished").unwrap();
    let published = run(root, &["publish", "--json"], true);
    assert_eq!(published["outcome"], "Success");
    assert_eq!(published["unityVerification"], "not_observed");
    assert_eq!(
        fs::read(root.join("delivery/binary/masterdata.bytes")).unwrap(),
        set.binary
    );
    for (name, bytes) in &set.csharp {
        assert_eq!(
            fs::read(root.join("delivery/generated").join(name)).unwrap(),
            *bytes
        );
    }
    assert_eq!(fs::read(&data).unwrap(), b"kind: [unfinished");
    let failure = run(root, &["build", "--publish"], false);
    assert_eq!(failure["diagnostic"]["code"], "E-BUILD-INVALID");
    assert_eq!(artifact(root).receipt, original_receipt);
    assert_eq!(
        fs::read(root.join("delivery/binary/masterdata.bytes")).unwrap(),
        set.binary
    );
    fs::write(&data, saved_data).unwrap();
    let combined = run(root, &["build", "--publish"], true);
    assert_eq!(combined["build"]["outcome"], "Success");
    assert_eq!(combined["publish"]["outcome"], "Success");
    let external_bytes = fs::read(root.join("delivery/binary/masterdata.bytes")).unwrap();
    fs::write(
        &config_path,
        config + "\n[[publish.targets]]\nkind=\"csharp\"\npath=\"sources\"\n",
    )
    .unwrap();
    let combined = run(root, &["build", "--publish"], false);
    assert_eq!(combined["outcome"], "Failure");
    assert_eq!(combined["build"]["outcome"], "Success");
    assert_eq!(combined["publish"]["outcome"], "NotAttempted");
    let retained = artifact(root);
    assert_eq!(retained.binary, external_bytes);
    assert_eq!(
        fs::read(root.join("delivery/binary/masterdata.bytes")).unwrap(),
        external_bytes
    );
    let expected: BTreeMap<_, _> = plan
        .csharp()
        .iter()
        .map(|(name, bytes)| (name.clone(), bytes.as_bytes().to_vec()))
        .collect();
    assert_eq!(retained.csharp, expected);
    run(root, &["build", "--profile", "missing"], false);
    assert_eq!(artifact(root).receipt, retained.receipt);
    println!(
        "PASS CLI saved Build / actual consumer / dry run / independent Publish / combined failure retains Build"
    );
}
