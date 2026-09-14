use masterdata_core::*;
use serde_yaml::Value;
fn docs() -> ProjectDocuments {
    let sources = [
        (
            "enum.yaml",
            "kind: type\nname: Rarity\nenum:\n  underlying: ulong\n  members:\n    - name: 'Rare' # keep name\n      value: 18446744073709551615\n    # between\n    - name: Common\n      value: 0\n",
        ),
        (
            "flags.yaml",
            "kind: type\nname: Feature\nflags:\n  underlying: ulong\n  members:\n    - name: None\n      value: 0\n    - name: Fire\n      value: 1\n",
        ),
        (
            "custom.yaml",
            "kind: type\nname: Reward\ncustom:\n  fields:\n    - key: 0\n      name: rarity\n      type: Rarity\n    - key: 1\n      name: features\n      type: Feature\n    - key: 2\n      name: note\n      type: string\n",
        ),
        (
            "vo.yaml",
            "kind: type\nname: ItemId\nvalueObject:\n  underlying: int # identity\n",
        ),
        (
            "schema.yaml",
            "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: ItemId\n  - key: 1\n    name: rewards\n    type: Reward\n    array: true\nprimaryKey:\n  fields: [id]\n",
        ),
        (
            "data.yaml",
            "kind: data\ntable: item\nrecords:\n  - id: 1\n    rewards:\n      - rarity: 'Rare' # occurrence\n        features: [Fire, # retain\n          ]\n        note: |\n          日本語\n          # literal data\n      # between values\n      - rarity: Common\n        features:\n          - Fire\n        note: \"Rare\" # unrelated\n",
        ),
    ];
    ProjectDocuments {
        files: sources
            .into_iter()
            .map(|(p, s)| parse_yaml_document(p.into(), s).unwrap())
            .collect(),
    }
}
fn run(
    docs: &ProjectDocuments,
    target: &str,
    operation: TypeMigrationOperation,
) -> TypeMigrationDryRun {
    dry_run_type_migration(
        docs,
        &TypeMigrationCommand {
            target: target.into(),
            operation,
        },
    )
    .unwrap()
}
fn text<'a>(result: &'a TypeMigrationDryRun, path: &str) -> &'a str {
    &result
        .candidate
        .transformed_documents
        .files
        .iter()
        .find(|f| f.path.to_str() == Some(path))
        .unwrap()
        .source
}
#[test]
fn enum_rename_updates_nested_occurrences_and_preserves_unrelated_text() {
    let docs = docs();
    let op = TypeMigrationOperation::RenameEnumMember {
        member: "Rare".into(),
        new_name: "Epic".into(),
    };
    let result = run(&docs, "Rarity", op.clone());
    assert_eq!(result, run(&docs, "Rarity", op));
    assert_eq!(result.affected_occurrence_count, 1);
    assert!(text(&result, "enum.yaml").contains("name: 'Epic' # keep name"));
    assert!(text(&result, "enum.yaml").contains("18446744073709551615"));
    assert!(text(&result, "data.yaml").contains("rarity: 'Epic' # occurrence"));
    assert!(text(&result, "data.yaml").contains("note: \"Rare\" # unrelated"));
    assert!(text(&result, "data.yaml").contains("# literal data"));
    assert_eq!(text(&result, "schema.yaml"), docs.files[4].source);
}
#[test]
fn flags_rename_handles_multiline_flow_and_block_sequences() {
    let r = run(
        &docs(),
        "Feature",
        TypeMigrationOperation::RenameEnumMember {
            member: "Fire".into(),
            new_name: "Water".into(),
        },
    );
    assert_eq!(r.affected_occurrence_count, 2);
    assert!(text(&r, "data.yaml").contains("[Water, # retain"));
    assert!(text(&r, "data.yaml").contains("- Water\n"));
}
#[test]
fn custom_add_rename_drop_preserve_nested_mappings() {
    let docs = docs();
    let rename = run(
        &docs,
        "Reward",
        TypeMigrationOperation::RenameCustomField {
            field: "rarity".into(),
            new_name: "grade".into(),
        },
    );
    assert_eq!(rename.affected_occurrence_count, 2);
    assert!(text(&rename, "data.yaml").contains("- grade: 'Rare'"));
    let add = run(
        &docs,
        "Reward",
        TypeMigrationOperation::AddCustomField {
            field: TypeFieldDefinition {
                key: 3,
                name: "amount".into(),
                type_name: "ulong".into(),
                nullable: false,
                array: false,
            },
            initializer: Some(Value::from(u64::MAX)),
        },
    );
    assert_eq!(add.affected_occurrence_count, 2);
    assert_eq!(
        text(&add, "data.yaml")
            .matches("amount: 18446744073709551615")
            .count(),
        2
    );
    let drop = run(
        &docs,
        "Reward",
        TypeMigrationOperation::DropCustomField {
            field: "rarity".into(),
        },
    );
    assert!(drop.candidate.destructive);
    assert!(!text(&drop, "data.yaml").contains("rarity:"));
    assert!(text(&drop, "data.yaml").contains("- features:"));
    let drop = run(
        &docs,
        "Reward",
        TypeMigrationOperation::DropCustomField {
            field: "note".into(),
        },
    );
    assert!(!text(&drop, "data.yaml").contains("# literal data"));
    assert!(text(&drop, "data.yaml").contains("# between values"));
}
#[test]
fn value_object_conversions_only_change_settings() {
    let r = run(
        &docs(),
        "ItemId",
        TypeMigrationOperation::SetValueObjectConversions(ConversionDefinition {
            from_underlying_implicit: true,
            to_underlying_implicit: false,
        }),
    );
    assert_eq!(r.candidate.affected_files.len(), 1);
    assert!(text(&r, "vo.yaml").contains("underlying: int # identity"));
    assert!(text(&r, "vo.yaml").contains("fromUnderlyingImplicit: true"));
}
#[test]
fn invalid_operations_fail_closed() {
    let docs = docs();
    for (target, op) in [
        (
            "Rarity",
            TypeMigrationOperation::DropEnumMember {
                member: "Rare".into(),
            },
        ),
        (
            "Feature",
            TypeMigrationOperation::DropEnumMember {
                member: "None".into(),
            },
        ),
        (
            "Feature",
            TypeMigrationOperation::RenameEnumMember {
                member: "None".into(),
                new_name: "Zero".into(),
            },
        ),
        (
            "Feature",
            TypeMigrationOperation::AddEnumMember(EnumMember {
                name: "Combo".into(),
                value: IntegerLiteral(3),
            }),
        ),
        (
            "Rarity",
            TypeMigrationOperation::AddEnumMember(EnumMember {
                name: "Rare".into(),
                value: IntegerLiteral(1),
            }),
        ),
        (
            "Reward",
            TypeMigrationOperation::AddCustomField {
                field: TypeFieldDefinition {
                    key: 3,
                    name: "amount".into(),
                    type_name: "int".into(),
                    nullable: false,
                    array: false,
                },
                initializer: None,
            },
        ),
    ] {
        assert!(
            dry_run_type_migration(
                &docs,
                &TypeMigrationCommand {
                    target: target.into(),
                    operation: op
                }
            )
            .is_err()
        );
    }
}
#[test]
fn unrelated_invalid_type_is_excluded_but_unknown_data_dependency_blocks() {
    let mut d = docs();
    d.files.push(
        parse_yaml_document(
            "invalid.yaml".into(),
            "kind: type\nname: Invalid\nvalueObject:\n  underlying: bool\n",
        )
        .unwrap(),
    );
    run(
        &d,
        "Rarity",
        TypeMigrationOperation::RenameEnumMember {
            member: "Rare".into(),
            new_name: "Epic".into(),
        },
    );
    d.files.push(
        parse_yaml_document(
            "unknown.yaml".into(),
            "kind: data\ntable: missing\nrecords: []\n",
        )
        .unwrap(),
    );
    assert!(
        dry_run_type_migration(
            &d,
            &TypeMigrationCommand {
                target: "Rarity".into(),
                operation: TypeMigrationOperation::DropEnumMember {
                    member: "Common".into()
                }
            }
        )
        .is_err()
    );
}

