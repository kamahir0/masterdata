use std::path::PathBuf;

use masterdata_core::{
    FieldModifier, PrimitiveType, ProjectDocuments, ResolvedType, TypeReference, TypeSystem,
    build_type_system, parse_yaml_document, validate_documents,
};
use serde_yaml::Value;

fn documents(sources: &[(&str, &str)]) -> ProjectDocuments {
    ProjectDocuments {
        files: sources
            .iter()
            .map(|(path, source)| parse_yaml_document(PathBuf::from(path), source).expect("YAML"))
            .collect(),
    }
}

fn type_system(source: &str) -> TypeSystem {
    let build = build_type_system(&documents(&[("type.yaml", source)]));
    assert!(
        build.diagnostics.is_empty(),
        "unexpected diagnostics: {:?}",
        build.diagnostics
    );
    build.model.expect("type system")
}

#[test]
fn type_documents_are_typed_and_resolved_by_category() {
    let documents = documents(&[
        (
            "item-id.yaml",
            "kind: type\nname: ItemId\nvalueObject:\n  underlying: int\n",
        ),
        (
            "reward.yaml",
            "kind: type\nname: Reward\ncustom:\n  fields:\n    - key: 5\n      name: note\n      type: string\n      nullable: true\n    - key: 1\n      name: itemId\n      type: ItemId\n    - key: 3\n      name: amounts\n      type: int\n      array: true\n",
        ),
        (
            "rarity.yaml",
            "kind: type\nname: ItemRarity\nenum:\n  underlying: long\n  members:\n    - name: Common\n      value: -2\n    - name: Legendary\n      value: 100\n",
        ),
        (
            "feature.yaml",
            "kind: type\nname: Feature\nflags:\n  underlying: int\n  members:\n    - name: None\n      value: 0\n    - name: Fire\n      value: 1\n    - name: Highest\n      value: -2147483648\n",
        ),
    ]);
    let build = build_type_system(&documents);
    assert!(build.diagnostics.is_empty(), "{:?}", build.diagnostics);
    let model = build.model.expect("resolved model");

    assert!(matches!(
        model.get("ItemId"),
        Some(ResolvedType::ValueObject { .. })
    ));
    assert!(matches!(
        model.get("Reward"),
        Some(ResolvedType::Custom { .. })
    ));
    assert!(matches!(
        model.get("ItemRarity"),
        Some(ResolvedType::Enum { .. })
    ));
    assert!(matches!(
        model.get("Feature"),
        Some(ResolvedType::Flags { .. })
    ));
    assert!(model.is_key_compatible(&TypeReference::Named("ItemId".to_owned())));
    assert!(model.is_key_compatible(&TypeReference::Named("ItemRarity".to_owned())));
    assert!(!model.is_key_compatible(&TypeReference::Named("Reward".to_owned())));
    assert!(!model.is_key_compatible(&TypeReference::Named("Feature".to_owned())));
    assert!(model.is_field_key_compatible(
        &TypeReference::Named("ItemId".to_owned()),
        FieldModifier::Required
    ));
    assert!(model.is_field_key_compatible(
        &TypeReference::Named("ItemRarity".to_owned()),
        FieldModifier::Required
    ));
    assert!(!model.is_field_key_compatible(
        &TypeReference::Named("ItemId".to_owned()),
        FieldModifier::Nullable
    ));
    assert!(model.is_comparison_capable(&TypeReference::Named("ItemId".to_owned())));
    assert!(model.is_comparison_capable(&TypeReference::Named("ItemRarity".to_owned())));
    assert!(!model.is_comparison_capable(&TypeReference::Named("Feature".to_owned())));

    let highest: Value = serde_yaml::from_str("[Highest]").expect("signed highest flag");
    assert_eq!(
        model.resolve_flags_value("Feature", &highest).unwrap(),
        1_u128 << 31
    );
}

