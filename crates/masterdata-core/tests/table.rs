use std::path::PathBuf;

use masterdata_core::{
    BuildSelection, PrimitiveType, ProjectDocuments, ResolvedType, TypeReference,
    build_type_system, parse_yaml_document, resolve_tables,
};
use serde_yaml::Value;
use std::cmp::Ordering;

fn documents(sources: &[(&str, &str)]) -> ProjectDocuments {
    ProjectDocuments {
        files: sources
            .iter()
            .map(|(path, source)| parse_yaml_document(PathBuf::from(path), source).unwrap())
            .collect(),
    }
}

fn table_build(
    sources: &[(&str, &str)],
    selection: &BuildSelection,
) -> masterdata_core::TableBuild {
    let documents = documents(sources);
    let type_build = build_type_system(&documents);
    assert!(
        type_build.diagnostics.is_empty(),
        "unexpected type diagnostics: {:?}",
        type_build.diagnostics
    );
    let type_system = type_build.model.expect("valid type system");
    resolve_tables(&documents, &type_system, selection)
}

fn diagnostic_codes(build: &masterdata_core::TableBuild) -> Vec<&str> {
    build
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect()
}

const CATEGORY_TYPE: &str = "kind: type\nname: Category\nenum:\n  underlying: int\n  members:\n    - name: Weapon\n      value: 1\n    - name: Armor\n      value: 2\n";

const FEATURE_TYPE: &str = "kind: type\nname: Feature\nflags:\n  underlying: uint\n  members:\n    - name: None\n      value: 0\n    - name: Fire\n      value: 1\n    - name: Ice\n      value: 2\n";

const REWARD_TYPE: &str = "kind: type\nname: Reward\ncustom:\n  fields:\n    - key: 0\n      name: amount\n      type: int\n";

#[test]
fn resolves_logical_table_records_and_preserves_declared_orders() {
    let schema = "kind: schema\ntable: item\ncsharpName: ItemMaster\nfields:\n  - key: 3\n    name: name\n    type: string\n  - key: 2\n    name: rarity\n    type: Category\n  - key: 1\n    name: category\n    type: string\n  - key: 0\n    name: id\n    type: int\nprimaryKey:\n  fields: [id]\nsecondaryKeys:\n  - fields: [category]\n    nonUnique: true\n  - fields: [category, rarity]\n";
    let first_data = "kind: data\ntable: item\nrecords:\n  - id: 2\n    category: armor\n    rarity: Armor\n    name: Shield\n    $tags: [common]\n";
    let second_data = "kind: data\ntable: item\nrecords:\n  - id: 1\n    category: weapon\n    rarity: Weapon\n    name: Sword\n";

    let build = table_build(
        &[
            ("category.yaml", CATEGORY_TYPE),
            ("schema.yaml", schema),
            ("first.yaml", first_data),
            ("second.yaml", second_data),
        ],
        &BuildSelection::unfiltered(),
    );
    assert!(build.diagnostics.is_empty(), "{:#?}", build.diagnostics);
    let tables = build.model.expect("resolved tables");
    assert_eq!(tables.len(), 1);
    let table = &tables[0];
    assert_eq!(table.identity, "item");
    assert_eq!(table.csharp_name, "ItemMaster");
    assert_eq!(
        table
            .fields
            .iter()
            .map(|field| field.name.as_str())
            .collect::<Vec<_>>(),
        ["name", "rarity", "category", "id"]
    );
    assert_eq!(table.primary_key.fields, ["id"]);
    assert_eq!(
        table
            .secondary_keys
            .iter()
            .map(|key| (&key.fields, key.non_unique, key.index_no))
            .collect::<Vec<_>>(),
        [
            (&vec!["category".to_owned()], true, 0),
            (&vec!["category".to_owned(), "rarity".to_owned()], false, 1),
        ]
    );
    assert_eq!(table.records.len(), 2);
    assert_eq!(
        table.records[0].fields.get("id").and_then(Value::as_i64),
        Some(1)
    );
    assert_eq!(
        table.records[1].fields.get("id").and_then(Value::as_i64),
        Some(2)
    );
    assert!(!table.records[0].fields.contains_key("$tags"));
}

