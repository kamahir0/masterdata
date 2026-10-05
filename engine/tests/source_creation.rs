use masterdata_engine::{
    creation::{self, Artifact, Request},
    instrument,
    native::{self, Fault, Outcome},
    project::Project,
    semantic::{self, Type},
    workspace::Workspace,
};
use std::{
    fs,
    path::{Path, PathBuf},
};
fn copy(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).unwrap();
    for e in fs::read_dir(src).unwrap() {
        let e = e.unwrap();
        if e.file_type().unwrap().is_dir() {
            copy(&e.path(), &dst.join(e.file_name()));
        } else {
            fs::copy(e.path(), dst.join(e.file_name())).unwrap();
        }
    }
}
fn fixture() -> tempfile::TempDir {
    let temp = tempfile::tempdir().unwrap();
    copy(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../fixtures/rewrite-oracle/v1/save-both/input"),
        temp.path(),
    );
    temp
}
fn request(w: &Workspace, category: &str, file: &str) -> Request {
    let path = format!("sources/{file}");
    Request {
        root: "sources".into(),
        artifact: w.creation_defaults(category, &path, Some("item")).unwrap(),
        path,
    }
}
#[test]
fn minimal_creation_proposals_avoid_members_colliding_with_their_own_type_name() {
    let temp = fixture();
    let w = Workspace::open(temp.path()).unwrap();
    for (category, filename) in [
        ("valueObject", "value.yaml"),
        ("valueObject", "equals.yaml"),
        ("enum", "none.yaml"),
        ("flags", "none.yaml"),
        ("flags", "enabled.yaml"),
        ("custom", "amount.yaml"),
    ] {
        let candidate = request(&w, category, filename);
        creation::candidate(&w.read, &candidate).unwrap();
    }
}

#[cfg(windows)]
#[test]
fn creation_rejects_windows_junction_destination_without_touching_its_target() {
    let temp = fixture();
    let mut w = Workspace::open(temp.path()).unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::write(outside.path().join("sentinel"), b"preserved").unwrap();
    let junction = temp.path().join("sources/junction");
    let result = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(&junction)
        .arg(outside.path())
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "junction setup: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    let request = request(&w, "valueObject", "junction/token.yaml");
    assert_eq!(w.create_source(&request).unwrap_err().code, "E-PATH-ALIAS");
    assert!(!outside.path().join("token.yaml").exists());
    assert_eq!(
        fs::read(outside.path().join("sentinel")).unwrap(),
        b"preserved"
    );
    fs::remove_dir(junction).unwrap();
}
#[test]
fn typed_creation_is_deterministic_exclusive_and_keeps_other_source_drafts() {
    let temp = fixture();
    let mut w = Workspace::open(temp.path()).unwrap();
    let selected = w.select("sources/data.yaml", 0, 32).unwrap();
    w.edit_text(
        "sources/data.yaml",
        0,
        &selected.rows[0].id,
        "note",
        "keep local draft",
    )
    .unwrap();
    let draft = w.current_doc("sources/data.yaml").unwrap().bytes.clone();
    let original = fs::read(temp.path().join("sources/data.yaml")).unwrap();
    // A candidate-independent invalid record must not block new declarations.
    fs::write(
        temp.path().join("sources/bad-data.yaml"),
        "kind: data\ntable: item\nrecords:\n  - id: invalid\n",
    )
    .unwrap();
    for (category, file) in [
        ("table", "new-table.yaml"),
        ("data", "new-data.yml"),
        ("valueObject", "item-token.yaml"),
        ("enum", "mode.yaml"),
        ("flags", "feature.yaml"),
        ("custom", "details.yaml"),
    ] {
        let request = request(&w, category, file);
        let first = creation::candidate(&w.validation_snapshot(), &request)
            .unwrap()
            .unwrap();
        let second = creation::candidate(&w.validation_snapshot(), &request)
            .unwrap()
            .unwrap();
        assert_eq!(first.bytes, second.bytes);
        assert!(w.creation_preview(&request)["valid"].as_bool().unwrap());
        assert_eq!(w.create_source(&request).unwrap().outcome, Outcome::Success);
        let bytes = fs::read(temp.path().join(&request.path)).unwrap();
        assert_eq!(bytes, first.bytes.as_bytes());
        assert_eq!(
            w.create_source(&request).unwrap().outcome,
            Outcome::Conflict
        );
        assert_eq!(fs::read(temp.path().join(&request.path)).unwrap(), bytes);
        assert_eq!(w.current_doc("sources/data.yaml").unwrap().bytes, draft);
        assert_eq!(w.dirty_paths(), ["sources/data.yaml"]);
        assert!(w.drafts["sources/data.yaml"].can_undo());
    }
    assert_eq!(
        fs::read(temp.path().join("sources/data.yaml")).unwrap(),
        original
    );
    let p = Project::open(temp.path()).unwrap();
    assert_eq!(p.record_sources("new-table"), ["sources/new-table.yaml"]);
    assert!(matches!(p.types["ItemToken"], Type::ValueObject { .. }));
    assert!(matches!(p.types["Details"], Type::Custom { .. }));
    let (_, measurement) = instrument::measure(|| w.select("sources/new-data.yml", 0, 32).unwrap());
    assert_eq!(measurement.work.project_discovery, 0);
    assert_eq!(measurement.work.project_enumeration, 0);
    assert_eq!(measurement.work.project_yaml_parse, 0);
    assert_eq!(measurement.work.project_validation, 0);
    assert!(!temp.path().join(".masterdata").exists());
}