#[test]
fn primitive_vocabulary_and_value_object_underlyings_are_exact() {
    for name in [
        "bool", "int", "uint", "long", "ulong", "float", "double", "string",
    ] {
        assert!(PrimitiveType::parse(name).is_some(), "{name}");
    }
    for alias in [
        "byte", "sbyte", "short", "ushort", "int16", "int32", "int64", "uint8", "uint16", "uint32",
        "uint64",
    ] {
        assert!(
            PrimitiveType::parse(alias).is_none(),
            "legacy alias {alias}"
        );
    }

    let sources = ["int", "uint", "long", "ulong", "string"]
        .into_iter()
        .enumerate()
        .map(|(index, underlying)| {
            (
                format!("type-{index}.yaml"),
                format!(
                    "kind: type\nname: Type{index}\nvalueObject:\n  underlying: {underlying}\n"
                ),
            )
        })
        .collect::<Vec<_>>();
    let source_refs = sources
        .iter()
        .map(|(path, source)| (path.as_str(), source.as_str()))
        .collect::<Vec<_>>();
    let build = build_type_system(&documents(&source_refs));
    assert!(build.diagnostics.is_empty(), "{:?}", build.diagnostics);
    let model = build.model.expect("value object model");
    for index in 0..5 {
        assert!(matches!(
            model.get(&format!("Type{index}")),
            Some(ResolvedType::ValueObject { .. })
        ));
    }
}

#[test]
fn conversion_options_use_canonical_camel_case_and_default_to_false() {
    let documents = documents(&[
        (
            "enabled.yaml",
            "kind: type\nname: Enabled\nvalueObject:\n  underlying: int\n  conversions:\n    fromUnderlyingImplicit: true\n",
        ),
        (
            "disabled.yaml",
            "kind: type\nname: Disabled\nvalueObject:\n  underlying: int\n",
        ),
    ]);
    let build = build_type_system(&documents);
    assert!(build.diagnostics.is_empty(), "{:?}", build.diagnostics);
    let model = build.model.expect("conversion model");
    let ResolvedType::ValueObject { conversions, .. } = model.get("Enabled").unwrap() else {
        panic!("expected Value Object")
    };
    assert!(conversions.from_underlying_implicit);
    assert!(!conversions.to_underlying_implicit);
    let ResolvedType::ValueObject { conversions, .. } = model.get("Disabled").unwrap() else {
        panic!("expected Value Object")
    };
    assert!(!conversions.from_underlying_implicit);
    assert!(!conversions.to_underlying_implicit);
}

#[test]
fn type_name_grammar_accepts_acronyms_and_rejects_repairs() {
    let valid_names = ["A", "AB", "ID", "URL", "REWARD", "Item2", "XML2Data"];
    let valid_sources = valid_names
        .iter()
        .enumerate()
        .map(|(index, name)| {
            (
                format!("valid-{index}.yaml"),
                format!("kind: type\nname: {name}\nvalueObject:\n  underlying: int\n"),
            )
        })
        .collect::<Vec<_>>();
    let valid_refs = valid_sources
        .iter()
        .map(|(path, source)| (path.as_str(), source.as_str()))
        .collect::<Vec<_>>();
    let valid_build = build_type_system(&documents(&valid_refs));
    assert!(
        valid_build.diagnostics.is_empty(),
        "{:?}",
        valid_build.diagnostics
    );

    for (index, name) in [
        "itemId",
        "reward_condition",
        "reward-condition",
        "1Reward",
        "あいう",
    ]
    .into_iter()
    .enumerate()
    {
        let codes = diagnostic_codes(&format!(
            "kind: type\nname: {name}\nvalueObject:\n  underlying: int\n"
        ));
        assert!(
            codes.contains(&"E-TYPE-INVALID-NAME".to_owned()),
            "{index}: {codes:?}"
        );
    }
}

#[test]
fn source_document_separates_table_and_type_identity() {
    let loaded = parse_yaml_document(
        PathBuf::from("item-id.yaml"),
        "kind: type\nname: ItemId\nvalueObject:\n  underlying: int\n",
    )
    .expect("type YAML");
    assert_eq!(loaded.document.kind(), "type");
    assert_eq!(loaded.document.table_identity(), None);
    assert_eq!(loaded.document.type_name(), Some("ItemId"));
}

#[test]
fn one_field_custom_type_is_mapping_and_rejects_unknown_members() {
    let model = type_system(
        "kind: type\nname: EnabledState\ncustom:\n  fields:\n    - key: 0\n      name: value\n      type: bool\n",
    );
    let valid: Value = serde_yaml::from_str("value: true").expect("mapping");
    model
        .validate_value("EnabledState", &valid)
        .expect("valid mapping");

    let scalar: Value = serde_yaml::from_str("true").expect("scalar");
    assert_eq!(
        model
            .validate_value("EnabledState", &scalar)
            .unwrap_err()
            .diagnostic()
            .code,
        "E-TYPE-CUSTOM-DATA-SHAPE"
    );
    let unknown: Value = serde_yaml::from_str("value: true\nextra: false").expect("mapping");
    assert_eq!(
        model
            .validate_value("EnabledState", &unknown)
            .unwrap_err()
            .diagnostic()
            .code,
        "E-TYPE-CUSTOM-UNKNOWN-MEMBER"
    );
}

