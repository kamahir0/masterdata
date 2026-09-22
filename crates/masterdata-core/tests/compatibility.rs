use std::path::PathBuf;

use masterdata_core::{
    ArtifactBinaryImpact, CompatibilityChangeKind, CompatibilitySnapshot, GeneratedApiImpact,
    ProjectDocuments, ProjectMetadata, SourceMigrationImpact, compare_compatibility,
    parse_yaml_document,
};

fn documents(sources: &[(&str, &str)]) -> ProjectDocuments {
    ProjectDocuments {
        files: sources
            .iter()
            .map(|(path, source)| parse_yaml_document(PathBuf::from(path), source).expect("YAML"))
            .collect(),
    }
}

fn snapshot(id: &str, sources: &[(&str, &str)]) -> CompatibilitySnapshot {
    CompatibilitySnapshot::new(
        ProjectMetadata {
            id: id.to_owned(),
            name: "Compatibility Fixture".to_owned(),
            version: "not-a-semver-policy-value".to_owned(),
        },
        documents(sources),
    )
}

const ITEM_SCHEMA: &str = "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: name\n    type: string\nprimaryKey:\n  fields: [id]\n";

#[test]
fn unchanged_snapshot_ignores_formatting_paths_file_split_and_record_order() {
    let baseline = snapshot(
        "compat.test",
        &[
            ("sources/item.yaml", ITEM_SCHEMA),
            (
                "sources/data.yaml",
                "kind: data\ntable: item\nrecords:\n  - id: 1\n    name: Potion\n  - id: 2\n    name: Ether\n",
            ),
        ],
    );
    let current = snapshot(
        "compat.test",
        &[
            (
                "moved/schema/item.yaml",
                "# formatting and path are not semantic identity\nkind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: name\n    type: string\nprimaryKey:\n  fields: [id]\n",
            ),
            (
                "moved/data/second.yaml",
                "kind: data\ntable: item\nrecords:\n  - name: Potion\n    id: 1\n",
            ),
            (
                "moved/data/first.yaml",
                "kind: data\ntable: item\nrecords:\n  - name: Ether\n    id: 2\n",
            ),
        ],
    );

    let report = compare_compatibility(&baseline, &current).expect("comparable snapshots");
    assert!(report.changes.is_empty(), "unexpected changes: {report:#?}");
    assert_eq!(report.baseline.version, "not-a-semver-policy-value");
}

#[test]
fn authoring_only_computed_view_change_has_no_runtime_compatibility_impact() {
    let baseline = snapshot(
        "compat.test",
        &[
            ("schema.yaml", ITEM_SCHEMA),
            (
                "views/display.yaml",
                "kind: view\nname: display\ntable: item\ncolumns:\n  - name: label\n    expression: 'name + \"!\"'\n",
            ),
        ],
    );
    let current = snapshot(
        "compat.test",
        &[
            ("schema.yaml", ITEM_SCHEMA),
            (
                "renamed-path.yaml",
                "kind: view\nname: display\ntable: item\ncolumns:\n  - name: label\n    expression: 'name + \"?\"'\n  - name: upper\n    expression: 'name'\n",
            ),
        ],
    );

    let report = compare_compatibility(&baseline, &current).expect("comparable snapshots");
    assert!(
        report.changes.is_empty(),
        "Computed View authoring changes must not be reported as runtime API/binary changes: {report:#?}"
    );
}

#[test]
fn data_only_change_is_separate_and_requires_rebuild() {
    let baseline = snapshot(
        "compat.test",
        &[
            ("schema.yaml", ITEM_SCHEMA),
            (
                "data.yaml",
                "kind: data\ntable: item\nrecords:\n  - id: 1\n    name: Potion\n",
            ),
        ],
    );
    let current = snapshot(
        "compat.test",
        &[
            ("schema.yaml", ITEM_SCHEMA),
            (
                "data.yaml",
                "kind: data\ntable: item\nrecords:\n  - id: 1\n    name: Elixir\n",
            ),
        ],
    );

    let report = compare_compatibility(&baseline, &current).expect("comparable snapshots");
    assert_eq!(report.changes.len(), 1);
    assert_eq!(report.changes[0].kind, CompatibilityChangeKind::DataChanged);
    assert_eq!(
        report.changes[0].generated_api,
        GeneratedApiImpact::Unchanged
    );
    assert_eq!(
        report.changes[0].source_migration,
        SourceMigrationImpact::NotRequired
    );
    assert_eq!(
        report.changes[0].artifact_binary,
        ArtifactBinaryImpact::RebuildRequired
    );
}

