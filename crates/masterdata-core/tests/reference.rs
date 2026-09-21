use std::path::PathBuf;

use masterdata_core::{
    BuildSelection, ProjectDocuments, ReferenceCardinality, ReferenceKeyKind, ReferenceOptionality,
    SourceDocument, build_type_system, parse_yaml_document, resolve_tables,
};

fn documents(sources: &[(&str, &str)]) -> ProjectDocuments {
    ProjectDocuments {
        files: sources
            .iter()
            .map(|(path, source)| parse_yaml_document(PathBuf::from(path), source).unwrap())
            .collect(),
    }
}

fn build(sources: &[(&str, &str)], selection: &BuildSelection) -> masterdata_core::TableBuild {
    let documents = documents(sources);
    let type_system = build_type_system(&documents).model.expect("type system");
    resolve_tables(&documents, &type_system, selection)
}

fn codes(build: &masterdata_core::TableBuild) -> Vec<&str> {
    build
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect()
}

const TARGET: &str = "kind: schema\ntable: category\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: region\n    type: string\n  - key: 2\n    name: code\n    type: string\nprimaryKey:\n  fields: [id]\nsecondaryKeys:\n  - fields: [region, id]\n  - fields: [code]\n    nonUnique: true\n";

#[test]
fn resolves_primary_unique_and_non_unique_reference_cardinality() {
    let source = "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: categoryId\n    type: int\n  - key: 2\n    name: code\n    type: string\nprimaryKey:\n  fields: [id]\nreferences:\n  - name: category\n    fields: [categoryId]\n    target:\n      table: category\n      fields: [id]\n  - name: categoriesByCode\n    fields: [code]\n    target:\n      table: category\n      fields: [code]\n";
    let target_data = "kind: data\ntable: category\nrecords:\n  - id: 10\n    region: apac\n    code: potion\n  - id: 11\n    region: eu\n    code: potion\n";
    let source_data =
        "kind: data\ntable: item\nrecords:\n  - id: 1\n    categoryId: 10\n    code: potion\n";
    let result = build(
        &[
            ("target.yaml", TARGET),
            ("source.yaml", source),
            ("target-data.yaml", target_data),
            ("source-data.yaml", source_data),
        ],
        &BuildSelection::unfiltered(),
    );
    assert!(result.diagnostics.is_empty(), "{:#?}", result.diagnostics);
    let tables = result.model.expect("resolved tables");
    let item = tables
        .iter()
        .find(|table| table.identity == "item")
        .unwrap();
    assert_eq!(item.references.len(), 2);
    assert_eq!(
        item.references[0].target_key_kind,
        ReferenceKeyKind::Primary
    );
    assert_eq!(item.references[0].cardinality, ReferenceCardinality::Single);
    assert_eq!(
        item.references[1].target_key_kind,
        ReferenceKeyKind::Secondary
    );
    assert_eq!(item.references[1].cardinality, ReferenceCardinality::Many);
}