#[test]
fn creation_rejects_collision_bad_declaration_missing_parent_and_path_escape_before_mutation() {
    let temp = fixture();
    let mut w = Workspace::open(temp.path()).unwrap();
    let mut invalid = request(&w, "table", "invalid.yaml");
    if let Artifact::Table { name, .. } = &mut invalid.artifact {
        *name = "item".into();
    }
    assert_eq!(
        w.create_source(&invalid).unwrap_err().code,
        "E-CREATE-IDENTITY"
    );
    invalid.artifact = Artifact::Data {
        table: "missing".into(),
    };
    assert_eq!(
        w.create_source(&invalid).unwrap_err().code,
        "E-TABLE-MISSING"
    );
    invalid.artifact = Artifact::Flags {
        name: "BadFlags".into(),
        underlying: "ulong".into(),
        members: vec![("None".into(), "0".into()), ("Combined".into(), "3".into())],
    };
    assert_eq!(
        w.create_source(&invalid).unwrap_err().code,
        "E-FLAGS-MEMBER"
    );
    assert!(!temp.path().join(&invalid.path).exists());
    invalid = request(&w, "valueObject", "safe.yaml");
    for path in [
        "../outside.yaml",
        "/outside.yaml",
        "sources/../outside.yaml",
        "sources/missing/new.yaml",
        "other/new.yaml",
        "sources/bad.txt",
        "sources/new:stream.yaml",
        "sources/alias./new.yaml",
    ] {
        invalid.path = path.into();
        assert!(w.create_source(&invalid).is_err(), "{path}");
    }
    invalid = request(&w, "folder", "group");
    assert_eq!(w.create_source(&invalid).unwrap().outcome, Outcome::Success);
    assert!(w.read.folders.contains("sources/group"));
    assert_eq!(
        w.create_source(&invalid).unwrap().outcome,
        Outcome::Conflict
    );
    assert_eq!(
        fs::read_dir(temp.path().join("sources/group"))
            .unwrap()
            .count(),
        0
    );
    let invalid = request(&w, "data", "ambiguous.yaml");
    let schema = fs::read(temp.path().join("sources/schema.yaml")).unwrap();
    fs::write(temp.path().join("sources/other-schema.yaml"), schema).unwrap();
    assert_eq!(
        w.create_source(&invalid).unwrap_err().code,
        "E-TABLE-MISSING"
    );
    assert!(!temp.path().join(&invalid.path).exists());
}

#[test]
fn exclusive_commit_preserves_a_target_created_after_preflight_and_a_replaced_parent() {
    let temp = fixture();
    let root = temp.path().canonicalize().unwrap();
    let source_root = root.join("sources");
    let path = "sources/race.yaml";
    let result = native::create_exclusive(
        &root,
        &source_root,
        path,
        Some("complete candidate\n"),
        Fault::None,
        || {
            fs::write(root.join(path), "outside winner\n").unwrap();
            Ok(())
        },
    );
    assert_eq!(result.outcome, Outcome::Conflict, "{}", result.message);
    assert_eq!(
        fs::read_to_string(root.join(path)).unwrap(),
        "outside winner\n"
    );
    fs::create_dir(root.join("sources/group")).unwrap();
    let result = native::create_exclusive(
        &root,
        &source_root,
        "sources/group/new.yaml",
        Some("complete candidate\n"),
        Fault::None,
        || {
            fs::rename(root.join("sources/group"), root.join("sources/prior-group")).unwrap();
            fs::create_dir(root.join("sources/group")).unwrap();
            Ok(())
        },
    );
    assert_eq!(result.outcome, Outcome::Conflict);
    assert!(!root.join("sources/group/new.yaml").exists());
    assert!(!root.join("sources/prior-group/new.yaml").exists());
}