#[test]
fn selection_precedes_primary_key_uniqueness() {
    let schema = "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: name\n    type: string\nprimaryKey:\n  fields: [id]\n";
    let data = "kind: data\ntable: item\nrecords:\n  - id: 1\n    name: Production\n    $tags: [production]\n  - id: 1\n    name: Debug\n    $tags: [debug]\n";

    let production =
        BuildSelection::new(["production"], std::iter::empty::<&str>()).expect("selection");
    let selected = table_build(&[("schema.yaml", schema), ("data.yaml", data)], &production);
    assert!(
        selected.diagnostics.is_empty(),
        "{:#?}",
        selected.diagnostics
    );
    assert_eq!(selected.model.unwrap()[0].records.len(), 1);

    let all = table_build(
        &[("schema.yaml", schema), ("data.yaml", data)],
        &BuildSelection::unfiltered(),
    );
    assert!(diagnostic_codes(&all).contains(&"E-TABLE-DUPLICATE-PRIMARY-VALUE"));
}

#[test]
fn selection_can_produce_a_valid_empty_table() {
    let schema = "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\nprimaryKey:\n  fields: [id]\n";
    let data = "kind: data\ntable: item\nrecords:\n  - id: 1\n    $tags: [debug]\n";
    let production =
        BuildSelection::new(["production"], std::iter::empty::<&str>()).expect("selection");
    let build = table_build(&[("schema.yaml", schema), ("data.yaml", data)], &production);
    assert!(build.diagnostics.is_empty(), "{:#?}", build.diagnostics);
    let tables = build.model.expect("resolved tables");
    assert_eq!(tables.len(), 1);
    assert!(tables[0].records.is_empty());
}