#[test]
fn nested_custom_types_remain_structural_mappings() {
    let documents = documents(&[
        (
            "reward.yaml",
            "kind: type\nname: Reward\ncustom:\n  fields:\n    - key: 0\n      name: cost\n      type: Cost\n",
        ),
        (
            "cost.yaml",
            "kind: type\nname: Cost\ncustom:\n  fields:\n    - key: 0\n      name: amount\n      type: int\n",
        ),
    ]);
    let build = build_type_system(&documents);
    assert!(build.diagnostics.is_empty(), "{:?}", build.diagnostics);
    let model = build.model.expect("nested Custom Type model");
    let reward: Value = serde_yaml::from_str("cost:\n  amount: 3").expect("nested mapping");
    model
        .validate_value("Reward", &reward)
        .expect("nested Custom Type mapping");

    let scalar: Value = serde_yaml::from_str("3").expect("scalar");
    assert!(model.validate_value("Reward", &scalar).is_err());
}

#[test]
fn field_modifiers_are_structured_and_arrays_are_immutable_shapes() {
    let model = type_system(
        "kind: type\nname: Reward\ncustom:\n  fields:\n    - key: 0\n      name: note\n      type: string\n      nullable: true\n    - key: 1\n      name: tags\n      type: string\n      array: true\n",
    );
    let ResolvedType::Custom { fields, .. } = model.get("Reward").expect("custom") else {
        panic!("expected custom")
    };
    assert_eq!(fields[0].modifier, FieldModifier::Nullable);
    assert_eq!(fields[1].modifier, FieldModifier::Array);

    let valid: Value = serde_yaml::from_str("note: null\ntags: []").expect("mapping");
    model
        .validate_value("Reward", &valid)
        .expect("valid values");
    let wrong: Value = serde_yaml::from_str("note: null\ntags: null").expect("mapping");
    assert_eq!(
        model
            .validate_value("Reward", &wrong)
            .unwrap_err()
            .diagnostic()
            .code,
        "E-TYPE-NULL-NOT-ALLOWED"
    );
    assert!(!model.is_field_key_compatible(
        &TypeReference::Named("Reward".to_owned()),
        FieldModifier::Required
    ));
    assert!(!model.is_field_comparison_capable(
        &TypeReference::Primitive(PrimitiveType::Int),
        FieldModifier::Array
    ));
}

#[test]
fn primitive_scalar_categories_are_strict() {
    let model = TypeSystem::default();
    let integer: Value = serde_yaml::from_str("1").expect("integer");
    let floating: Value = serde_yaml::from_str("1.0").expect("float");
    let boolean: Value = serde_yaml::from_str("true").expect("boolean");
    let empty: Value = serde_yaml::from_str("\"\"").expect("empty string");
    model.validate_value("int", &integer).expect("int");
    model
        .validate_value("string", &empty)
        .expect("empty string");
    assert!(model.validate_value("float", &integer).is_err());
    assert!(model.validate_value("int", &floating).is_err());
    assert!(model.validate_value("string", &boolean).is_err());

    let too_large: Value = serde_yaml::from_str("2147483648").expect("integer");
    assert_eq!(
        model
            .validate_value("int", &too_large)
            .unwrap_err()
            .diagnostic()
            .code,
        "E-TYPE-INTEGER-OUT-OF-RANGE"
    );
    assert_eq!(PrimitiveType::Int.integer_width(), Some(32));
}