#[test]
fn conversion_combinations_and_enum_add_do_not_require_unrelated_data_validity() {
    let mut d = docs();
    let i = d
        .files
        .iter()
        .position(|f| f.path.to_str() == Some("data.yaml"))
        .unwrap();
    let source = d.files[i].source.replace("id: 1", "id: wrong");
    d.files[i] = parse_yaml_document("data.yaml".into(), &source).unwrap();
    for from in [false, true] {
        for to in [false, true] {
            run(
                &d,
                "ItemId",
                TypeMigrationOperation::SetValueObjectConversions(ConversionDefinition {
                    from_underlying_implicit: from,
                    to_underlying_implicit: to,
                }),
            );
        }
    }
    run(
        &d,
        "Rarity",
        TypeMigrationOperation::AddEnumMember(EnumMember {
            name: "New".into(),
            value: IntegerLiteral(10),
        }),
    );
}
#[test]
fn custom_initializer_validates_cycles_keys_and_exact_scalar_category() {
    let d = docs();
    for (key, name, ty, nullable, array, initializer) in [
        (0, "amount", "int", false, false, Value::from(1)),
        (3, "equals", "int", false, false, Value::from(1)),
        (3, "amount", "Reward", false, false, Value::Null),
        (3, "amount", "int", true, true, Value::Null),
        (3, "amount", "int", false, false, Value::from(1.0)),
    ] {
        assert!(
            dry_run_type_migration(
                &d,
                &TypeMigrationCommand {
                    target: "Reward".into(),
                    operation: TypeMigrationOperation::AddCustomField {
                        field: TypeFieldDefinition {
                            key,
                            name: name.into(),
                            type_name: ty.into(),
                            nullable,
                            array
                        },
                        initializer: Some(initializer)
                    }
                }
            )
            .is_err()
        );
    }
}
#[test]
fn bare_sequence_declaration_drop_preserves_comments_and_crlf() {
    let source = "kind: type\r\nname: Choice\r\nenum:\r\n  underlying: long\r\n  members:\r\n    - # first\r\n      name: First\r\n      value: -9223372036854775808\r\n    # keep\r\n    -\r\n      name: Second\r\n      value: 9223372036854775807\r\n";
    let d = ProjectDocuments {
        files: vec![parse_yaml_document("choice.yaml".into(), source).unwrap()],
    };
    let r = run(
        &d,
        "Choice",
        TypeMigrationOperation::DropEnumMember {
            member: "First".into(),
        },
    );
    let source = text(&r, "choice.yaml");
    assert!(source.contains("# first\r\n"));
    assert!(source.contains("# keep\r\n"));
    assert!(source.contains("9223372036854775807"));
    assert!(!source.contains("First"));
}
#[test]
fn zero_occurrence_custom_add_allows_absent_initializer_and_last_field_drop_fails() {
    let d=ProjectDocuments{files:vec![parse_yaml_document("custom.yaml".into(),"kind: type\nname: Reward\ncustom:\n  fields:\n    - key: 0\n      name: amount\n      type: int\n").unwrap()]};
    let r = run(
        &d,
        "Reward",
        TypeMigrationOperation::AddCustomField {
            field: TypeFieldDefinition {
                key: 1,
                name: "note".into(),
                type_name: "string".into(),
                nullable: true,
                array: false,
            },
            initializer: None,
        },
    );
    assert_eq!(r.affected_occurrence_count, 0);
    assert!(
        dry_run_type_migration(
            &d,
            &TypeMigrationCommand {
                target: "Reward".into(),
                operation: TypeMigrationOperation::DropCustomField {
                    field: "amount".into()
                }
            }
        )
        .is_err()
    );
}

