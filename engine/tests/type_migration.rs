use masterdata_engine::{
    creation::Declaration,
    migration::{self, Plan},
    native::{self, Outcome, SourceSetPlan},
    project::Project,
    semantic::Type,
    source::Value,
    type_migration::{self, Command},
};
use std::{collections::BTreeMap, fs, path::PathBuf};

fn fixture() -> tempfile::TempDir {
    let temp = tempfile::tempdir().unwrap();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fixtures/full");
    fs::create_dir(temp.path().join("sources")).unwrap();
    fs::copy(
        root.join("masterdata.toml"),
        temp.path().join("masterdata.toml"),
    )
    .unwrap();
    for entry in fs::read_dir(root.join("sources")).unwrap() {
        let entry = entry.unwrap();
        fs::copy(
            entry.path(),
            temp.path().join("sources").join(entry.file_name()),
        )
        .unwrap();
    }
    temp
}
fn write(temp: &tempfile::TempDir, name: &str, bytes: &str) {
    fs::write(temp.path().join("sources").join(name), bytes).unwrap();
}
fn disk(temp: &tempfile::TempDir) -> BTreeMap<String, Vec<u8>> {
    fs::read_dir(temp.path().join("sources"))
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            (
                format!("sources/{}", entry.file_name().to_str().unwrap()),
                fs::read(entry.path()).unwrap(),
            )
        })
        .collect()
}
fn plan(temp: &tempfile::TempDir, command: Command) -> Plan {
    let before = disk(temp);
    let plan = type_migration::derive(&Project::open(temp.path()).unwrap(), command).unwrap();
    assert_eq!(disk(temp), before, "Plan changed source bytes");
    assert!(
        !temp.path().join(".masterdata").exists(),
        "Plan persisted recovery/artifacts"
    );
    plan
}
fn native_plan(temp: &tempfile::TempDir, command: Command) -> SourceSetPlan {
    let project = Project::open(temp.path()).unwrap();
    SourceSetPlan::prepare(
        &project,
        migration::derive(&project, migration::Command::Type { command }).unwrap(),
    )
    .unwrap()
}
fn wrapper(temp: &tempfile::TempDir) {
    write(
        temp,
        "wrapper.yaml",
        "kind: type\nname: Wrapper\ncustom:\n  fields:\n    - key: 0\n      name: rarity\n      type: Rarity\n    - key: 1\n      name: tags\n      type: ItemTags\n    - key: 2\n      name: reward\n      type: Reward\n    - key: 3\n      name: note\n      type: string\n",
    );
    write(
        temp,
        "nested.yaml",
        "kind: schema\ntable: nested\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: values\n    type: Wrapper\n    array: true\nprimaryKey:\n  fields: [id]\nrecords:\n  - id: 1\n    values:\n      - rarity: 'Rare' # target\n        tags: [Fire, Highest]\n        reward: {values: [1], itemId: 2001, 'note': null, amount: 1}\n        note: Rare # unrelated text\n      - rarity: 'Rare' # equal occurrence\n        tags: [Fire, Highest]\n        reward: {values: [1], itemId: 2001, 'note': null, amount: 1}\n        note: Rare # unrelated text\n",
    );
}
fn rename_enum(name: &str, member: &str, new_name: &str) -> Command {
    Command::RenameEnumMember {
        type_name: name.into(),
        member: member.into(),
        new_name: new_name.into(),
    }
}
fn custom_add(initializer: Option<Value>) -> Command {
    Command::AddCustomField {
        type_name: "Reward".into(),
        declaration: Declaration {
            key: "20".into(),
            name: "debt".into(),
            type_name: "long".into(),
            nullable: false,
            array: false,
        },
        initializer,
    }
}