#[test]
fn messagepack_key_and_secondary_declaration_reorder_are_not_identity_changes() {
    let baseline = snapshot(
        "compat.test",
        &[(
            "schema.yaml",
            "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: code\n    type: string\n  - key: 2\n    name: label\n    type: string\nprimaryKey:\n  fields: [id]\nsecondaryKeys:\n  - fields: [code]\n  - fields: [label]\n",
        )],
    );
    let current = snapshot(
        "compat.test",
        &[(
            "schema.yaml",
            "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 9\n    name: code\n    type: string\n  - key: 2\n    name: label\n    type: string\nprimaryKey:\n  fields: [id]\nsecondaryKeys:\n  - fields: [label]\n  - fields: [code]\n",
        )],
    );

    let report = compare_compatibility(&baseline, &current).expect("comparable snapshots");
    let kinds = report
        .changes
        .iter()
        .map(|change| change.kind)
        .collect::<Vec<_>>();
    assert!(kinds.contains(&CompatibilityChangeKind::FieldMessagePackKeyChanged));
    assert!(kinds.contains(&CompatibilityChangeKind::SecondaryKeyDeclarationReordered));
    assert!(!kinds.contains(&CompatibilityChangeKind::SecondaryKeyAdded));
    assert!(!kinds.contains(&CompatibilityChangeKind::SecondaryKeyRemoved));
    assert!(report.changes.iter().any(|change| {
        change.kind == CompatibilityChangeKind::SecondaryKeyDeclarationReordered
            && change.reason.contains("indexNo")
    }));
}

#[test]
fn unmatched_field_and_table_are_reported_as_add_remove_without_rename_guess() {
    let baseline = snapshot(
        "compat.test",
        &[(
            "schema.yaml",
            "kind: schema\ntable: old-item\nfields:\n  - key: 0\n    name: oldName\n    type: int\nprimaryKey:\n  fields: [oldName]\n",
        )],
    );
    let current = snapshot(
        "compat.test",
        &[(
            "schema.yaml",
            "kind: schema\ntable: new-item\nfields:\n  - key: 0\n    name: newName\n    type: int\nprimaryKey:\n  fields: [newName]\n",
        )],
    );

    let report = compare_compatibility(&baseline, &current).expect("comparable snapshots");
    assert!(
        report
            .changes
            .iter()
            .any(|change| change.kind == CompatibilityChangeKind::TableRemoved)
    );
    assert!(
        report
            .changes
            .iter()
            .any(|change| change.kind == CompatibilityChangeKind::TableAdded)
    );

    let old_field_schema = ITEM_SCHEMA.replace("name: name", "name: oldName");
    let new_field_schema = ITEM_SCHEMA.replace("name: name", "name: newName");
    let field_baseline = snapshot("compat.test", &[("schema.yaml", &old_field_schema)]);
    let field_current = snapshot("compat.test", &[("schema.yaml", &new_field_schema)]);
    let field_report =
        compare_compatibility(&field_baseline, &field_current).expect("comparable field snapshots");
    assert!(
        field_report
            .changes
            .iter()
            .any(|change| change.kind == CompatibilityChangeKind::FieldRemoved)
    );
    assert!(
        field_report
            .changes
            .iter()
            .any(|change| change.kind == CompatibilityChangeKind::FieldAdded)
    );
    assert!(field_report.changes.iter().all(|change| {
        matches!(
            change.kind,
            CompatibilityChangeKind::FieldAdded | CompatibilityChangeKind::FieldRemoved
        )
    }));
}