#[test]
fn unrelated_scalar_error_inside_dependent_custom_does_not_block_enum_rename() {
    let mut d = docs();
    let i = d
        .files
        .iter()
        .position(|f| f.path.to_str() == Some("data.yaml"))
        .unwrap();
    let source = d.files[i].source.replace("note: \"Rare\"", "note: 42");
    d.files[i] = parse_yaml_document("data.yaml".into(), &source).unwrap();
    let r = run(
        &d,
        "Rarity",
        TypeMigrationOperation::RenameEnumMember {
            member: "Rare".into(),
            new_name: "Epic".into(),
        },
    );
    assert!(text(&r, "data.yaml").contains("note: 42 # unrelated"));
}

#[test]
fn custom_mutation_reaches_nested_custom_array_and_skips_nullable_absence() {
    let mut d = docs();
    d.files.push(parse_yaml_document("wrapper.yaml".into(),"kind: type\nname: Wrapper\ncustom:\n  fields:\n    - key: 0\n      name: reward\n      type: Reward\n      nullable: true\n").unwrap());
    let si = d
        .files
        .iter()
        .position(|f| f.path.to_str() == Some("schema.yaml"))
        .unwrap();
    d.files[si] = parse_yaml_document(
        "schema.yaml".into(),
        &d.files[si].source.replace("type: Reward", "type: Wrapper"),
    )
    .unwrap();
    let di = d
        .files
        .iter()
        .position(|f| f.path.to_str() == Some("data.yaml"))
        .unwrap();
    d.files[di]=parse_yaml_document("data.yaml".into(),"kind: data\ntable: item\nrecords:\n  - id: 1\n    rewards:\n      - reward: null # preserve\n      - reward:\n          rarity: Rare\n          features: []\n          note: nested\n").unwrap();
    let r = run(
        &d,
        "Reward",
        TypeMigrationOperation::RenameCustomField {
            field: "rarity".into(),
            new_name: "grade".into(),
        },
    );
    assert_eq!(r.affected_occurrence_count, 1);
    assert!(text(&r, "data.yaml").contains("grade: Rare"));
    assert!(text(&r, "data.yaml").contains("reward: null # preserve"));
}