#[test]
fn conversion_settings_preserve_underlying_data_crlf_and_existing_flow_style() {
    let temp = fixture();
    let old = "# type\r\nkind: type\r\nname: ItemId\r\nvalueObject:\r\n  underlying: int # meaning\r\n  conversions: {fromUnderlyingImplicit: false, toUnderlyingImplicit: false}\r\n";
    write(&temp, "item-id.yaml", old);
    let command = Command::SetValueObjectConversions {
        type_name: "ItemId".into(),
        from_implicit: true,
        to_implicit: false,
    };
    let p = plan(&temp, command.clone());
    assert_eq!(p.candidates.len(), 1);
    assert_eq!(p.affected_records, 0);
    assert_eq!(
        p.candidates["sources/item-id.yaml"].after.bytes.as_ref(),
        old.replace(
            "fromUnderlyingImplicit: false",
            "fromUnderlyingImplicit: true"
        )
    );
    write(
        &temp,
        "item-id.yaml",
        "kind: type\nname: ItemId\nvalueObject:\n  underlying: int\n",
    );
    let p = plan(&temp, command.clone());
    assert!(p.candidates["sources/item-id.yaml"].after.bytes.contains(
        "  conversions:\n    fromUnderlyingImplicit: true\n    toUnderlyingImplicit: false\n"
    ));
    write(
        &temp,
        "item-id.yaml",
        "kind: type\nname: ItemId\nvalueObject: {underlying: int}\n",
    );
    assert_eq!(
        type_migration::derive(&Project::open(temp.path()).unwrap(), command)
            .unwrap_err()
            .code,
        "E-SOURCE-UNSAFE",
        "newly materialized mapping must not become flow syntax"
    );
}
#[test]
fn enum_and_flags_rename_resolve_nested_array_occurrences_without_text_replacement() {
    for (name, member, new_name, count) in [
        ("Rarity", "Rare", "Epic", 4),
        ("ItemTags", "Fire", "Flame", 4),
    ] {
        let temp = fixture();
        wrapper(&temp);
        let before = disk(&temp);
        let command = rename_enum(name, member, new_name);
        let p = plan(&temp, command.clone());
        assert_eq!(p.affected_records, count);
        assert_eq!(p.candidates.len(), 3);
        let again = plan(&temp, command);
        for (path, candidate) in &p.candidates {
            assert_eq!(candidate.after.bytes, again.candidates[path].after.bytes);
        }
        let nested = p.candidates["sources/nested.yaml"].after.bytes.as_ref();
        let original = String::from_utf8(before["sources/nested.yaml"].clone()).unwrap();
        let expected = if name == "Rarity" {
            original.replace("rarity: 'Rare'", "rarity: 'Epic'")
        } else {
            original.replace("[Fire, Highest]", "[Flame, Highest]")
        };
        assert_eq!(nested, expected);
        assert!(nested.contains("note: Rare # unrelated text"));
        let actual = Project::open(temp.path()).unwrap();
        let Type::Enum { members, .. } = &actual.types[name] else {
            panic!()
        };
        let declared = p.candidates[&format!(
            "sources/{}.yaml",
            if name == "Rarity" {
                "rarity"
            } else {
                "item-tags"
            }
        )]
            .after
            .clone();
        let (
            _,
            Type::Enum {
                members: changed, ..
            },
        ) = masterdata_engine::semantic::parse_type(&declared).unwrap()
        else {
            panic!()
        };
        assert_eq!(
            members.iter().map(|(_, value)| value).collect::<Vec<_>>(),
            changed.iter().map(|(_, value)| value).collect::<Vec<_>>()
        );
    }
}
#[test]
fn enum_add_requires_explicit_lossless_number_and_shared_atomic_bit_validation() {
    let temp = fixture();
    write(
        &temp,
        "huge.yaml",
        "kind: type\nname: Huge\nenum:\n  underlying: ulong\n  members:\n    - name: Zero\n      value: 0\n",
    );
    let p = plan(
        &temp,
        Command::AddEnumMember {
            type_name: "Huge".into(),
            name: "Maximum".into(),
            value: "18446744073709551615".into(),
        },
    );
    assert_eq!(p.candidates.len(), 1);
    assert_eq!(p.affected_records, 0);
    assert!(
        p.candidates["sources/huge.yaml"]
            .after
            .bytes
            .ends_with("    - name: Maximum\n      value: 18446744073709551615\n")
    );
    for (name, value) in [
        ("Composite", "3"),
        ("Missing", ""),
        ("Fire", "4"),
        ("TooLarge", "18446744073709551616"),
        ("ItemTags", "4"),
    ] {
        assert!(
            type_migration::derive(
                &Project::open(temp.path()).unwrap(),
                Command::AddEnumMember {
                    type_name: "ItemTags".into(),
                    name: name.into(),
                    value: value.into()
                }
            )
            .is_err()
        );
    }
    let p = plan(
        &temp,
        Command::AddEnumMember {
            type_name: "ItemTags".into(),
            name: "Snow".into(),
            value: "4".into(),
        },
    );
    assert!(
        p.candidates["sources/item-tags.yaml"]
            .after
            .bytes
            .ends_with("    - name: Snow\n      value: 4\n")
            || p.candidates["sources/item-tags.yaml"]
                .after
                .bytes
                .ends_with("    - name: Snow\r\n      value: 4\r\n")
    );
}
#[test]
fn enum_drop_rejects_used_occurrences_and_none_then_requires_native_authorization() {
    let temp = fixture();
    let project = Project::open(temp.path()).unwrap();
    assert!(
        type_migration::derive(
            &project,
            Command::DropEnumMember {
                type_name: "Rarity".into(),
                member: "Rare".into()
            }
        )
        .is_err()
    );
    for command in [
        rename_enum("ItemTags", "None", "Empty"),
        Command::DropEnumMember {
            type_name: "ItemTags".into(),
            member: "None".into(),
        },
    ] {
        assert!(type_migration::derive(&project, command).is_err());
    }
    let command = Command::DropEnumMember {
        type_name: "Rarity".into(),
        member: "Minimum".into(),
    };
    let p = native_plan(&temp, command);
    let before = disk(&temp);
    assert_eq!(p.plan.candidates.len(), 1);
    assert!(p.plan.destructive);
    assert_eq!(
        p.commit(false, native::SetFault::None).unwrap_err().code,
        "E-MIGRATION-AUTHORIZATION"
    );
    assert_eq!(disk(&temp), before);
    assert_eq!(
        p.commit(true, native::SetFault::None).unwrap().outcome,
        Outcome::Success
    );
    for (path, bytes) in before {
        if path != "sources/rarity.yaml" {
            assert_eq!(fs::read(temp.path().join(path)).unwrap(), bytes);
        }
    }
}
#[test]
fn custom_add_uses_one_explicit_constant_for_every_mapping_and_preserves_non_targets() {
    let temp = fixture();
    wrapper(&temp);
    assert!(
        type_migration::derive(&Project::open(temp.path()).unwrap(), custom_add(None)).is_err()
    );
    assert!(
        type_migration::derive(
            &Project::open(temp.path()).unwrap(),
            custom_add(Some(Value::Null))
        )
        .is_err()
    );
    let p = plan(
        &temp,
        custom_add(Some(Value::Literal("-9223372036854775808".into()))),
    );
    assert_eq!(p.affected_records, 4);
    assert_eq!(p.candidates.len(), 3);
    let before = fs::read_to_string(temp.path().join("sources/nested.yaml")).unwrap();
    assert_eq!(
        p.candidates["sources/nested.yaml"].after.bytes.as_ref(),
        before.replace(
            "'note': null, amount: 1}",
            "'note': null, amount: 1, debt: -9223372036854775808}"
        )
    );
    let before = fs::read_to_string(temp.path().join("sources/catalog-data.yaml")).unwrap();
    let nl = if before.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    assert_eq!(
        p.candidates["sources/catalog-data.yaml"]
            .after
            .bytes
            .as_ref(),
        before
            .replace(
                &format!("      amount: 4294967295{nl}"),
                &format!("      amount: 4294967295{nl}      debt: -9223372036854775808{nl}")
            )
            .replace(
                &format!("      amount: 1{nl}"),
                &format!("      amount: 1{nl}      debt: -9223372036854775808{nl}")
            )
    );
}
#[test]
fn custom_rename_and_drop_preserve_keys_order_and_quoted_member_presentation() {
    for drop in [false, true] {
        let temp = fixture();
        wrapper(&temp);
        let command = if drop {
            Command::DropCustomField {
                type_name: "Reward".into(),
                field: "note".into(),
            }
        } else {
            Command::RenameCustomField {
                type_name: "Reward".into(),
                field: "note".into(),
                new_name: "memo".into(),
            }
        };
        let p = plan(&temp, command);
        assert_eq!(p.affected_records, 4);
        assert_eq!(p.destructive, drop);
        let before = fs::read_to_string(temp.path().join("sources/nested.yaml")).unwrap();
        let expected = if drop {
            before.replace("'note': null,", "")
        } else {
            before.replace("'note': null", "'memo': null")
        };
        assert_eq!(
            p.candidates["sources/nested.yaml"].after.bytes.as_ref(),
            expected
        );
        let (_, Type::Custom { fields }) =
            masterdata_engine::semantic::parse_type(&p.candidates["sources/reward.yaml"].after)
                .unwrap()
        else {
            panic!()
        };
        assert_eq!(
            fields.iter().map(|f| f.key).collect::<Vec<_>>(),
            if drop {
                vec![9, 0, 2]
            } else {
                vec![9, 0, 5, 2]
            }
        );
        if !drop {
            assert_eq!(fields[2].name, "memo");
            assert!(fields[2].nullable);
            assert_eq!(fields[2].type_name, "string");
        }
    }
}
#[test]
fn safely_classified_unrelated_errors_survive_but_unclassifiable_dependency_blocks() {
    let temp = fixture();
    write(
        &temp,
        "other-type.yaml",
        "kind: type\nname: Other\nenum:\n  underlying: int\n  members:\n    - name: Invalid\n      value: quoted\n",
    );
    write(
        &temp,
        "other.yaml",
        "kind: schema\ntable: other\nfields:\n  - key: 0\n    name: id\n    type: int\nprimaryKey:\n  fields: [id]\nrecords:\n  - id: invalid\n",
    );
    let data = temp.path().join("sources/catalog-data.yaml");
    let bytes = fs::read_to_string(&data)
        .unwrap()
        .replace("numbers: [1, -2]", "numbers: [1, invalid]")
        .replace("rarities: [Common]", "rarities: [Unknown]");
    fs::write(&data, &bytes).unwrap();
    let p = plan(&temp, rename_enum("Rarity", "Rare", "Epic"));
    assert!(
        p.candidates["sources/catalog-data.yaml"]
            .after
            .bytes
            .contains("numbers: [1, invalid]")
    );
    assert!(
        p.candidates["sources/catalog-data.yaml"]
            .after
            .bytes
            .contains("rarities: [Unknown]")
    );
    write(&temp, "unknown.yaml", "kind: invalid\n");
    assert!(
        type_migration::derive(
            &Project::open(temp.path()).unwrap(),
            rename_enum("Rarity", "Rare", "Epic")
        )
        .is_err()
    );
    fs::remove_file(temp.path().join("sources/unknown.yaml")).unwrap();
    fs::write(data, bytes.replace("rarities: [Unknown]", "rarities: Rare")).unwrap();
    assert!(
        type_migration::derive(
            &Project::open(temp.path()).unwrap(),
            rename_enum("Rarity", "Rare", "Epic")
        )
        .is_err()
    );
}
#[test]
fn required_new_type_identity_and_affected_reference_closure_are_not_hidden_by_old_errors() {
    let temp = fixture();
    let duplicate = fs::read(temp.path().join("sources/item-id.yaml")).unwrap();
    fs::write(
        temp.path().join("sources/item-id-duplicate.yaml"),
        duplicate,
    )
    .unwrap();
    let mut command = custom_add(Some(Value::Literal("1".into())));
    if let Command::AddCustomField { declaration, .. } = &mut command {
        declaration.type_name = "ItemId".into();
    }
    assert!(type_migration::derive(&Project::open(temp.path()).unwrap(), command).is_err());
    let mut workspace = masterdata_engine::workspace::Workspace::open(temp.path()).unwrap();
    assert!(
        workspace
            .select_view("sources/item-id.yaml", 0, 32)
            .is_err()
    );
    let table = workspace
        .select("sources/catalog-data.yaml", 0, 32)
        .unwrap();
    let column = table
        .columns
        .iter()
        .position(|column| column.field.name == "itemId")
        .unwrap();
    assert!(
        !table.rows[0].cells[column].editable,
        "ambiguous type became editable through an arbitrary definition"
    );
    fs::remove_file(temp.path().join("sources/item-id-duplicate.yaml")).unwrap();
    write(
        &temp,
        "link.yaml",
        "kind: schema\ntable: link\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: item\n    type: ItemId\nprimaryKey:\n  fields: [id]\nreferences:\n  - name: targetItem\n    fields: [item]\n    target:\n      table: item\n      fields: [itemId]\n    csharpName: GetTargetItem\nrecords: []\n",
    );
    let command = Command::SetValueObjectConversions {
        type_name: "ItemId".into(),
        from_implicit: true,
        to_implicit: false,
    };
    assert!(
        type_migration::derive(&Project::open(temp.path()).unwrap(), command).is_err(),
        "nominal Reference key must resolve after the type operation"
    );
}
#[test]
fn type_selection_keeps_the_workspace_and_warm_work_counts_zero_with_local_freshness() {
    use masterdata_engine::workspace::{SelectionProjection, Workspace};
    let temp = fixture();
    write(
        &temp,
        "unrelated.yaml",
        "kind: schema\ntable: independent\nfields:\n  - key: 0\n    name: id\n    type: int\nprimaryKey:\n  fields: [id]\nrecords:\n  - id: 1\n",
    );
    let mut w = Workspace::open(temp.path()).unwrap();
    let p = w.select("sources/unrelated.yaml", 0, 32).unwrap();
    let row = p.rows[0].id.clone();
    w.edit(
        "sources/unrelated.yaml",
        p.revision,
        &row,
        &["id".into()],
        &Value::Literal("2".into()),
    )
    .unwrap();
    w.select("sources/catalog-data.yaml", 0, 32).unwrap();
    for source in [
        "sources/item-id.yaml",
        "sources/reward.yaml",
        "sources/rarity.yaml",
        "sources/item-tags.yaml",
    ] {
        let SelectionProjection::Type(view) = w.select_view(source, 0, 32).unwrap() else {
            panic!("type selection returned a Table")
        };
        assert_eq!(view.kind, "type");
        assert_eq!(view.clicked, source);
        assert_eq!(view.measurement.work.project_discovery, 0);
        assert_eq!(view.measurement.work.project_enumeration, 0);
        assert_eq!(view.measurement.work.project_yaml_parse, 0);
        assert_eq!(view.measurement.work.project_validation, 0);
        assert_eq!(view.measurement.work.local_parse, 0);
    }
    fs::remove_file(temp.path().join("sources/item-id.yaml")).unwrap();
    assert!(
        w.select_view("sources/reward.yaml", 0, 32).is_err(),
        "missing dependency left an editable Type snapshot"
    );
    assert!(
        w.select("sources/catalog-data.yaml", 0, 32).is_err(),
        "missing dependency left an editable dependent Table"
    );
    let p = w.select("sources/unrelated.yaml", 0, 32).unwrap();
    assert!(p.dirty && p.can_undo);
    assert_eq!(p.rows[0].id, row);
    assert_eq!(p.rows[0].cells[0].display, "2");
}
#[test]
fn removing_a_duplicate_type_restores_the_remaining_current_declaration_locally() {
    use masterdata_engine::workspace::{SelectionProjection, Workspace};
    let temp = fixture();
    let bytes = fs::read(temp.path().join("sources/rarity.yaml")).unwrap();
    fs::write(temp.path().join("sources/zz-duplicate.yaml"), bytes).unwrap();
    let mut w = Workspace::open(temp.path()).unwrap();
    assert!(w.select_view("sources/rarity.yaml", 0, 32).is_err());
    fs::remove_file(temp.path().join("sources/zz-duplicate.yaml")).unwrap();
    assert!(w.refresh_source("sources/zz-duplicate.yaml").is_err());
    let SelectionProjection::Type(view) = w.select_view("sources/rarity.yaml", 0, 32).unwrap()
    else {
        panic!("remaining declaration did not become current")
    };
    assert_eq!(view.name, "Rarity");
    assert_eq!(view.measurement.work.project_discovery, 0);
    assert_eq!(view.measurement.work.project_enumeration, 0);
    assert_eq!(view.measurement.work.project_yaml_parse, 0);
    assert_eq!(view.measurement.work.project_validation, 0);
    assert!(w.select("sources/catalog-data.yaml", 0, 32).is_ok());
}