#[test]
fn enum_and_flags_data_use_their_symbolic_shapes() {
    let model = type_system(
        "kind: type\nname: Rarity\nenum:\n  underlying: int\n  members:\n    - name: Common\n      value: 1\n    - name: Rare\n      value: 10\n\n",
    );
    let symbolic: Value = serde_yaml::from_str("Rare").expect("symbol");
    model
        .validate_value("Rarity", &symbolic)
        .expect("symbolic enum");
    let numeric: Value = serde_yaml::from_str("10").expect("numeric");
    assert!(model.validate_value("Rarity", &numeric).is_err());

    let flags = type_system(
        "kind: type\nname: Feature\nflags:\n  underlying: uint\n  members:\n    - name: None\n      value: 0\n    - name: Fire\n      value: 1\n    - name: Ice\n      value: 2\n",
    );
    for source in ["[None]", "[Fire, Ice]", "[Ice, Fire]"] {
        let value: Value = serde_yaml::from_str(source).expect("flags");
        flags
            .validate_value("Feature", &value)
            .expect("valid flags");
    }
    let fire_ice: Value = serde_yaml::from_str("[Fire, Ice]").expect("flags");
    let ice_fire: Value = serde_yaml::from_str("[Ice, Fire]").expect("flags");
    assert_eq!(flags.resolve_flags_value("Feature", &fire_ice).unwrap(), 3);
    assert_eq!(
        flags.resolve_flags_value("Feature", &fire_ice).unwrap(),
        flags.resolve_flags_value("Feature", &ice_fire).unwrap()
    );
    for source in ["[]", "[None, Fire]", "[Fire, Fire]", "3", "Fire | Ice"] {
        let value: Value = serde_yaml::from_str(source).expect("flags value");
        assert!(flags.validate_value("Feature", &value).is_err(), "{source}");
    }
}

#[test]
fn enum_integer_lexemes_accept_canonical_decimal_boundaries() {
    for (underlying, value) in [
        ("int", "0"),
        ("int", "-0"),
        ("int", "1"),
        ("int", "-1"),
        ("int", "2147483647"),
        ("long", "-9223372036854775808"),
        ("ulong", "18446744073709551615"),
    ] {
        let source = format!(
            "kind: type\nname: Boundary\nenum:\n  underlying: {underlying}\n  members:\n    - name: Value\n      value: {value}\n"
        );
        let loaded = parse_yaml_document(PathBuf::from("boundary.yaml"), &source)
            .expect("canonical integer lexeme");
        let build = build_type_system(&ProjectDocuments {
            files: vec![loaded],
        });
        assert!(
            build.diagnostics.is_empty(),
            "{underlying} {value}: {:?}",
            build.diagnostics
        );
    }
}

#[test]
fn enum_integer_lexemes_reject_noncanonical_numeric_forms() {
    for value in [
        "+1", "01", "-01", "0x1", "0X1", "0o1", "0O1", "0b1", "0B1", "1_000", "1.0", "1e3",
    ] {
        let source = format!(
            "kind: type\nname: Invalid\nenum:\n  underlying: int\n  members:\n    - name: Value\n      value: {value}\n"
        );
        let error = parse_yaml_document(PathBuf::from("invalid.yaml"), &source)
            .expect_err("noncanonical integer lexeme");
        assert_eq!(error.diagnostic().code, "E-YAML-INVALID-INTEGER", "{value}");
        assert_eq!(
            error.diagnostic().related_requirements,
            ["YAML-SUBSET-011"],
            "{value}"
        );
    }
}

#[test]
fn integer_lexical_gate_applies_to_flags_and_ignores_non_integer_text() {
    let enum_source = "kind: type\nname: Rarity\nenum:\n  underlying: int\n  members:\n    - name: Common\n      value: 1 # 0x1\n";
    let enum_document = parse_yaml_document(PathBuf::from("enum.yaml"), enum_source)
        .expect("comment after canonical integer");
    let enum_build = build_type_system(&ProjectDocuments {
        files: vec![enum_document],
    });
    assert!(
        enum_build.diagnostics.is_empty(),
        "{:?}",
        enum_build.diagnostics
    );

    let flags_source = "kind: type\nname: Feature\nflags:\n  underlying: uint\n  members:\n    - name: None\n      value: 0\n    - name: Fire\n      value: +1\n";
    let error = parse_yaml_document(PathBuf::from("flags.yaml"), flags_source)
        .expect_err("flags must use canonical integer lexemes");
    assert_eq!(error.diagnostic().code, "E-YAML-INVALID-INTEGER");
    assert_eq!(error.diagnostic().related_requirements, ["YAML-SUBSET-011"]);

    let bare_dash_source = "kind: type\nname: BareDash\nenum:\n  underlying: int\n  members:\n    -\n      name: Value\n      value: 0x1\n";
    let error = parse_yaml_document(PathBuf::from("bare-dash.yaml"), bare_dash_source)
        .expect_err("bare sequence item must still be checked");
    assert_eq!(error.diagnostic().code, "E-YAML-INVALID-INTEGER");

    let indentless_source = "kind: type\nname: Indentless\nenum:\n  underlying: int\n  members:\n  - name: Value\n    value: 0x1\n";
    let error = parse_yaml_document(PathBuf::from("indentless.yaml"), indentless_source)
        .expect_err("indentless sequence item must still be checked");
    assert_eq!(error.diagnostic().code, "E-YAML-INVALID-INTEGER");

    let quoted_string = parse_yaml_document(
        PathBuf::from("quoted.yaml"),
        "kind: data\ntable: item\nrecords:\n  - name: \"0x1\"\n",
    )
    .expect("quoted string");
    assert_eq!(quoted_string.document.kind(), "data");

    let block_scalar = parse_yaml_document(
        PathBuf::from("block.yaml"),
        "kind: data\ntable: item\nrecords:\n  - description: |\n      0x1\n      +1\n",
    )
    .expect("literal block scalar");
    assert_eq!(block_scalar.document.kind(), "data");
}