#[test]
fn project_mismatch_and_invalid_snapshot_are_structured_input_failures() {
    let valid = snapshot("baseline.id", &[("schema.yaml", ITEM_SCHEMA)]);
    let other_project = snapshot("current.id", &[("schema.yaml", ITEM_SCHEMA)]);
    let error = compare_compatibility(&valid, &other_project).expect_err("project mismatch");
    assert_eq!(error.diagnostic().code, "E-COMPAT-PROJECT-MISMATCH");

    let invalid = snapshot(
        "baseline.id",
        &[(
            "invalid.yaml",
            "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\n",
        )],
    );
    let error = compare_compatibility(&invalid, &valid).expect_err("invalid baseline");
    assert_eq!(error.diagnostic().code, "E-COMPAT-BASELINE-INVALID");
    assert!(
        error
            .diagnostic()
            .message
            .contains("canonical semantic snapshot")
    );
}

#[test]
fn type_and_reference_presentation_changes_are_deterministic_and_axis_specific() {
    let baseline = snapshot(
        "compat.test",
        &[
            (
                "types.yaml",
                "kind: type\nname: Rarity\nenum:\n  underlying: int\n  members:\n    - name: Common\n      value: 1\n",
            ),
            (
                "category.yaml",
                "kind: schema\ntable: category\nfields:\n  - key: 0\n    name: id\n    type: int\nprimaryKey:\n  fields: [id]\n",
            ),
            (
                "item.yaml",
                "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: categoryId\n    type: int\nprimaryKey:\n  fields: [id]\nreferences:\n  - name: category\n    fields: [categoryId]\n    target:\n      table: category\n      fields: [id]\n",
            ),
        ],
    );
    let current = snapshot(
        "compat.test",
        &[
            (
                "types.yaml",
                "kind: type\nname: Rarity\nenum:\n  underlying: int\n  members:\n    - name: Common\n      value: 1\n    - name: Rare\n      value: 2\n",
            ),
            (
                "category.yaml",
                "kind: schema\ntable: category\nfields:\n  - key: 0\n    name: id\n    type: int\nprimaryKey:\n  fields: [id]\n",
            ),
            (
                "item.yaml",
                "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: categoryId\n    type: int\nprimaryKey:\n  fields: [id]\nreferences:\n  - name: category\n    csharpName: CategoryMaster\n    fields: [categoryId]\n    target:\n      table: category\n      fields: [id]\n",
            ),
        ],
    );

    let first = compare_compatibility(&baseline, &current).expect("comparable snapshots");
    let second = compare_compatibility(&baseline, &current).expect("repeat comparison");
    assert_eq!(first, second);
    assert!(
        first
            .changes
            .iter()
            .any(|change| change.kind == CompatibilityChangeKind::EnumMemberAdded)
    );
    let reference = first
        .changes
        .iter()
        .find(|change| change.kind == CompatibilityChangeKind::ReferencePresentationChanged)
        .expect("reference presentation change");
    assert_eq!(reference.generated_api, GeneratedApiImpact::Breaking);
    assert_eq!(
        reference.source_migration,
        SourceMigrationImpact::NotRequired
    );
}

#[test]
fn table_and_key_evolution_reports_presentation_schema_and_query_impacts() {
    let baseline = snapshot(
        "compat.test",
        &[(
            "schema.yaml",
            "kind: schema\ntable: item\ncsharpName: Item\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: code\n    type: string\n  - key: 2\n    name: legacy\n    type: int\nprimaryKey:\n  fields: [id]\nsecondaryKeys:\n  - fields: [code]\n",
        )],
    );
    let current = snapshot(
        "compat.test",
        &[(
            "schema.yaml",
            "kind: schema\ntable: item\ncsharpName: ItemRow\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: code\n    type: string\n  - key: 3\n    name: added\n    type: int\nprimaryKey:\n  fields: [id, added]\nsecondaryKeys:\n  - fields: [code]\n    nonUnique: true\n",
        )],
    );

    let report = compare_compatibility(&baseline, &current).expect("comparable snapshots");
    assert!(
        report
            .changes
            .iter()
            .any(|change| change.kind == CompatibilityChangeKind::TablePresentationChanged)
    );
    assert!(
        report
            .changes
            .iter()
            .any(|change| change.kind == CompatibilityChangeKind::FieldAdded)
    );
    assert!(
        report
            .changes
            .iter()
            .any(|change| change.kind == CompatibilityChangeKind::FieldRemoved)
    );
    assert!(
        report
            .changes
            .iter()
            .any(|change| change.kind == CompatibilityChangeKind::PrimaryKeyChanged)
    );
    let uniqueness = report
        .changes
        .iter()
        .find(|change| change.kind == CompatibilityChangeKind::SecondaryKeyUniquenessChanged)
        .expect("secondary uniqueness change");
    assert_eq!(uniqueness.generated_api, GeneratedApiImpact::Breaking);
    assert_eq!(
        uniqueness.artifact_binary,
        ArtifactBinaryImpact::CrossSchemaInteroperabilityNotGuaranteed
    );
}

