use masterdata_engine::{
    creation::Declaration,
    migration::{self, Command},
    project::Project,
    source::Value,
};
use std::{
    fs,
    path::{Path, PathBuf},
};
fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fixtures/rewrite-oracle/v1")
}
fn fixture(input: &Path) -> tempfile::TempDir {
    let temp = tempfile::tempdir().unwrap();
    fs::create_dir(temp.path().join("sources")).unwrap();
    for entry in fs::read_dir(input).unwrap() {
        let entry = entry.unwrap();
        fs::copy(
            entry.path(),
            temp.path().join("sources").join(entry.file_name()),
        )
        .unwrap();
    }
    fs::copy(
        root().join("save-both/input/masterdata.toml"),
        temp.path().join("masterdata.toml"),
    )
    .unwrap();
    temp
}
fn add(name: &str) -> Command {
    Command::AddField {
        table: "item".into(),
        declaration: Declaration {
            key: "2".into(),
            name: name.into(),
            type_name: "int".into(),
            nullable: false,
            array: false,
        },
        initializer: Some(Value::Literal("7".into())),
        position: None,
    }
}
#[test]
fn independent_migration_plans_match_exact_bytes_and_do_not_touch_disk() {
    for id in ["migration-add", "migration-rename", "migration-drop"] {
        let dir = root().join(id);
        let temp = fixture(&dir.join("input"));
        let project = Project::open(temp.path()).unwrap();
        let command = match id {
            "migration-add" => add("rank"),
            "migration-rename" => Command::RenameField {
                table: "item".into(),
                field: "id".into(),
                new_name: "itemId".into(),
            },
            _ => Command::DropField {
                table: "item".into(),
                field: "note".into(),
            },
        };
        let plan = migration::derive(&project, command.clone()).unwrap();
        let again = migration::derive(&project, command).unwrap();
        assert_eq!(plan.affected_records, 2);
        assert_eq!(plan.destructive, id == "migration-drop");
        assert_eq!(plan.candidates.len(), 3);
        for (path, candidate) in &plan.candidates {
            let file = Path::new(path).file_name().unwrap();
            assert_eq!(
                candidate.after.bytes.as_bytes(),
                fs::read(dir.join("expected").join(file)).unwrap(),
                "{id}/{path}"
            );
            assert_eq!(
                candidate.before.bytes.as_bytes(),
                fs::read(temp.path().join(path)).unwrap(),
                "Plan wrote {path}"
            );
            assert_eq!(candidate.after.bytes, again.candidates[path].after.bytes);
        }
        assert!(!temp.path().join(".masterdata").exists());
    }
}
#[test]
fn inline_schema_records_are_one_candidate_and_unrelated_invalid_values_do_not_block() {
    let temp = fixture(&root().join("migration-add/input"));
    let schema = temp.path().join("sources/schema.yaml");
    fs::write(
        &schema,
        fs::read_to_string(&schema).unwrap()
            + "records:\n  - id: 3\n    note: 4 # existing unrelated value\n",
    )
    .unwrap();
    fs::write(temp.path().join("sources/other-schema.yaml"),"kind: schema\ntable: other\nfields:\n  - key: 0\n    name: id\n    type: int\nprimaryKey:\n  fields: [id]\nrecords:\n  - id: invalid\n").unwrap();
    let p = Project::open(temp.path()).unwrap();
    let plan = migration::derive(&p, add("rank")).unwrap();
    assert_eq!(plan.affected_records, 3);
    assert_eq!(plan.candidates.len(), 3);
    assert!(!plan.candidates.contains_key("sources/other-schema.yaml"));
    assert!(
        plan.candidates["sources/schema.yaml"]
            .after
            .bytes
            .contains("note: 4 # existing unrelated value\n    rank: 7\n")
    );
    fs::write(temp.path().join("sources/unknown.yaml"), "kind: [\n").unwrap();
    let p = Project::open(temp.path()).unwrap();
    assert_eq!(
        migration::derive(&p, add("rank")).unwrap_err().code,
        "E-MIGRATION-RESOLUTION"
    );
}
#[test]
fn add_uses_explicit_typed_constant_and_presentation_position_without_renumbering_keys() {
    let temp = fixture(&root().join("migration-add/input"));
    let p = Project::open(temp.path()).unwrap();
    let mut command = add("rank");
    if let Command::AddField { initializer, .. } = &mut command {
        *initializer = None;
    }
    assert!(migration::derive(&p, command).is_err());
    let mut command = add("rank");
    if let Command::AddField { initializer, .. } = &mut command {
        *initializer = Some(Value::Text("not integer".into()));
    }
    assert!(migration::derive(&p, command).is_err());
    let mut command = add("rank");
    if let Command::AddField { position, .. } = &mut command {
        *position = Some(0);
    }
    let plan = migration::derive(&p, command).unwrap();
    let bytes = &plan.candidates["sources/schema.yaml"].after.bytes;
    assert!(bytes.find("key: 2").unwrap() < bytes.find("key: 0").unwrap());
    for path in ["sources/one.yaml", "sources/two.yaml"] {
        let bytes = &plan.candidates[path].after.bytes;
        assert!(bytes.find("note:").unwrap() < bytes.find("rank:").unwrap());
    }
}
#[test]
fn rename_updates_resolved_reference_components_only_and_drop_never_removes_relationships() {
    let temp = fixture(&root().join("migration-rename/input"));
    fs::write(temp.path().join("sources/link.yaml"),"kind: schema\ntable: link\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: item\n    type: int\nprimaryKey:\n  fields: [id]\nreferences:\n  - name: rewardItem\n    fields: ['item'] # source binding\n    target:\n      table: item\n      fields: [\"id\"] # target binding\n    csharpName: GetRewardItemMaster\nrecords:\n  - id: 1\n    item: 1\n").unwrap();
    let p = Project::open(temp.path()).unwrap();
    let command = Command::RenameField {
        table: "item".into(),
        field: "id".into(),
        new_name: "itemId".into(),
    };
    let plan = migration::derive(&p, command).unwrap();
    let before = fs::read_to_string(temp.path().join("sources/link.yaml")).unwrap();
    assert_eq!(
        plan.candidates["sources/link.yaml"].after.bytes.as_ref(),
        before.replace(
            "fields: [\"id\"] # target binding",
            "fields: [\"itemId\"] # target binding"
        )
    );
    assert!(
        migration::derive(
            &p,
            Command::DropField {
                table: "item".into(),
                field: "id".into()
            }
        )
        .is_err()
    );
    let command = Command::SetFieldDeclaration {
        table: "item".into(),
        field: "id".into(),
        type_name: "long".into(),
        nullable: false,
        array: false,
    };
    assert!(migration::derive(&p, command).is_err());
}
#[test]
fn strict_declaration_changes_preserve_values_and_use_shared_interpreter() {
    let temp = fixture(&root().join("migration-add/input"));
    let p = Project::open(temp.path()).unwrap();
    let command = Command::SetFieldDeclaration {
        table: "item".into(),
        field: "note".into(),
        type_name: "int".into(),
        nullable: false,
        array: false,
    };
    assert!(migration::derive(&p, command).is_err());
    let command = Command::SetFieldDeclaration {
        table: "item".into(),
        field: "id".into(),
        type_name: "long".into(),
        nullable: false,
        array: false,
    };
    let plan = migration::derive(&p, command).unwrap();
    assert_eq!(plan.candidates.len(), 1);
    assert!(
        plan.candidates["sources/schema.yaml"]
            .after
            .bytes
            .contains("type: long")
    );
    fs::write(
        temp.path().join("sources/duplicate.yaml"),
        fs::read(temp.path().join("sources/schema.yaml")).unwrap(),
    )
    .unwrap();
    let p = Project::open(temp.path()).unwrap();
    assert!(migration::derive(&p, add("rank")).is_err());
}