#[test]
fn schema_validation_collects_types_and_migrated_messagepack_keys() {
    let documents = documents(&[
        (
            "item-id.yaml",
            "kind: type\nname: ItemId\nvalueObject:\n  underlying: int\n",
        ),
        (
            "item-schema.yaml",
            "kind: schema\ntable: item\nfields:\n  - key: 4\n    name: itemId\n    type: ItemId\n  - key: 1\n    name: tags\n    type: string\n    array: true\nprimaryKey:\n  fields: [itemId]\n",
        ),
    ]);
    let report = validate_documents(&documents);
    assert!(report.valid, "{report:?}");
    assert_eq!(report.type_documents, 1);
    assert_eq!(report.types, ["ItemId"]);
}

fn diagnostic_codes(source: &str) -> Vec<String> {
    let documents = documents(&[("invalid.yaml", source)]);
    validate_documents(&documents)
        .diagnostics
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect()
}

#[test]
fn invalid_type_declarations_have_individual_diagnostics() {
    assert!(
        diagnostic_codes("kind: type\nname: itemId\nvalueObject:\n  underlying: int\n")
            .contains(&"E-TYPE-INVALID-NAME".to_owned())
    );
    for underlying in ["bool", "float", "double"] {
        assert!(
            diagnostic_codes(&format!(
                "kind: type\nname: ItemId\nvalueObject:\n  underlying: {underlying}\n"
            ))
            .contains(&"E-VO-INVALID-UNDERLYING".to_owned()),
            "{underlying}"
        );
    }
    let missing_underlying = parse_yaml_document(
        PathBuf::from("missing-underlying.yaml"),
        "kind: type\nname: ItemRarity\nenum:\n  members:\n    - name: Common\n      value: 1\n",
    )
    .expect_err("missing underlying");
    assert_eq!(missing_underlying.diagnostic().code, "E-YAML-SHAPE");
    assert!(diagnostic_codes(
        "kind: type\nname: Rarity\nenum:\n  underlying: int\n  members:\n    - name: Common\n      value: 1\n    - name: Rare\n      value: 1\n"
    )
    .contains(&"E-ENUM-DUPLICATE-VALUE".to_owned()));
    assert!(diagnostic_codes(
        "kind: type\nname: Feature\nflags:\n  underlying: int\n  members:\n    - name: None\n      value: 0\n    - name: FireAndIce\n      value: 3\n"
    )
    .contains(&"E-FLAGS-NON_ATOMIC-MEMBER".to_owned()));
    assert!(diagnostic_codes(
        "kind: type\nname: Reward\ncustom:\n  fields:\n    - key: 0\n      name: value\n      type: Reward\n"
    )
    .contains(&"E-CUSTOM-RECURSION".to_owned()));
}