#[test]
fn resolves_composite_primary_and_secondary_reference_shapes() {
    let target = "kind: schema\ntable: composite-category\nfields:\n  - key: 0\n    name: region\n    type: string\n  - key: 1\n    name: id\n    type: int\n  - key: 2\n    name: code\n    type: string\nprimaryKey:\n  fields: [region, id]\nsecondaryKeys:\n  - fields: [code, id]\n  - fields: [region, code]\n    nonUnique: true\n";
    let source = "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: primaryRegion\n    type: string\n  - key: 2\n    name: primaryId\n    type: int\n  - key: 3\n    name: uniqueCode\n    type: string\n  - key: 4\n    name: uniqueId\n    type: int\n  - key: 5\n    name: manyRegion\n    type: string\n  - key: 6\n    name: manyCode\n    type: string\nprimaryKey:\n  fields: [id]\nreferences:\n  - name: compositePrimary\n    fields: [primaryRegion, primaryId]\n    target:\n      table: composite-category\n      fields: [region, id]\n  - name: compositeUnique\n    fields: [uniqueCode, uniqueId]\n    target:\n      table: composite-category\n      fields: [code, id]\n  - name: compositeMany\n    fields: [manyRegion, manyCode]\n    target:\n      table: composite-category\n      fields: [region, code]\n";
    let target_data = "kind: data\ntable: composite-category\nrecords:\n  - region: apac\n    id: 1\n    code: potion\n  - region: apac\n    id: 2\n    code: potion\n";
    let source_data = "kind: data\ntable: item\nrecords:\n  - id: 10\n    primaryRegion: apac\n    primaryId: 1\n    uniqueCode: potion\n    uniqueId: 2\n    manyRegion: apac\n    manyCode: potion\n";
    let result = build(
        &[
            ("target.yaml", target),
            ("source.yaml", source),
            ("target-data.yaml", target_data),
            ("source-data.yaml", source_data),
        ],
        &BuildSelection::unfiltered(),
    );
    assert!(result.diagnostics.is_empty(), "{:#?}", result.diagnostics);
    let item = result
        .model
        .unwrap()
        .into_iter()
        .find(|table| table.identity == "item")
        .unwrap();
    assert_eq!(item.references.len(), 3);
    assert_eq!(
        item.references[0].target_key_kind,
        ReferenceKeyKind::Primary
    );
    assert_eq!(item.references[0].cardinality, ReferenceCardinality::Single);
    assert_eq!(
        item.references[1].target_key_kind,
        ReferenceKeyKind::Secondary
    );
    assert_eq!(item.references[1].cardinality, ReferenceCardinality::Single);
    assert_eq!(
        item.references[2].target_key_kind,
        ReferenceKeyKind::Secondary
    );
    assert_eq!(item.references[2].cardinality, ReferenceCardinality::Many);
}

#[test]
fn required_reference_missing_target_is_build_blocking() {
    let source = "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: categoryId\n    type: int\nprimaryKey:\n  fields: [id]\nreferences:\n  - name: category\n    fields: [categoryId]\n    target:\n      table: category\n      fields: [id]\n";
    let data = "kind: data\ntable: item\nrecords:\n  - id: 1\n    categoryId: 404\n";
    let result = build(
        &[
            ("target.yaml", TARGET),
            ("source.yaml", source),
            ("source-data.yaml", data),
        ],
        &BuildSelection::unfiltered(),
    );
    assert!(codes(&result).contains(&"E-REFERENCE-MISSING-TARGET"));
    assert!(result.model.is_none());
}

#[test]
fn nullable_and_non_unique_missing_targets_remain_build_blocking() {
    let nullable_source = "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: categoryId\n    type: int\n    nullable: true\nprimaryKey:\n  fields: [id]\nreferences:\n  - name: category\n    fields: [categoryId]\n    target:\n      table: category\n      fields: [id]\n";
    let nullable_data = "kind: data\ntable: item\nrecords:\n  - id: 1\n    categoryId: 404\n";
    let target_data =
        "kind: data\ntable: category\nrecords:\n  - id: 10\n    region: apac\n    code: potion\n";
    let nullable = build(
        &[
            ("target.yaml", TARGET),
            ("source.yaml", nullable_source),
            ("target-data.yaml", target_data),
            ("source-data.yaml", nullable_data),
        ],
        &BuildSelection::unfiltered(),
    );
    assert!(codes(&nullable).contains(&"E-REFERENCE-MISSING-TARGET"));

    let non_unique_source = "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: code\n    type: string\nprimaryKey:\n  fields: [id]\nreferences:\n  - name: categories\n    fields: [code]\n    target:\n      table: category\n      fields: [code]\n";
    let non_unique_data = "kind: data\ntable: item\nrecords:\n  - id: 1\n    code: missing\n";
    let non_unique = build(
        &[
            ("target.yaml", TARGET),
            ("source.yaml", non_unique_source),
            ("target-data.yaml", target_data),
            ("source-data.yaml", non_unique_data),
        ],
        &BuildSelection::unfiltered(),
    );
    assert!(codes(&non_unique).contains(&"E-REFERENCE-MISSING-TARGET"));
}