#[test]
fn reference_optionality_and_cardinality_are_compared_from_resolved_targets() {
    let baseline = snapshot(
        "compat.test",
        &[
            (
                "category.yaml",
                "kind: schema\ntable: category\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: code\n    type: string\nprimaryKey:\n  fields: [id]\nsecondaryKeys:\n  - fields: [code]\n",
            ),
            (
                "item.yaml",
                "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: categoryId\n    type: int\n  - key: 2\n    name: categoryCode\n    type: string\nprimaryKey:\n  fields: [id]\nreferences:\n  - name: category\n    fields: [categoryId]\n    target:\n      table: category\n      fields: [id]\n  - name: categoriesByCode\n    fields: [categoryCode]\n    target:\n      table: category\n      fields: [code]\n",
            ),
        ],
    );
    let current = snapshot(
        "compat.test",
        &[
            (
                "category.yaml",
                "kind: schema\ntable: category\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: code\n    type: string\nprimaryKey:\n  fields: [id]\nsecondaryKeys:\n  - fields: [code]\n    nonUnique: true\n",
            ),
            (
                "item.yaml",
                "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: categoryId\n    type: int\n    nullable: true\n  - key: 2\n    name: categoryCode\n    type: string\nprimaryKey:\n  fields: [id]\nreferences:\n  - name: category\n    fields: [categoryId]\n    target:\n      table: category\n      fields: [id]\n  - name: categoriesByCode\n    fields: [categoryCode]\n    target:\n      table: category\n      fields: [code]\n",
            ),
        ],
    );

    let report = compare_compatibility(&baseline, &current).expect("comparable snapshots");
    assert!(
        report
            .changes
            .iter()
            .any(|change| change.kind == CompatibilityChangeKind::ReferenceOptionalityChanged)
    );
    let cardinality = report
        .changes
        .iter()
        .find(|change| change.kind == CompatibilityChangeKind::ReferenceCardinalityChanged)
        .expect("reference cardinality change");
    assert_eq!(cardinality.generated_api, GeneratedApiImpact::Breaking);
    assert_eq!(
        cardinality.artifact_binary,
        ArtifactBinaryImpact::CrossSchemaInteroperabilityNotGuaranteed
    );
}

#[test]
fn custom_and_flags_type_changes_are_reported_without_shape_rename_inference() {
    let baseline = snapshot(
        "compat.test",
        &[
            (
                "reward.yaml",
                "kind: type\nname: Reward\ncustom:\n  fields:\n    - key: 0\n      name: amount\n      type: int\n",
            ),
            (
                "flags.yaml",
                "kind: type\nname: Feature\nflags:\n  underlying: int\n  members:\n    - name: None\n      value: 0\n",
            ),
        ],
    );
    let current = snapshot(
        "compat.test",
        &[
            (
                "reward.yaml",
                "kind: type\nname: Reward\ncustom:\n  fields:\n    - key: 0\n      name: amount\n      type: int\n    - key: 1\n      name: note\n      type: string\n",
            ),
            (
                "flags.yaml",
                "kind: type\nname: Feature\nflags:\n  underlying: int\n  members:\n    - name: None\n      value: 0\n    - name: Fire\n      value: 1\n",
            ),
        ],
    );

    let report = compare_compatibility(&baseline, &current).expect("comparable snapshots");
    assert!(
        report
            .changes
            .iter()
            .any(|change| change.kind == CompatibilityChangeKind::TypeFieldAdded)
    );
    assert!(
        report
            .changes
            .iter()
            .any(|change| change.kind == CompatibilityChangeKind::EnumMemberAdded)
    );
}