#[test]
fn invalid_type_boundaries_have_specific_diagnostics() {
    assert!(diagnostic_codes(
        "kind: type\nname: Reward\ncustom:\n  fields:\n    - key: 0\n      name: first\n      type: int\n    - key: 0\n      name: second\n      type: int\n"
    )
    .contains(&"E-TYPE-DUPLICATE-FIELD-KEY".to_owned()));
    assert!(diagnostic_codes(
        "kind: type\nname: Reward\ncustom:\n  fields:\n    - key: 0\n      name: class\n      type: int\n"
    )
    .contains(&"E-TYPE-INVALID-FIELD-NAME".to_owned()));
    assert!(diagnostic_codes(
        "kind: type\nname: Reward\ncustom:\n  fields:\n    - key: 0\n      name: child\n      type: MissingType\n"
    )
    .contains(&"E-TYPE-UNKNOWN-REFERENCE".to_owned()));
    assert!(diagnostic_codes(
        "kind: type\nname: Reward\ncustom:\n  fields:\n    - key: 0\n      name: child\n      type: int\n      nullable: true\n      array: true\n"
    )
    .contains(&"E-TYPE-INVALID-MODIFIERS".to_owned()));
    assert!(diagnostic_codes(
        "kind: type\nname: Rarity\nenum:\n  underlying: int\n  members:\n    - name: Common\n      value: 1\n    - name: Common\n      value: 2\n"
    )
    .contains(&"E-ENUM-DUPLICATE-MEMBER-NAME".to_owned()));
    assert!(diagnostic_codes(
        "kind: type\nname: Feature\nflags:\n  underlying: int\n  members:\n    - name: Fire\n      value: 1\n"
    )
    .contains(&"E-FLAGS-NONE".to_owned()));
    assert!(diagnostic_codes(
        "kind: type\nname: Feature\nflags:\n  underlying: int\n  members:\n    - name: None\n      value: 0\n    - name: Fire\n      value: 1\n    - name: Ice\n      value: 1\n"
    )
    .contains(&"E-ENUM-DUPLICATE-VALUE".to_owned()));
    assert!(diagnostic_codes(
        "kind: type\nname: Feature\nflags:\n  underlying: int\n  members:\n    - name: None\n      value: 0\n    - name: OtherZero\n      value: 0\n"
    )
    .contains(&"E-FLAGS-NONE".to_owned()));

    assert!(diagnostic_codes(
        "kind: type\nname: Reward\ncustom:\n  fields:\n    - key: 0\n      name: equals\n      type: int\n"
    )
    .contains(&"E-TYPE-GENERATED-MEMBER-COLLISION".to_owned()));
    assert!(diagnostic_codes(
        "kind: type\nname: Reward\ncustom:\n  fields:\n    - key: 0\n      name: reward\n      type: int\n"
    )
    .contains(&"E-TYPE-GENERATED-MEMBER-COLLISION".to_owned()));
    assert!(
        diagnostic_codes("kind: type\nname: Value\nvalueObject:\n  underlying: int\n")
            .contains(&"E-TYPE-GENERATED-MEMBER-COLLISION".to_owned())
    );

    let duplicate_types = documents(&[
        (
            "first.yaml",
            "kind: type\nname: Reward\ncustom:\n  fields:\n    - key: 0\n      name: amount\n      type: int\n",
        ),
        (
            "second.yaml",
            "kind: type\nname: Reward\ncustom:\n  fields:\n    - key: 0\n      name: amount\n      type: int\n",
        ),
    ]);
    assert!(
        validate_documents(&duplicate_types)
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == "E-TYPE-DUPLICATE-NAME")
    );
}

#[test]
fn indirect_and_modifier_wrapped_custom_cycles_are_rejected() {
    for modifier in ["", "\n      nullable: true", "\n      array: true"] {
        let source = format!(
            "kind: type\nname: A\ncustom:\n  fields:\n    - key: 0\n      name: b\n      type: B{modifier}\n---\nkind: type\nname: B\ncustom:\n  fields:\n    - key: 0\n      name: a\n      type: A\n"
        );
        // A source file contains one document by contract; this test builds
        // the two declarations as two typed documents to exercise the graph.
        let a_source = source.split("---").next().expect("A document").to_owned();
        let documents = documents(&[
            ("a.yaml", &a_source),
            (
                "b.yaml",
                "kind: type\nname: B\ncustom:\n  fields:\n    - key: 0\n      name: a\n      type: A\n",
            ),
        ]);
        let codes = validate_documents(&documents)
            .diagnostics
            .into_iter()
            .map(|diagnostic| diagnostic.code)
            .collect::<Vec<_>>();
        assert!(
            codes.contains(&"E-CUSTOM-RECURSION".to_owned()),
            "{codes:?}"
        );
    }
}

#[test]
fn legacy_field_id_surface_is_not_a_current_schema_shape() {
    let error = parse_yaml_document(
        PathBuf::from("legacy.yaml"),
        "kind: schema\ntable: item\nfields:\n  - id: 0\n    name: id\n    type: int\n",
    )
    .expect_err("legacy id must be rejected");
    assert_eq!(error.diagnostic().code, "E-YAML-SHAPE");
}