#[test]
fn nullable_scalar_and_composite_follow_all_null_non_null_and_partial_null_rules() {
    let scalar_source = "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: categoryId\n    type: int\n    nullable: true\nprimaryKey:\n  fields: [id]\nreferences:\n  - name: category\n    fields: [categoryId]\n    target:\n      table: category\n      fields: [id]\n";
    let scalar_data = "kind: data\ntable: item\nrecords:\n  - id: 1\n    categoryId: null\n  - id: 2\n    categoryId: 10\n";
    let target_data =
        "kind: data\ntable: category\nrecords:\n  - id: 10\n    region: apac\n    code: potion\n";
    let scalar = build(
        &[
            ("target.yaml", TARGET),
            ("source.yaml", scalar_source),
            ("target-data.yaml", target_data),
            ("source-data.yaml", scalar_data),
        ],
        &BuildSelection::unfiltered(),
    );
    assert!(scalar.diagnostics.is_empty(), "{:#?}", scalar.diagnostics);
    let item = scalar
        .model
        .as_ref()
        .unwrap()
        .iter()
        .find(|table| table.identity == "item")
        .unwrap();
    assert_eq!(
        item.references[0].optionality,
        ReferenceOptionality::Nullable
    );

    let composite_source = "kind: schema\ntable: composite-item\nfields:\n  - key: 0\n    name: itemId\n    type: int\n  - key: 1\n    name: region\n    type: string\n    nullable: true\n  - key: 2\n    name: categoryId\n    type: int\n    nullable: true\nprimaryKey:\n  fields: [itemId]\nreferences:\n  - name: category\n    fields: [region, categoryId]\n    target:\n      table: category\n      fields: [region, id]\n";
    let composite_data = "kind: data\ntable: composite-item\nrecords:\n  - itemId: 1\n    region: null\n    categoryId: null\n  - itemId: 2\n    region: apac\n    categoryId: null\n";
    let composite = build(
        &[
            ("target.yaml", TARGET),
            ("source.yaml", composite_source),
            ("target-data.yaml", target_data),
            ("source-data.yaml", composite_data),
        ],
        &BuildSelection::unfiltered(),
    );
    assert!(codes(&composite).contains(&"E-REFERENCE-PARTIAL-NULL"));
    assert!(!codes(&composite).contains(&"E-REFERENCE-MISSING-TARGET"));
}

#[test]
fn mixed_nullable_and_array_components_are_rejected() {
    let source = "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: region\n    type: string\n  - key: 2\n    name: categoryId\n    type: int\n    nullable: true\n  - key: 3\n    name: tags\n    type: int\n    array: true\nprimaryKey:\n  fields: [id]\nreferences:\n  - name: mixed\n    fields: [region, categoryId]\n    target:\n      table: category\n      fields: [region, id]\n  - name: array\n    fields: [tags]\n    target:\n      table: category\n      fields: [id]\n";
    let result = build(
        &[("target.yaml", TARGET), ("source.yaml", source)],
        &BuildSelection::unfiltered(),
    );
    let codes = codes(&result);
    assert!(codes.contains(&"E-REFERENCE-MIXED-MODIFIERS"));
    assert!(codes.contains(&"E-REFERENCE-SOURCE-ARRAY"));
}