#[test]
fn resolves_composite_keys_and_canonicalizes_lexicographically() {
    let schema = "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: region\n    type: string\n  - key: 1\n    name: id\n    type: int\n  - key: 2\n    name: category\n    type: string\n  - key: 3\n    name: rarity\n    type: int\nprimaryKey:\n  fields: [region, id]\nsecondaryKeys:\n  - fields: [category]\n    nonUnique: true\n  - fields: [category, rarity]\n";
    let data_a = "kind: data\ntable: item\nrecords:\n  - region: b\n    id: 2\n    category: armor\n    rarity: 2\n";
    let data_b = "kind: data\ntable: item\nrecords:\n  - region: a\n    id: 2\n    category: weapon\n    rarity: 2\n  - region: a\n    id: 1\n    category: weapon\n    rarity: 1\n";
    let build = table_build(
        &[
            ("schema.yaml", schema),
            ("data-a.yaml", data_a),
            ("data-b.yaml", data_b),
        ],
        &BuildSelection::unfiltered(),
    );
    assert!(build.diagnostics.is_empty(), "{:#?}", build.diagnostics);
    let records = &build.model.unwrap()[0].records;
    let keys = records
        .iter()
        .map(|record| {
            (
                record.fields["region"].as_str().unwrap(),
                record.fields["id"].as_i64().unwrap(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(keys, [("a", 1), ("a", 2), ("b", 2)]);
}

#[test]
fn validates_record_shape_using_the_existing_type_system() {
    let schema = "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: name\n    type: string\n  - key: 2\n    name: note\n    type: string\n    nullable: true\n  - key: 3\n    name: tags\n    type: string\n    array: true\n  - key: 4\n    name: feature\n    type: Feature\nprimaryKey:\n  fields: [id]\n";
    let valid_data = "kind: data\ntable: item\nrecords:\n  - id: 1\n    name: Potion\n    note: null\n    tags: []\n    feature: [None]\n    $tags: [common]\n";
    let valid = table_build(
        &[
            ("feature.yaml", FEATURE_TYPE),
            ("schema.yaml", schema),
            ("data.yaml", valid_data),
        ],
        &BuildSelection::unfiltered(),
    );
    assert!(valid.diagnostics.is_empty(), "{:#?}", valid.diagnostics);

    let missing = "kind: data\ntable: item\nrecords:\n  - id: 1\n    note: null\n    tags: []\n    feature: [None]\n";
    let missing_build = table_build(
        &[
            ("feature.yaml", FEATURE_TYPE),
            ("schema.yaml", schema),
            ("data.yaml", missing),
        ],
        &BuildSelection::unfiltered(),
    );
    assert!(diagnostic_codes(&missing_build).contains(&"E-TABLE-MISSING-REQUIRED-FIELD"));

    let unknown = "kind: data\ntable: item\nrecords:\n  - id: 1\n    name: Potion\n    note: null\n    tags: []\n    feature: [None]\n    extra: true\n";
    let unknown_build = table_build(
        &[
            ("feature.yaml", FEATURE_TYPE),
            ("schema.yaml", schema),
            ("data.yaml", unknown),
        ],
        &BuildSelection::unfiltered(),
    );
    assert!(diagnostic_codes(&unknown_build).contains(&"E-TABLE-UNKNOWN-RECORD-FIELD"));

    let invalid_flags = "kind: data\ntable: item\nrecords:\n  - id: 1\n    name: Potion\n    note: null\n    tags: []\n    feature: 3\n";
    let invalid_flags_build = table_build(
        &[
            ("feature.yaml", FEATURE_TYPE),
            ("schema.yaml", schema),
            ("data.yaml", invalid_flags),
        ],
        &BuildSelection::unfiltered(),
    );
    assert!(diagnostic_codes(&invalid_flags_build).contains(&"E-TABLE-INVALID-RECORD-VALUE"));
}

#[test]
fn table_data_accepts_canonical_integer_lexemes_and_rejects_invalid_forms() {
    for value in [
        "0",
        "-0",
        "1",
        "-1",
        "2147483647",
        "-9223372036854775808",
        "18446744073709551615",
    ] {
        let source = format!("kind: data\ntable: item\nrecords:\n  - id: {value}\n");
        parse_yaml_document(PathBuf::from("valid-data.yaml"), &source)
            .unwrap_or_else(|error| panic!("{value}: {error}"));
    }

    for value in [
        "+1", "01", "-01", "0x1", "0X1", "0o1", "0O1", "0b1", "0B1", "1_000",
    ] {
        let source = format!("kind: data\ntable: item\nrecords:\n  - id: {value}\n");
        let error = parse_yaml_document(PathBuf::from("invalid-data.yaml"), &source)
            .expect_err("invalid integer spelling");
        assert_eq!(error.diagnostic().code, "E-YAML-INVALID-INTEGER", "{value}");
        assert_eq!(error.diagnostic().related_requirements, ["YAML-SUBSET-011"]);
    }
}

#[test]
fn table_data_rejects_invalid_float_lexemes_before_normalization() {
    for value in [
        ".5",
        "1.",
        "+1.5",
        "1_000.0",
        "NaN",
        "Infinity",
        "+Infinity",
        "-Infinity",
    ] {
        let source = format!("kind: data\ntable: item\nrecords:\n  - value: {value}\n");
        let error = parse_yaml_document(PathBuf::from("invalid-float.yaml"), &source)
            .expect_err("invalid float spelling");
        assert_eq!(error.diagnostic().code, "E-YAML-INVALID-FLOAT", "{value}");
        assert_eq!(error.diagnostic().related_requirements, ["YAML-SUBSET-012"]);
    }
}

#[test]
fn table_data_preserves_string_and_block_scalar_false_positive_cases() {
    let schema = "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: enabled\n    type: bool\n  - key: 2\n    name: name\n    type: string\n  - key: 3\n    name: label\n    type: string\n  - key: 4\n    name: note\n    type: string\n    nullable: true\n  - key: 5\n    name: text\n    type: string\nprimaryKey:\n  fields: [id]\n";
    let valid_data = "kind: data\ntable: item\nrecords:\n  - id: 1 # 0x1\n    enabled: true\n    name: \"0x1 # True: ~\"\n    label: 'It''s 0x1 # True: ~'\n    note: null\n    text: |\n      0x1\n      True\n      ~\n";
    let valid = table_build(
        &[("schema.yaml", schema), ("data.yaml", valid_data)],
        &BuildSelection::unfiltered(),
    );
    assert!(valid.diagnostics.is_empty(), "{valid:#?}");

    for value in ["True", "TRUE", "False", "FALSE"] {
        let data = valid_data.replace("enabled: true", &format!("enabled: {value}"));
        let build = table_build(
            &[("schema.yaml", schema), ("data.yaml", &data)],
            &BuildSelection::unfiltered(),
        );
        assert!(
            diagnostic_codes(&build).contains(&"E-TABLE-INVALID-RECORD-VALUE"),
            "{value}: {build:#?}"
        );
    }

    for value in ["yes", "no", "on", "off"] {
        let data = valid_data.replace("name: \"0x1 # True: ~\"", &format!("name: {value}"));
        let build = table_build(
            &[("schema.yaml", schema), ("data.yaml", &data)],
            &BuildSelection::unfiltered(),
        );
        assert!(build.diagnostics.is_empty(), "{value}: {build:#?}");
    }
}

#[test]
fn table_data_rejects_yaml_null_shorthand_and_empty_mapping_values() {
    let null_shorthand = "kind: data\ntable: item\nrecords:\n  - note: ~\n";
    let error = parse_yaml_document(PathBuf::from("null-shorthand.yaml"), null_shorthand)
        .expect_err("YAML null shorthand");
    assert_eq!(error.diagnostic().code, "E-YAML-INVALID-NULL");
    assert_eq!(error.diagnostic().related_requirements, ["YAML-SUBSET-010"]);

    let missing_value = "kind: data\ntable: item\nrecords:\n  - note:\n";
    let error = parse_yaml_document(PathBuf::from("missing-value.yaml"), missing_value)
        .expect_err("implicit empty mapping value");
    assert_eq!(error.diagnostic().code, "E-YAML-MISSING-VALUE");
    assert_eq!(error.diagnostic().related_requirements, ["YAML-SUBSET-017"]);
}

#[test]
fn rejects_invalid_primary_and_secondary_key_shapes() {
    let missing_primary =
        "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\n";
    let build = table_build(
        &[("schema.yaml", missing_primary)],
        &BuildSelection::unfiltered(),
    );
    assert!(diagnostic_codes(&build).contains(&"E-TABLE-MISSING-PRIMARY-KEY"));

    let invalid_shapes = "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: optional\n    type: string\n    nullable: true\n  - key: 2\n    name: values\n    type: int\n    array: true\nprimaryKey:\n  fields: [optional, optional]\nsecondaryKeys:\n  - fields: []\n  - fields: [missing]\n  - fields: [values]\n  - fields: [id, id]\n  - fields: [id]\n  - fields: [id]\n";
    let build = table_build(
        &[("schema.yaml", invalid_shapes)],
        &BuildSelection::unfiltered(),
    );
    let codes = diagnostic_codes(&build);
    assert!(codes.contains(&"E-TABLE-DUPLICATE-PRIMARY-COMPONENT"));
    assert!(codes.contains(&"E-TABLE-EMPTY-SECONDARY-KEY"));
    assert!(codes.contains(&"E-TABLE-UNKNOWN-SECONDARY-FIELD"));
    assert!(codes.contains(&"E-TABLE-INVALID-SECONDARY-CAPABILITY"));
    assert!(codes.contains(&"E-TABLE-DUPLICATE-SECONDARY-SHAPE"));
    assert!(codes.contains(&"E-TABLE-INVALID-PRIMARY-CAPABILITY"));

    let same_as_primary = "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\nprimaryKey:\n  fields: [id]\nsecondaryKeys:\n  - fields: [id]\n";
    let same_shape = table_build(
        &[("schema.yaml", same_as_primary)],
        &BuildSelection::unfiltered(),
    );
    assert!(diagnostic_codes(&same_shape).contains(&"E-TABLE-SECONDARY-PRIMARY-SHAPE"));
}

#[test]
fn rejects_flags_and_custom_types_as_key_components() {
    let schema = "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: feature\n    type: Feature\n  - key: 2\n    name: reward\n    type: Reward\nprimaryKey:\n  fields: [feature]\nsecondaryKeys:\n  - fields: [reward]\n";
    let build = table_build(
        &[
            ("feature.yaml", FEATURE_TYPE),
            ("reward.yaml", REWARD_TYPE),
            ("schema.yaml", schema),
        ],
        &BuildSelection::unfiltered(),
    );
    let codes = diagnostic_codes(&build);
    assert!(codes.contains(&"E-TABLE-INVALID-PRIMARY-CAPABILITY"));
    assert!(codes.contains(&"E-TABLE-INVALID-SECONDARY-CAPABILITY"));
}

#[test]
fn rejects_generated_secondary_query_name_collisions() {
    let schema = "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: fooAndBar\n    type: int\n  - key: 2\n    name: foo\n    type: int\n  - key: 3\n    name: bar\n    type: int\nprimaryKey:\n  fields: [id]\nsecondaryKeys:\n  - fields: [fooAndBar]\n  - fields: [foo, bar]\n";
    let build = table_build(&[("schema.yaml", schema)], &BuildSelection::unfiltered());
    assert!(diagnostic_codes(&build).contains(&"E-TABLE-QUERY-NAME-COLLISION"));
}

#[test]
fn rejects_table_property_name_collisions_with_generated_type() {
    let implicit_name = "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: item\n    type: int\nprimaryKey:\n  fields: [item]\n";
    let implicit = table_build(
        &[("schema.yaml", implicit_name)],
        &BuildSelection::unfiltered(),
    );
    assert!(diagnostic_codes(&implicit).contains(&"E-TABLE-GENERATED-MEMBER-COLLISION"));
    assert!(implicit.model.is_none());

    let explicit_name = "kind: schema\ntable: item\ncsharpName: ItemData\nfields:\n  - key: 0\n    name: itemData\n    type: int\nprimaryKey:\n  fields: [itemData]\n";
    let explicit = table_build(
        &[("schema.yaml", explicit_name)],
        &BuildSelection::unfiltered(),
    );
    assert!(diagnostic_codes(&explicit).contains(&"E-TABLE-GENERATED-MEMBER-COLLISION"));
    assert!(explicit.model.is_none());
}

#[test]
fn rejects_non_unique_secondary_duplicates_but_allows_non_unique_duplicates() {
    let schema = "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: category\n    type: string\n  - key: 2\n    name: rarity\n    type: int\nprimaryKey:\n  fields: [id]\nsecondaryKeys:\n  - fields: [category]\n    nonUnique: true\n  - fields: [category, rarity]\n";
    let valid_data = "kind: data\ntable: item\nrecords:\n  - id: 1\n    category: weapon\n    rarity: 1\n  - id: 2\n    category: weapon\n    rarity: 2\n";
    let valid = table_build(
        &[("schema.yaml", schema), ("data.yaml", valid_data)],
        &BuildSelection::unfiltered(),
    );
    assert!(valid.diagnostics.is_empty(), "{:#?}", valid.diagnostics);

    let duplicate_unique = "kind: data\ntable: item\nrecords:\n  - id: 1\n    category: weapon\n    rarity: 1\n  - id: 2\n    category: weapon\n    rarity: 1\n";
    let duplicate = table_build(
        &[("schema.yaml", schema), ("data.yaml", duplicate_unique)],
        &BuildSelection::unfiltered(),
    );
    assert!(diagnostic_codes(&duplicate).contains(&"E-TABLE-DUPLICATE-UNIQUE-SECONDARY-VALUE"));
}

#[test]
fn type_system_remains_the_capability_owner_for_table_fields() {
    let documents = documents(&[("category.yaml", CATEGORY_TYPE)]);
    let type_build = build_type_system(&documents);
    assert!(type_build.diagnostics.is_empty());
    let type_system = type_build.model.expect("type system should resolve");
    assert!(matches!(
        type_system.get("Category"),
        Some(ResolvedType::Enum { .. })
    ));
    assert!(type_system.is_key_compatible(&type_system.resolve_reference("Category").unwrap()));
    assert!(type_system.is_comparison_capable(&type_system.resolve_reference("Category").unwrap()));
}

#[test]
fn canonical_order_uses_normal_enum_numeric_values() {
    let category = "kind: type\nname: Category\nenum:\n  underlying: int\n  members:\n    - name: Weapon\n      value: 10\n    - name: Armor\n      value: 1\n";
    let schema = "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: category\n    type: Category\nprimaryKey:\n  fields: [category]\n";
    let data = "kind: data\ntable: item\nrecords:\n  - category: Weapon\n  - category: Armor\n";
    let build = table_build(
        &[
            ("category.yaml", category),
            ("schema.yaml", schema),
            ("data.yaml", data),
        ],
        &BuildSelection::unfiltered(),
    );
    assert!(build.diagnostics.is_empty(), "{:#?}", build.diagnostics);
    let records = &build.model.expect("resolved tables")[0].records;
    let categories = records
        .iter()
        .map(|record| record.fields["category"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(categories, ["Armor", "Weapon"]);
}

#[test]
fn string_key_order_matches_ordinal_utf16_without_normalization() {
    let documents = documents(&[(
        "schema.yaml",
        "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: value\n    type: string\nprimaryKey:\n  fields: [value]\n",
    )]);
    let type_system = build_type_system(&documents).model.expect("type system");
    let bmp = Value::String("\u{e000}".to_owned());
    let supplementary = Value::String("\u{10000}".to_owned());
    assert_eq!(
        type_system
            .compare_reference_values(
                &TypeReference::Primitive(PrimitiveType::String),
                &bmp,
                &supplementary,
            )
            .expect("ordinal comparison"),
        Ordering::Greater
    );
}