#[test]
fn typed_initializer_uses_shared_scalar_interpretation_and_distinguishes_unset_from_null() {
    use masterdata_engine::{
        initializer::{self, Input},
        semantic::Field,
    };
    let project = Project::open(fixture().path()).unwrap();
    let field = Field {
        key: 0,
        name: "x".into(),
        type_name: "long".into(),
        nullable: false,
        array: false,
    };
    assert_eq!(
        initializer::resolve(
            &field,
            &Input::Scalar {
                text: "-9223372036854775808".into()
            },
            &project.types
        )
        .unwrap(),
        Value::Literal("-9223372036854775808".into())
    );
    assert!(
        initializer::resolve(
            &field,
            &Input::Scalar {
                text: "9223372036854775808".into()
            },
            &project.types
        )
        .is_err()
    );
    assert!(initializer::resolve(&field, &Input::Null, &project.types).is_err());
    let field = Field {
        nullable: true,
        ..field
    };
    assert_eq!(
        initializer::resolve(&field, &Input::Null, &project.types).unwrap(),
        Value::Null
    );
    assert!(initializer::resolve(&field, &Input::Unset, &project.types).is_err());
    let field = Field {
        type_name: "string".into(),
        ..field
    };
    assert_eq!(
        initializer::resolve(
            &field,
            &Input::Scalar {
                text: "null".into()
            },
            &project.types
        )
        .unwrap(),
        Value::Text("null".into())
    );
    let field = Field {
        type_name: "Reward".into(),
        nullable: false,
        ..field
    };
    let input = Input::Mapping {
        value: vec![
            (
                "values".into(),
                Input::Sequence {
                    value: vec![Input::Scalar {
                        text: "9223372036854775807".into(),
                    }],
                },
            ),
            (
                "itemId".into(),
                Input::Scalar {
                    text: "2001".into(),
                },
            ),
            ("note".into(), Input::Null),
            ("amount".into(), Input::Scalar { text: "1".into() }),
        ],
    };
    assert_eq!(
        initializer::resolve(&field, &input, &project.types).unwrap(),
        Value::Mapping(vec![
            (
                "values".into(),
                Value::Sequence(vec![Value::Literal("9223372036854775807".into())])
            ),
            ("itemId".into(), Value::Literal("2001".into())),
            ("note".into(), Value::Null),
            ("amount".into(), Value::Literal("1".into())),
        ])
    );
}
#[test]
fn type_plan_is_reviewable_with_affected_dirty_source_but_apply_preserves_every_draft() {
    use masterdata_engine::workspace::{SelectionProjection, Workspace};
    let temp = fixture();
    write(
        &temp,
        "independent.yaml",
        "kind: schema\ntable: independent\nfields:\n  - key: 0\n    name: id\n    type: int\nprimaryKey:\n  fields: [id]\nrecords:\n  - id: 1\n",
    );
    let mut w = Workspace::open(temp.path()).unwrap();
    let p = w.select("sources/independent.yaml", 0, 32).unwrap();
    w.edit(
        "sources/independent.yaml",
        p.revision,
        &p.rows[0].id,
        &["id".into()],
        &Value::Literal("2".into()),
    )
    .unwrap();
    let p = w.select("sources/catalog-data.yaml", 0, 32).unwrap();
    w.edit(
        "sources/catalog-data.yaml",
        p.revision,
        &p.rows[0].id,
        &["name".into()],
        &Value::Text("local".into()),
    )
    .unwrap();
    let SelectionProjection::Type(p) = w.select_view("sources/rarity.yaml", 0, 32).unwrap() else {
        panic!()
    };
    let before = disk(&temp);
    let review = w
        .prepare_type_migration(
            &p.source,
            &p.identity,
            rename_enum("Rarity", "Rare", "Epic"),
        )
        .unwrap();
    assert_eq!(review.dirty_sources, vec!["sources/catalog-data.yaml"]);
    assert!(w.apply_migration(&review.token, false).is_err());
    assert_eq!(disk(&temp), before);
    w.discard_source("sources/catalog-data.yaml").unwrap();
    assert_eq!(
        w.apply_migration(&review.token, false).unwrap().outcome,
        Outcome::Success
    );
    let p = w.select("sources/independent.yaml", 0, 32).unwrap();
    assert!(p.dirty && p.can_undo);
    assert_eq!(p.rows[0].cells[0].display, "2");
    assert_eq!(w.dirty_paths(), vec!["sources/independent.yaml"]);
}
#[test]
#[cfg(feature = "oracle-faults")]
fn type_operations_share_stale_closure_rollback_and_recovery_protocol() {
    use masterdata_engine::workspace::Workspace;
    let temp = fixture();
    let command = rename_enum("Rarity", "Rare", "Epic");
    let p = native_plan(&temp, command.clone());
    let before = disk(&temp);
    let config = temp.path().join("masterdata.toml");
    let bytes = fs::read_to_string(&config).unwrap();
    fs::write(&config, bytes.clone() + "\n# changed\n").unwrap();
    assert_eq!(
        p.commit(false, native::SetFault::None).unwrap_err().code,
        "E-MIGRATION-STALE"
    );
    assert_eq!(disk(&temp), before);
    fs::write(config, bytes).unwrap();
    let p = native_plan(&temp, command.clone());
    let failure = p
        .commit(
            false,
            native::SetFault::CommitFailure {
                source: "sources/rarity.yaml".into(),
                rollback_failure: None,
            },
        )
        .unwrap();
    assert_eq!(failure.outcome, Outcome::Failure);
    assert_eq!(disk(&temp), before);
    let p = native_plan(&temp, command);
    let failure = p
        .commit(
            false,
            native::SetFault::CommitFailure {
                source: "sources/rarity.yaml".into(),
                rollback_failure: Some("sources/catalog-data.yaml".into()),
            },
        )
        .unwrap();
    assert_eq!(failure.outcome, Outcome::RecoveryRequired);
    assert!(failure.recovery.is_some());
    let mut w = Workspace::open(temp.path()).unwrap();
    assert!(w.recovery_required);
    let p = w.select("sources/catalog-data.yaml", 0, 32).unwrap();
    assert!(!p.rows.is_empty());
    assert_eq!(
        w.prepare_migration(migration::Command::Type {
            command: rename_enum("ItemTags", "Fire", "Flame")
        })
        .unwrap_err()
        .code,
        "E-RECOVERY-REQUIRED"
    );
}