#[cfg(unix)]
#[test]
fn symlink_creation_destination_never_writes_inside_or_outside_an_alias() {
    use std::os::unix::fs::symlink;
    let temp = fixture();
    let outside = tempfile::tempdir().unwrap();
    symlink(outside.path(), temp.path().join("sources/alias")).unwrap();
    let mut w = Workspace::open(temp.path()).unwrap();
    let request = request(&w, "table", "alias/new.yaml");
    assert_eq!(w.create_source(&request).unwrap_err().code, "E-PATH-ALIAS");
    assert!(!outside.path().join("new.yaml").exists());
}

#[test]
fn source_creation_keeps_ordered_keys_schema_only_and_lossless_enum_members() {
    let temp = fixture();
    let mut w = Workspace::open(temp.path()).unwrap();
    let mut request = request(&w, "table", "other.yaml");
    if let Artifact::Table {
        fields,
        primary,
        secondary,
        inline,
        ..
    } = &mut request.artifact
    {
        *inline = false;
        fields[0].key = "9".into();
        fields.push(
            semantic::Field {
                key: 2,
                name: "code".into(),
                type_name: "string".into(),
                nullable: false,
                array: false,
            }
            .into(),
        );
        primary.fields = vec!["code".into(), "id".into()];
        secondary.push(semantic::Key {
            fields: vec!["id".into()],
            non_unique: true,
        });
    }
    assert_eq!(w.create_source(&request).unwrap().outcome, Outcome::Success);
    let source = w.select(&request.path, 0, 32).unwrap();
    assert!(source.source.is_none());
    assert!(!source.can_add);
    assert_eq!(source.table.primary.fields, ["code", "id"]);
    assert_eq!(
        source
            .columns
            .iter()
            .map(|c| c.field.key)
            .collect::<Vec<_>>(),
        [9, 2]
    );
    let request = Request {
        root: "sources".into(),
        path: "sources/huge.yaml".into(),
        artifact: Artifact::Enum {
            name: "Huge".into(),
            underlying: "ulong".into(),
            members: vec![("Maximum".into(), "18446744073709551615".into())],
        },
    };
    assert_eq!(w.create_source(&request).unwrap().outcome, Outcome::Success);
    let Type::Enum { members, .. } = &w.read.types["Huge"] else {
        panic!()
    };
    assert_eq!(members[0].1, "18446744073709551615");
    assert!(
        fs::read_to_string(temp.path().join(request.path))
            .unwrap()
            .contains("value: 18446744073709551615")
    );
}

#[cfg(feature = "oracle-faults")]
#[test]
fn creation_failure_leaves_no_partial_file_and_unknown_requires_explicit_recheck() {
    let temp = fixture();
    let mut w = Workspace::open(temp.path()).unwrap();
    let request = request(&w, "valueObject", "token.yaml");
    assert_eq!(
        w.create_source_with_fault(&request, Fault::BeforeCommit)
            .unwrap()
            .outcome,
        Outcome::Failure
    );
    assert!(!temp.path().join(&request.path).exists());
    assert_eq!(
        w.create_source_with_fault(&request, Fault::AfterCommitObservation)
            .unwrap()
            .outcome,
        Outcome::OutcomeUnknown
    );
    let candidate = creation::document(&request.artifact).unwrap().unwrap();
    assert_eq!(
        fs::read(temp.path().join(&request.path)).unwrap(),
        candidate.bytes.as_bytes()
    );
    assert_eq!(w.uncertain_paths(), std::slice::from_ref(&request.path));
    assert!(w.save_all().unwrap().is_empty());
    assert_eq!(w.uncertain_paths(), std::slice::from_ref(&request.path));
    assert_eq!(
        w.create_source(&request).unwrap_err().code,
        "E-OUTCOME-UNKNOWN"
    );
    assert_eq!(
        w.recheck_creation(&request.path).unwrap().outcome,
        Outcome::Success
    );
    assert!(w.uncertain_paths().is_empty());
    assert!(w.read.types.contains_key("Token"));
    assert_eq!(
        w.create_source(&request).unwrap().outcome,
        Outcome::Conflict
    );
}