#[test]
fn reference_declaration_diagnostics_cover_identity_and_type_mismatches() {
    let source = "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: region\n    type: string\n  - key: 2\n    name: code\n    type: string\nprimaryKey:\n  fields: [id]\nreferences:\n  - name: duplicate\n    fields: [id]\n    target:\n      table: category\n      fields: [id]\n  - name: duplicate\n    fields: [id]\n    target:\n      table: category\n      fields: [id]\n  - name: duplicateSource\n    fields: [id, id]\n    target:\n      table: category\n      fields: [id]\n  - name: unknownSource\n    fields: [missing]\n    target:\n      table: category\n      fields: [id]\n  - name: unknownTarget\n    fields: [id]\n    target:\n      table: category\n      fields: [missing]\n  - name: reversed\n    fields: [region, id]\n    target:\n      table: category\n      fields: [id, region]\n  - name: wrongCount\n    fields: [id]\n    target:\n      table: category\n      fields: [region, id]\n  - name: wrongType\n    fields: [id]\n    target:\n      table: category\n      fields: [code]\n";
    let result = build(
        &[("target.yaml", TARGET), ("source.yaml", source)],
        &BuildSelection::unfiltered(),
    );
    let codes = codes(&result);
    for code in [
        "E-REFERENCE-DUPLICATE-NAME",
        "E-REFERENCE-DUPLICATE-SOURCE-FIELD",
        "E-REFERENCE-UNKNOWN-SOURCE-FIELD",
        "E-REFERENCE-UNKNOWN-TARGET-FIELD",
        "E-REFERENCE-TARGET-NOT-KEY",
        "E-REFERENCE-COMPONENT-COUNT",
        "E-REFERENCE-TYPE-INCOMPATIBLE",
    ] {
        assert!(codes.contains(&code), "missing {code}: {codes:?}");
    }
}

#[test]
fn reference_validation_uses_selected_source_and_target_datasets() {
    let source = "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: categoryId\n    type: int\nprimaryKey:\n  fields: [id]\nreferences:\n  - name: category\n    fields: [categoryId]\n    target:\n      table: category\n      fields: [id]\n";
    let target = "kind: schema\ntable: category\nfields:\n  - key: 0\n    name: id\n    type: int\nprimaryKey:\n  fields: [id]\n";
    let data =
        "kind: data\ntable: item\nrecords:\n  - id: 1\n    categoryId: 404\n    $tags: [debug]\n";
    let production = BuildSelection::new(["production"], std::iter::empty::<&str>()).unwrap();
    let source_excluded = build(
        &[
            ("target.yaml", target),
            ("source.yaml", source),
            ("data.yaml", data),
        ],
        &production,
    );
    assert!(
        source_excluded.diagnostics.is_empty(),
        "{:#?}",
        source_excluded.diagnostics
    );

    let target_data = "kind: data\ntable: category\nrecords:\n  - id: 10\n    $tags: [debug]\n";
    let both_data = "kind: data\ntable: item\nrecords:\n  - id: 2\n    categoryId: 10\n    $tags: [production]\n";
    let target_excluded = build(
        &[
            ("target.yaml", target),
            ("source.yaml", source),
            ("source-data.yaml", both_data),
            ("target-data.yaml", target_data),
        ],
        &production,
    );
    assert!(codes(&target_excluded).contains(&"E-REFERENCE-MISSING-TARGET"));
}

#[test]
fn reference_ast_is_typed_and_unknown_members_are_rejected() {
    let loaded = parse_yaml_document(
        PathBuf::from("schema.yaml"),
        "kind: schema\ntable: item\nreferences:\n  - name: category\n    fields: [categoryId]\n    target:\n      table: category\n      fields: [id]\n",
    )
    .unwrap();
    match loaded.document {
        SourceDocument::Schema(schema) => {
            assert_eq!(schema.references[0].target.fields, ["id"]);
        }
        _ => panic!("expected schema"),
    }
    let unknown = parse_yaml_document(
        PathBuf::from("unknown.yaml"),
        "kind: schema\ntable: item\nreferences:\n  - name: category\n    fields: [categoryId]\n    target:\n      table: category\n      fields: [id]\n    extra: true\n",
    )
    .expect_err("unknown Reference member must fail typed parsing");
    assert_eq!(unknown.diagnostic().code, "E-YAML-SHAPE");
}
