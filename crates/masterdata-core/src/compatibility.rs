//! Released Compatibility v1 semantic comparison.
//!
//! This module compares two caller-materialized canonical source snapshots.
//! It deliberately stops at the shared parser / Type System / Table boundary:
//! artifacts, receipts, Git state, and source mutation are not inputs to the
//! compatibility decision.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::config::ProjectMetadata;
use crate::document::{ProjectDocuments, SourceDocument};
use crate::error::{Diagnostic, ErrorKind, MasterdataError, Result};
use crate::table::{
    ResolvedReference, ResolvedTable, generated_query_name, reference_csharp_name, resolve_tables,
};
use crate::type_system::{ResolvedField, ResolvedType, build_type_system};

/// A caller-materialized canonical source snapshot.  The analyzer does not
/// discover or reload the filesystem after this value is constructed.
#[derive(Debug, Clone, PartialEq)]
pub struct CompatibilitySnapshot {
    pub project: ProjectMetadata,
    pub documents: ProjectDocuments,
}

impl CompatibilitySnapshot {
    pub fn new(project: ProjectMetadata, documents: ProjectDocuments) -> Self {
        Self { project, documents }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CompatibilityProjectSnapshot {
    pub project_id: String,
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum GeneratedApiImpact {
    Unchanged,
    Additive,
    Breaking,
    ReviewRequired,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum SourceMigrationImpact {
    NotRequired,
    SupportedOperation,
    DestructiveAuthorizationRequired,
    ManualActionRequired,
    ReviewRequired,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactBinaryImpact {
    Unchanged,
    RebuildRequired,
    CrossSchemaInteroperabilityNotGuaranteed,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ExternalContractImpact {
    NotAssessed,
    ExternalPolicyRequired,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum CompatibilitySubjectKind {
    Type,
    TypeField,
    Table,
    Field,
    PrimaryKey,
    SecondaryKey,
    Reference,
    Data,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CompatibilitySubject {
    pub kind: CompatibilitySubjectKind,
    pub owner: String,
    pub member: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum CompatibilityChangeKind {
    TypeAdded,
    TypeRemoved,
    TypeShapeChanged,
    TypeUnderlyingChanged,
    TypeConversionChanged,
    TypeFieldAdded,
    TypeFieldRemoved,
    TypeFieldShapeChanged,
    TypeFieldMessagePackKeyChanged,
    TypeFieldDeclarationReordered,
    EnumMemberAdded,
    EnumMemberRemoved,
    EnumMemberValueChanged,
    TableAdded,
    TableRemoved,
    TablePresentationChanged,
    FieldAdded,
    FieldRemoved,
    FieldTypeChanged,
    FieldModifierChanged,
    FieldMessagePackKeyChanged,
    FieldDeclarationReordered,
    PrimaryKeyChanged,
    SecondaryKeyAdded,
    SecondaryKeyRemoved,
    SecondaryKeyUniquenessChanged,
    SecondaryKeyDeclarationReordered,
    ReferenceAdded,
    ReferenceRemoved,
    ReferencePresentationChanged,
    ReferenceSourceChanged,
    ReferenceTargetChanged,
    ReferenceOptionalityChanged,
    ReferenceCardinalityChanged,
    DataChanged,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CompatibilityChange {
    pub subject: CompatibilitySubject,
    pub kind: CompatibilityChangeKind,
    pub baseline_locator: Option<String>,
    pub current_locator: Option<String>,
    pub generated_api: GeneratedApiImpact,
    pub source_migration: SourceMigrationImpact,
    pub source_operation: Option<String>,
    pub artifact_binary: ArtifactBinaryImpact,
    pub external_contract: ExternalContractImpact,
    pub reason: String,
    pub related_requirements: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CompatibilityClassificationCount {
    pub classification: String,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CompatibilitySummary {
    pub change_count: usize,
    pub generated_api: Vec<CompatibilityClassificationCount>,
    pub source_migration: Vec<CompatibilityClassificationCount>,
    pub artifact_binary: Vec<CompatibilityClassificationCount>,
    pub external_contract: Vec<CompatibilityClassificationCount>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CompatibilityReport {
    pub baseline: CompatibilityProjectSnapshot,
    pub current: CompatibilityProjectSnapshot,
    pub changes: Vec<CompatibilityChange>,
    pub summary: CompatibilitySummary,
}

#[derive(Debug, Clone)]
struct ResolvedCompatibilitySnapshot {
    project: ProjectMetadata,
    types: BTreeMap<String, ResolvedType>,
    tables: BTreeMap<String, ResolvedTable>,
    data: BTreeMap<String, Vec<String>>,
}

/// Compare two explicit snapshots using shared Rust semantics.
pub fn compare_compatibility(
    baseline: &CompatibilitySnapshot,
    current: &CompatibilitySnapshot,
) -> Result<CompatibilityReport> {
    if baseline.project.id != current.project.id {
        return Err(MasterdataError::new(
            "E-COMPAT-PROJECT-MISMATCH",
            ErrorKind::Validation,
            format!(
                "baseline project.id `{}` does not match current project.id `{}`",
                baseline.project.id, current.project.id
            ),
        )
        .with_related_requirement("COMPAT-RELEASED-002"));
    }

    let baseline = resolve_snapshot(baseline, "baseline")?;
    let current = resolve_snapshot(current, "current")?;
    let mut changes = Vec::new();

    compare_types(&baseline, &current, &mut changes);
    compare_tables(&baseline, &current, &mut changes);
    compare_data(&baseline, &current, &mut changes);

    changes.sort_by(|left, right| {
        left.subject
            .kind
            .cmp(&right.subject.kind)
            .then_with(|| left.subject.owner.cmp(&right.subject.owner))
            .then_with(|| left.subject.member.cmp(&right.subject.member))
            .then_with(|| left.kind.cmp(&right.kind))
    });

    let baseline_metadata = project_snapshot_metadata(&baseline.project);
    let current_metadata = project_snapshot_metadata(&current.project);
    let summary = summarize(&changes);
    Ok(CompatibilityReport {
        baseline: baseline_metadata,
        current: current_metadata,
        changes,
        summary,
    })
}

/// Compatibility name used by application adapters that treat the operation
/// as an analyzer rather than a raw diff.
pub fn analyze_compatibility(
    baseline: &CompatibilitySnapshot,
    current: &CompatibilitySnapshot,
) -> Result<CompatibilityReport> {
    compare_compatibility(baseline, current)
}

fn resolve_snapshot(
    snapshot: &CompatibilitySnapshot,
    role: &str,
) -> Result<ResolvedCompatibilitySnapshot> {
    let type_build = build_type_system(&snapshot.documents);
    let type_system = match type_build.model {
        Some(model) if type_build.diagnostics.is_empty() => model,
        _ => return Err(snapshot_input_error(role, type_build.diagnostics)),
    };

    // Schema comparison does not require selected record constraints.  Keep
    // only schema/type documents for the Table resolver so a missing target
    // record cannot turn an otherwise comparable schema into an arbitrary
    // binary-build validation requirement.
    let structural_documents = ProjectDocuments {
        files: snapshot
            .documents
            .files
            .iter()
            .filter(|loaded| {
                matches!(
                    loaded.document,
                    SourceDocument::Schema(_) | SourceDocument::Type(_)
                )
            })
            .cloned()
            .collect(),
    };
    let table_build = resolve_tables(
        &structural_documents,
        &type_system,
        &crate::table::BuildSelection::unfiltered(),
    );
    let tables = match table_build.model {
        Some(model) if table_build.diagnostics.is_empty() => model,
        _ => return Err(snapshot_input_error(role, table_build.diagnostics)),
    };
    let tables = tables
        .into_iter()
        .map(|table| (table.identity.clone(), table))
        .collect::<BTreeMap<_, _>>();
    let table_names = tables.keys().cloned().collect::<BTreeSet<_>>();

    let mut data = BTreeMap::<String, Vec<String>>::new();
    for (_, document) in snapshot.documents.data() {
        if !table_names.contains(&document.table) {
            return Err(snapshot_input_error(
                role,
                vec![Diagnostic::new(
                    "E-COMPAT-UNKNOWN-DATA-TABLE",
                    ErrorKind::Validation,
                    format!("data document refers to unknown table `{}`", document.table),
                )],
            ));
        }
        let records = data.entry(document.table.clone()).or_default();
        for record in &document.records {
            let serialized = serde_json::to_string(record).map_err(|error| {
                snapshot_input_error(
                    role,
                    vec![Diagnostic::new(
                        "E-COMPAT-DATA-CANONICALIZATION",
                        ErrorKind::Validation,
                        format!("could not canonicalize data record: {error}"),
                    )],
                )
            })?;
            records.push(serialized);
        }
    }
    for records in data.values_mut() {
        records.sort();
    }

    let types = type_system.types.clone();
    Ok(ResolvedCompatibilitySnapshot {
        project: snapshot.project.clone(),
        types,
        tables,
        data,
    })
}

fn snapshot_input_error(role: &str, diagnostics: Vec<Diagnostic>) -> MasterdataError {
    let mut diagnostic = diagnostics.into_iter().next().unwrap_or_else(|| {
        Diagnostic::new(
            "E-COMPAT-SNAPSHOT-INVALID",
            ErrorKind::Validation,
            "snapshot could not be resolved",
        )
    });
    let original_code = diagnostic.code.clone();
    diagnostic.code = format!("E-COMPAT-{}-INVALID", role.to_ascii_uppercase());
    diagnostic.message = format!(
        "{role} snapshot is not a comparable canonical semantic snapshot ({original_code}): {}",
        diagnostic.message
    );
    diagnostic = diagnostic.with_related_requirement("COMPAT-RELEASED-002");
    MasterdataError {
        diagnostic: Box::new(diagnostic),
    }
}

fn project_snapshot_metadata(project: &ProjectMetadata) -> CompatibilityProjectSnapshot {
    CompatibilityProjectSnapshot {
        project_id: project.id.clone(),
        name: project.name.clone(),
        version: project.version.clone(),
    }
}

fn compare_types(
    baseline: &ResolvedCompatibilitySnapshot,
    current: &ResolvedCompatibilitySnapshot,
    changes: &mut Vec<CompatibilityChange>,
) {
    let names = baseline
        .types
        .keys()
        .chain(current.types.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    for name in names {
        match (baseline.types.get(&name), current.types.get(&name)) {
            (None, Some(_)) => push_change(
                changes,
                subject(CompatibilitySubjectKind::Type, name.clone(), None),
                CompatibilityChangeKind::TypeAdded,
                None,
                Some(locator("type", &name, None)),
                GeneratedApiImpact::Additive,
                SourceMigrationImpact::ManualActionRequired,
                None,
                ArtifactBinaryImpact::CrossSchemaInteroperabilityNotGuaranteed,
                ExternalContractImpact::NotAssessed,
                "Type declaration was added.",
                &[
                    "COMPAT-RELEASED-004",
                    "COMPAT-RELEASED-007",
                    "COMPAT-RELEASED-008",
                ],
            ),
            (Some(_), None) => push_change(
                changes,
                subject(CompatibilitySubjectKind::Type, name.clone(), None),
                CompatibilityChangeKind::TypeRemoved,
                Some(locator("type", &name, None)),
                None,
                GeneratedApiImpact::Breaking,
                SourceMigrationImpact::ManualActionRequired,
                None,
                ArtifactBinaryImpact::CrossSchemaInteroperabilityNotGuaranteed,
                ExternalContractImpact::NotAssessed,
                "Type declaration was removed; no rename lineage is inferred.",
                &[
                    "COMPAT-RELEASED-004",
                    "COMPAT-RELEASED-007",
                    "COMPAT-RELEASED-009",
                ],
            ),
            (Some(left), Some(right)) => compare_type(name, left, right, changes),
            (None, None) => unreachable!(),
        }
    }
}

fn compare_type(
    name: String,
    baseline: &ResolvedType,
    current: &ResolvedType,
    changes: &mut Vec<CompatibilityChange>,
) {
    match (baseline, current) {
        (
            ResolvedType::ValueObject {
                underlying: left_underlying,
                conversions: left_conversions,
                ..
            },
            ResolvedType::ValueObject {
                underlying: right_underlying,
                conversions: right_conversions,
                ..
            },
        ) => {
            if left_underlying != right_underlying {
                push_change(
                    changes,
                    subject(CompatibilitySubjectKind::Type, name.clone(), None),
                    CompatibilityChangeKind::TypeUnderlyingChanged,
                    Some(locator("type", &name, None)),
                    Some(locator("type", &name, None)),
                    GeneratedApiImpact::Breaking,
                    SourceMigrationImpact::ManualActionRequired,
                    None,
                    ArtifactBinaryImpact::CrossSchemaInteroperabilityNotGuaranteed,
                    ExternalContractImpact::ExternalPolicyRequired,
                    "Value Object underlying primitive changed.",
                    &[
                        "COMPAT-RELEASED-007",
                        "COMPAT-RELEASED-009",
                        "COMPAT-RELEASED-010",
                    ],
                );
            }
            if left_conversions != right_conversions {
                let impact = if (!left_conversions.from_underlying_implicit
                    && right_conversions.from_underlying_implicit)
                    || (!left_conversions.to_underlying_implicit
                        && right_conversions.to_underlying_implicit)
                {
                    GeneratedApiImpact::Additive
                } else {
                    GeneratedApiImpact::Breaking
                };
                push_change(
                    changes,
                    subject(CompatibilitySubjectKind::Type, name.clone(), None),
                    CompatibilityChangeKind::TypeConversionChanged,
                    Some(locator("type", &name, None)),
                    Some(locator("type", &name, None)),
                    impact,
                    SourceMigrationImpact::SupportedOperation,
                    Some("SetValueObjectConversions"),
                    ArtifactBinaryImpact::RebuildRequired,
                    ExternalContractImpact::NotAssessed,
                    "Value Object implicit conversion surface changed.",
                    &["COMPAT-RELEASED-007", "COMPAT-RELEASED-008"],
                );
            }
        }
        (ResolvedType::Custom { fields: left, .. }, ResolvedType::Custom { fields: right, .. }) => {
            compare_type_fields(&name, left, right, changes)
        }
        (
            ResolvedType::Enum {
                underlying: left_underlying,
                members: left_members,
                ..
            },
            ResolvedType::Enum {
                underlying: right_underlying,
                members: right_members,
                ..
            },
        )
        | (
            ResolvedType::Flags {
                underlying: left_underlying,
                members: left_members,
                ..
            },
            ResolvedType::Flags {
                underlying: right_underlying,
                members: right_members,
                ..
            },
        ) => {
            if left_underlying != right_underlying {
                push_change(
                    changes,
                    subject(CompatibilitySubjectKind::Type, name.clone(), None),
                    CompatibilityChangeKind::TypeUnderlyingChanged,
                    Some(locator("type", &name, None)),
                    Some(locator("type", &name, None)),
                    GeneratedApiImpact::Breaking,
                    SourceMigrationImpact::ManualActionRequired,
                    None,
                    ArtifactBinaryImpact::CrossSchemaInteroperabilityNotGuaranteed,
                    ExternalContractImpact::ExternalPolicyRequired,
                    "Enum or Flags underlying primitive changed.",
                    &[
                        "COMPAT-RELEASED-007",
                        "COMPAT-RELEASED-009",
                        "COMPAT-RELEASED-010",
                    ],
                );
            }
            compare_enum_members(&name, left_members, right_members, changes);
        }
        _ => push_change(
            changes,
            subject(CompatibilitySubjectKind::Type, name.clone(), None),
            CompatibilityChangeKind::TypeShapeChanged,
            Some(locator("type", &name, None)),
            Some(locator("type", &name, None)),
            GeneratedApiImpact::Breaking,
            SourceMigrationImpact::ManualActionRequired,
            None,
            ArtifactBinaryImpact::CrossSchemaInteroperabilityNotGuaranteed,
            ExternalContractImpact::ExternalPolicyRequired,
            "Type category or generated type shape changed.",
            &[
                "COMPAT-RELEASED-007",
                "COMPAT-RELEASED-008",
                "COMPAT-RELEASED-009",
            ],
        ),
    }
}

fn compare_type_fields(
    type_name: &str,
    baseline: &[ResolvedField],
    current: &[ResolvedField],
    changes: &mut Vec<CompatibilityChange>,
) {
    let baseline_by_name = baseline
        .iter()
        .map(|field| (field.name.clone(), field))
        .collect::<BTreeMap<_, _>>();
    let current_by_name = current
        .iter()
        .map(|field| (field.name.clone(), field))
        .collect::<BTreeMap<_, _>>();
    let names = baseline_by_name
        .keys()
        .chain(current_by_name.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    for name in names {
        match (baseline_by_name.get(&name), current_by_name.get(&name)) {
            (None, Some(_)) => push_change(
                changes,
                subject(
                    CompatibilitySubjectKind::TypeField,
                    type_name.to_owned(),
                    Some(name.clone()),
                ),
                CompatibilityChangeKind::TypeFieldAdded,
                None,
                Some(locator("type-field", type_name, Some(&name))),
                GeneratedApiImpact::Breaking,
                SourceMigrationImpact::SupportedOperation,
                Some("AddCustomField"),
                ArtifactBinaryImpact::CrossSchemaInteroperabilityNotGuaranteed,
                ExternalContractImpact::NotAssessed,
                "Custom Type field was added; its public constructor shape changes.",
                &[
                    "COMPAT-RELEASED-004",
                    "COMPAT-RELEASED-007",
                    "COMPAT-RELEASED-008",
                ],
            ),
            (Some(_), None) => push_change(
                changes,
                subject(
                    CompatibilitySubjectKind::TypeField,
                    type_name.to_owned(),
                    Some(name.clone()),
                ),
                CompatibilityChangeKind::TypeFieldRemoved,
                Some(locator("type-field", type_name, Some(&name))),
                None,
                GeneratedApiImpact::Breaking,
                SourceMigrationImpact::DestructiveAuthorizationRequired,
                Some("DropCustomField"),
                ArtifactBinaryImpact::CrossSchemaInteroperabilityNotGuaranteed,
                ExternalContractImpact::ExternalPolicyRequired,
                "Custom Type field was removed; no rename lineage is inferred.",
                &[
                    "COMPAT-RELEASED-004",
                    "COMPAT-RELEASED-007",
                    "COMPAT-RELEASED-008",
                ],
            ),
            (Some(left), Some(right)) => {
                if left.base_type != right.base_type {
                    push_change(
                        changes,
                        subject(
                            CompatibilitySubjectKind::TypeField,
                            type_name.to_owned(),
                            Some(name.clone()),
                        ),
                        CompatibilityChangeKind::TypeFieldShapeChanged,
                        Some(locator("type-field", type_name, Some(&name))),
                        Some(locator("type-field", type_name, Some(&name))),
                        GeneratedApiImpact::Breaking,
                        SourceMigrationImpact::ManualActionRequired,
                        None,
                        ArtifactBinaryImpact::CrossSchemaInteroperabilityNotGuaranteed,
                        ExternalContractImpact::ExternalPolicyRequired,
                        "Custom Type field base type changed.",
                        &[
                            "COMPAT-RELEASED-007",
                            "COMPAT-RELEASED-008",
                            "COMPAT-RELEASED-009",
                        ],
                    );
                }
                if left.modifier != right.modifier {
                    push_change(
                        changes,
                        subject(
                            CompatibilitySubjectKind::TypeField,
                            type_name.to_owned(),
                            Some(name.clone()),
                        ),
                        CompatibilityChangeKind::TypeFieldShapeChanged,
                        Some(locator("type-field", type_name, Some(&name))),
                        Some(locator("type-field", type_name, Some(&name))),
                        GeneratedApiImpact::Breaking,
                        SourceMigrationImpact::ManualActionRequired,
                        None,
                        ArtifactBinaryImpact::CrossSchemaInteroperabilityNotGuaranteed,
                        ExternalContractImpact::ExternalPolicyRequired,
                        "Custom Type field modifier changed.",
                        &[
                            "COMPAT-RELEASED-007",
                            "COMPAT-RELEASED-008",
                            "COMPAT-RELEASED-009",
                        ],
                    );
                }
                if left.key != right.key {
                    push_change(
                        changes,
                        subject(
                            CompatibilitySubjectKind::TypeField,
                            type_name.to_owned(),
                            Some(name.clone()),
                        ),
                        CompatibilityChangeKind::TypeFieldMessagePackKeyChanged,
                        Some(locator("type-field", type_name, Some(&name))),
                        Some(locator("type-field", type_name, Some(&name))),
                        GeneratedApiImpact::Unchanged,
                        SourceMigrationImpact::NotRequired,
                        None,
                        ArtifactBinaryImpact::CrossSchemaInteroperabilityNotGuaranteed,
                        ExternalContractImpact::ExternalPolicyRequired,
                        "Custom Type MessagePack key changed without changing logical field identity.",
                        &[
                            "COMPAT-RELEASED-004",
                            "COMPAT-RELEASED-005",
                            "COMPAT-RELEASED-009",
                        ],
                    );
                }
            }
            (None, None) => unreachable!(),
        }
    }
    if baseline.iter().map(|field| &field.name).collect::<Vec<_>>()
        != current.iter().map(|field| &field.name).collect::<Vec<_>>()
        && baseline.len() == current.len()
        && baseline
            .iter()
            .map(|field| &field.name)
            .collect::<BTreeSet<_>>()
            == current
                .iter()
                .map(|field| &field.name)
                .collect::<BTreeSet<_>>()
    {
        push_change(
            changes,
            subject(CompatibilitySubjectKind::Type, type_name.to_owned(), None),
            CompatibilityChangeKind::TypeFieldDeclarationReordered,
            Some(locator("type", type_name, None)),
            Some(locator("type", type_name, None)),
            GeneratedApiImpact::Breaking,
            SourceMigrationImpact::NotRequired,
            None,
            ArtifactBinaryImpact::RebuildRequired,
            ExternalContractImpact::NotAssessed,
            "Custom Type declaration order changed; public constructor parameter order follows it.",
            &["COMPAT-RELEASED-007", "COMPAT-RELEASED-009"],
        );
    }
}

fn compare_enum_members(
    type_name: &str,
    baseline: &[crate::ResolvedEnumMember],
    current: &[crate::ResolvedEnumMember],
    changes: &mut Vec<CompatibilityChange>,
) {
    let baseline_by_name = baseline
        .iter()
        .map(|member| (member.name.clone(), member.value))
        .collect::<BTreeMap<_, _>>();
    let current_by_name = current
        .iter()
        .map(|member| (member.name.clone(), member.value))
        .collect::<BTreeMap<_, _>>();
    let names = baseline_by_name
        .keys()
        .chain(current_by_name.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    for member in names {
        match (baseline_by_name.get(&member), current_by_name.get(&member)) {
            (None, Some(_)) => push_change(
                changes,
                subject(
                    CompatibilitySubjectKind::Type,
                    type_name.to_owned(),
                    Some(member.clone()),
                ),
                CompatibilityChangeKind::EnumMemberAdded,
                None,
                Some(locator("enum-member", type_name, Some(&member))),
                GeneratedApiImpact::Additive,
                SourceMigrationImpact::SupportedOperation,
                Some("AddEnumMember"),
                ArtifactBinaryImpact::CrossSchemaInteroperabilityNotGuaranteed,
                ExternalContractImpact::ExternalPolicyRequired,
                "Enum or Flags member was added.",
                &[
                    "COMPAT-RELEASED-004",
                    "COMPAT-RELEASED-007",
                    "COMPAT-RELEASED-010",
                ],
            ),
            (Some(_), None) => push_change(
                changes,
                subject(
                    CompatibilitySubjectKind::Type,
                    type_name.to_owned(),
                    Some(member.clone()),
                ),
                CompatibilityChangeKind::EnumMemberRemoved,
                Some(locator("enum-member", type_name, Some(&member))),
                None,
                GeneratedApiImpact::Breaking,
                SourceMigrationImpact::DestructiveAuthorizationRequired,
                Some("DropEnumMember"),
                ArtifactBinaryImpact::CrossSchemaInteroperabilityNotGuaranteed,
                ExternalContractImpact::ExternalPolicyRequired,
                "Enum or Flags member was removed; no rename is inferred.",
                &[
                    "COMPAT-RELEASED-004",
                    "COMPAT-RELEASED-007",
                    "COMPAT-RELEASED-010",
                ],
            ),
            (Some(left), Some(right)) if left != right => push_change(
                changes,
                subject(
                    CompatibilitySubjectKind::Type,
                    type_name.to_owned(),
                    Some(member.clone()),
                ),
                CompatibilityChangeKind::EnumMemberValueChanged,
                Some(locator("enum-member", type_name, Some(&member))),
                Some(locator("enum-member", type_name, Some(&member))),
                GeneratedApiImpact::Breaking,
                SourceMigrationImpact::ManualActionRequired,
                None,
                ArtifactBinaryImpact::CrossSchemaInteroperabilityNotGuaranteed,
                ExternalContractImpact::ExternalPolicyRequired,
                "Enum or Flags member numeric value changed.",
                &[
                    "COMPAT-RELEASED-007",
                    "COMPAT-RELEASED-009",
                    "COMPAT-RELEASED-010",
                ],
            ),
            _ => {}
        }
    }
}

fn compare_tables(
    baseline: &ResolvedCompatibilitySnapshot,
    current: &ResolvedCompatibilitySnapshot,
    changes: &mut Vec<CompatibilityChange>,
) {
    let names = baseline
        .tables
        .keys()
        .chain(current.tables.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    for name in names {
        match (baseline.tables.get(&name), current.tables.get(&name)) {
            (None, Some(_)) => push_change(
                changes,
                subject(CompatibilitySubjectKind::Table, name.clone(), None),
                CompatibilityChangeKind::TableAdded,
                None,
                Some(locator("table", &name, None)),
                GeneratedApiImpact::Additive,
                SourceMigrationImpact::ManualActionRequired,
                None,
                ArtifactBinaryImpact::CrossSchemaInteroperabilityNotGuaranteed,
                ExternalContractImpact::NotAssessed,
                "Table was added.",
                &[
                    "COMPAT-RELEASED-004",
                    "COMPAT-RELEASED-007",
                    "COMPAT-RELEASED-009",
                ],
            ),
            (Some(_), None) => push_change(
                changes,
                subject(CompatibilitySubjectKind::Table, name.clone(), None),
                CompatibilityChangeKind::TableRemoved,
                Some(locator("table", &name, None)),
                None,
                GeneratedApiImpact::Breaking,
                SourceMigrationImpact::ManualActionRequired,
                None,
                ArtifactBinaryImpact::CrossSchemaInteroperabilityNotGuaranteed,
                ExternalContractImpact::NotAssessed,
                "Table was removed; no rename lineage is inferred.",
                &[
                    "COMPAT-RELEASED-004",
                    "COMPAT-RELEASED-007",
                    "COMPAT-RELEASED-009",
                ],
            ),
            (Some(left), Some(right)) => compare_table(left, right, changes),
            (None, None) => unreachable!(),
        }
    }
}

fn compare_table(
    baseline: &ResolvedTable,
    current: &ResolvedTable,
    changes: &mut Vec<CompatibilityChange>,
) {
    let table = baseline.identity.clone();
    if baseline.csharp_name != current.csharp_name {
        push_change(
            changes,
            subject(CompatibilitySubjectKind::Table, table.clone(), None),
            CompatibilityChangeKind::TablePresentationChanged,
            Some(locator("table", &table, None)),
            Some(locator("table", &table, None)),
            GeneratedApiImpact::Breaking,
            SourceMigrationImpact::NotRequired,
            None,
            ArtifactBinaryImpact::RebuildRequired,
            ExternalContractImpact::NotAssessed,
            "Table generated C# type presentation changed.",
            &[
                "COMPAT-RELEASED-004",
                "COMPAT-RELEASED-007",
                "COMPAT-RELEASED-009",
            ],
        );
    }

    let baseline_fields = baseline
        .fields
        .iter()
        .map(|field| (field.name.clone(), field))
        .collect::<BTreeMap<_, _>>();
    let current_fields = current
        .fields
        .iter()
        .map(|field| (field.name.clone(), field))
        .collect::<BTreeMap<_, _>>();
    let field_names = baseline_fields
        .keys()
        .chain(current_fields.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    for name in field_names {
        match (baseline_fields.get(&name), current_fields.get(&name)) {
            (None, Some(_)) => push_change(
                changes,
                subject(
                    CompatibilitySubjectKind::Field,
                    table.clone(),
                    Some(name.clone()),
                ),
                CompatibilityChangeKind::FieldAdded,
                None,
                Some(locator("field", &table, Some(&name))),
                GeneratedApiImpact::Additive,
                SourceMigrationImpact::SupportedOperation,
                Some("AddField"),
                ArtifactBinaryImpact::CrossSchemaInteroperabilityNotGuaranteed,
                ExternalContractImpact::NotAssessed,
                "Table field was added.",
                &[
                    "COMPAT-RELEASED-004",
                    "COMPAT-RELEASED-007",
                    "COMPAT-RELEASED-008",
                ],
            ),
            (Some(_), None) => push_change(
                changes,
                subject(
                    CompatibilitySubjectKind::Field,
                    table.clone(),
                    Some(name.clone()),
                ),
                CompatibilityChangeKind::FieldRemoved,
                Some(locator("field", &table, Some(&name))),
                None,
                GeneratedApiImpact::Breaking,
                SourceMigrationImpact::DestructiveAuthorizationRequired,
                Some("DropField"),
                ArtifactBinaryImpact::CrossSchemaInteroperabilityNotGuaranteed,
                ExternalContractImpact::ExternalPolicyRequired,
                "Table field was removed; no rename lineage is inferred.",
                &[
                    "COMPAT-RELEASED-004",
                    "COMPAT-RELEASED-007",
                    "COMPAT-RELEASED-008",
                ],
            ),
            (Some(left), Some(right)) => {
                if left.base_type != right.base_type {
                    push_change(
                        changes,
                        subject(
                            CompatibilitySubjectKind::Field,
                            table.clone(),
                            Some(name.clone()),
                        ),
                        CompatibilityChangeKind::FieldTypeChanged,
                        Some(locator("field", &table, Some(&name))),
                        Some(locator("field", &table, Some(&name))),
                        GeneratedApiImpact::Breaking,
                        SourceMigrationImpact::ManualActionRequired,
                        None,
                        ArtifactBinaryImpact::CrossSchemaInteroperabilityNotGuaranteed,
                        ExternalContractImpact::ExternalPolicyRequired,
                        "Table field base type changed.",
                        &[
                            "COMPAT-RELEASED-007",
                            "COMPAT-RELEASED-008",
                            "COMPAT-RELEASED-009",
                        ],
                    );
                }
                if left.modifier != right.modifier {
                    push_change(
                        changes,
                        subject(
                            CompatibilitySubjectKind::Field,
                            table.clone(),
                            Some(name.clone()),
                        ),
                        CompatibilityChangeKind::FieldModifierChanged,
                        Some(locator("field", &table, Some(&name))),
                        Some(locator("field", &table, Some(&name))),
                        GeneratedApiImpact::Breaking,
                        SourceMigrationImpact::ManualActionRequired,
                        None,
                        ArtifactBinaryImpact::CrossSchemaInteroperabilityNotGuaranteed,
                        ExternalContractImpact::ExternalPolicyRequired,
                        "Table field modifier or nullability changed.",
                        &[
                            "COMPAT-RELEASED-007",
                            "COMPAT-RELEASED-008",
                            "COMPAT-RELEASED-009",
                        ],
                    );
                }
                if left.key != right.key {
                    push_change(
                        changes,
                        subject(
                            CompatibilitySubjectKind::Field,
                            table.clone(),
                            Some(name.clone()),
                        ),
                        CompatibilityChangeKind::FieldMessagePackKeyChanged,
                        Some(locator("field", &table, Some(&name))),
                        Some(locator("field", &table, Some(&name))),
                        GeneratedApiImpact::Unchanged,
                        SourceMigrationImpact::NotRequired,
                        None,
                        ArtifactBinaryImpact::CrossSchemaInteroperabilityNotGuaranteed,
                        ExternalContractImpact::ExternalPolicyRequired,
                        "MessagePack field key changed; it is not logical field identity.",
                        &[
                            "COMPAT-RELEASED-004",
                            "COMPAT-RELEASED-005",
                            "COMPAT-RELEASED-009",
                        ],
                    );
                }
            }
            (None, None) => unreachable!(),
        }
    }

    if baseline.fields.len() == current.fields.len()
        && baseline
            .fields
            .iter()
            .map(|field| &field.name)
            .collect::<BTreeSet<_>>()
            == current
                .fields
                .iter()
                .map(|field| &field.name)
                .collect::<BTreeSet<_>>()
        && baseline
            .fields
            .iter()
            .map(|field| &field.name)
            .collect::<Vec<_>>()
            != current
                .fields
                .iter()
                .map(|field| &field.name)
                .collect::<Vec<_>>()
    {
        push_change(
            changes,
            subject(CompatibilitySubjectKind::Table, table.clone(), None),
            CompatibilityChangeKind::FieldDeclarationReordered,
            Some(locator("table", &table, None)),
            Some(locator("table", &table, None)),
            GeneratedApiImpact::Unchanged,
            SourceMigrationImpact::NotRequired,
            None,
            ArtifactBinaryImpact::RebuildRequired,
            ExternalContractImpact::NotAssessed,
            "Table field declaration order changed without changing field symbols.",
            &["COMPAT-RELEASED-004", "COMPAT-RELEASED-009"],
        );
    }

    if baseline.primary_key.fields != current.primary_key.fields {
        let baseline_query = query_name_for_fields(baseline, &baseline.primary_key.fields);
        let current_query = query_name_for_fields(current, &current.primary_key.fields);
        let reason = format!(
            "Primary Key ordered field sequence changed ({baseline_query} -> {current_query})."
        );
        push_change(
            changes,
            subject(CompatibilitySubjectKind::PrimaryKey, table.clone(), None),
            CompatibilityChangeKind::PrimaryKeyChanged,
            Some(locator("primary-key", &table, None)),
            Some(locator("primary-key", &table, None)),
            GeneratedApiImpact::Breaking,
            SourceMigrationImpact::ManualActionRequired,
            None,
            ArtifactBinaryImpact::CrossSchemaInteroperabilityNotGuaranteed,
            ExternalContractImpact::ExternalPolicyRequired,
            &reason,
            &[
                "COMPAT-RELEASED-005",
                "COMPAT-RELEASED-007",
                "COMPAT-RELEASED-009",
            ],
        );
    }

    compare_secondary_keys(baseline, current, changes);
    compare_references(baseline, current, changes);
}

fn compare_secondary_keys(
    baseline: &ResolvedTable,
    current: &ResolvedTable,
    changes: &mut Vec<CompatibilityChange>,
) {
    let table = baseline.identity.clone();
    let baseline_by_shape = baseline
        .secondary_keys
        .iter()
        .map(|key| (key.fields.clone(), key))
        .collect::<BTreeMap<_, _>>();
    let current_by_shape = current
        .secondary_keys
        .iter()
        .map(|key| (key.fields.clone(), key))
        .collect::<BTreeMap<_, _>>();
    let shapes = baseline_by_shape
        .keys()
        .chain(current_by_shape.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    for shape in shapes {
        let member = Some(format!("[{}]", shape.join(",")));
        match (baseline_by_shape.get(&shape), current_by_shape.get(&shape)) {
            (None, Some(_)) => {
                let query_name = query_name_for_fields(current, &shape);
                let reason =
                    format!("Secondary Key ordered field sequence was added ({query_name}).");
                push_change(
                    changes,
                    subject(
                        CompatibilitySubjectKind::SecondaryKey,
                        table.clone(),
                        member.clone(),
                    ),
                    CompatibilityChangeKind::SecondaryKeyAdded,
                    None,
                    Some(locator("secondary-key", &table, member.as_deref())),
                    GeneratedApiImpact::Additive,
                    SourceMigrationImpact::ManualActionRequired,
                    None,
                    ArtifactBinaryImpact::CrossSchemaInteroperabilityNotGuaranteed,
                    ExternalContractImpact::NotAssessed,
                    &reason,
                    &[
                        "COMPAT-RELEASED-005",
                        "COMPAT-RELEASED-007",
                        "COMPAT-RELEASED-008",
                    ],
                );
            }
            (Some(_), None) => {
                let query_name = query_name_for_fields(baseline, &shape);
                let reason =
                    format!("Secondary Key ordered field sequence was removed ({query_name}).");
                push_change(
                    changes,
                    subject(
                        CompatibilitySubjectKind::SecondaryKey,
                        table.clone(),
                        member.clone(),
                    ),
                    CompatibilityChangeKind::SecondaryKeyRemoved,
                    Some(locator("secondary-key", &table, member.as_deref())),
                    None,
                    GeneratedApiImpact::Breaking,
                    SourceMigrationImpact::ManualActionRequired,
                    None,
                    ArtifactBinaryImpact::CrossSchemaInteroperabilityNotGuaranteed,
                    ExternalContractImpact::ExternalPolicyRequired,
                    &reason,
                    &[
                        "COMPAT-RELEASED-005",
                        "COMPAT-RELEASED-007",
                        "COMPAT-RELEASED-008",
                    ],
                );
            }
            (Some(left), Some(right)) if left.non_unique != right.non_unique => push_change(
                changes,
                subject(
                    CompatibilitySubjectKind::SecondaryKey,
                    table.clone(),
                    member.clone(),
                ),
                CompatibilityChangeKind::SecondaryKeyUniquenessChanged,
                Some(locator("secondary-key", &table, member.as_deref())),
                Some(locator("secondary-key", &table, member.as_deref())),
                GeneratedApiImpact::Breaking,
                SourceMigrationImpact::ManualActionRequired,
                None,
                ArtifactBinaryImpact::CrossSchemaInteroperabilityNotGuaranteed,
                ExternalContractImpact::ExternalPolicyRequired,
                "Secondary Key unique/non-unique return contract changed.",
                &[
                    "COMPAT-RELEASED-005",
                    "COMPAT-RELEASED-007",
                    "COMPAT-RELEASED-009",
                ],
            ),
            _ => {}
        }
    }
    let baseline_shapes = baseline
        .secondary_keys
        .iter()
        .map(|key| key.fields.clone())
        .collect::<Vec<_>>();
    let current_shapes = current
        .secondary_keys
        .iter()
        .map(|key| key.fields.clone())
        .collect::<Vec<_>>();
    if baseline_shapes == current_shapes {
        return;
    }
    let baseline_set = baseline_shapes.iter().cloned().collect::<BTreeSet<_>>();
    let current_set = current_shapes.iter().cloned().collect::<BTreeSet<_>>();
    if baseline_set == current_set {
        push_change(
            changes,
            subject(CompatibilitySubjectKind::SecondaryKey, table.clone(), None),
            CompatibilityChangeKind::SecondaryKeyDeclarationReordered,
            Some(locator("secondary-keys", &table, None)),
            Some(locator("secondary-keys", &table, None)),
            GeneratedApiImpact::Unchanged,
            SourceMigrationImpact::NotRequired,
            None,
            ArtifactBinaryImpact::RebuildRequired,
            ExternalContractImpact::NotAssessed,
            "Secondary Key declaration order changed; backend indexNo is not logical identity.",
            &[
                "COMPAT-RELEASED-004",
                "COMPAT-RELEASED-005",
                "COMPAT-RELEASED-009",
            ],
        );
    }
}

fn query_name_for_fields(table: &ResolvedTable, names: &[String]) -> String {
    let fields = names
        .iter()
        .filter_map(|name| table.fields.iter().find(|field| field.name == *name))
        .cloned()
        .collect::<Vec<_>>();
    generated_query_name(&fields)
}

fn compare_references(
    baseline: &ResolvedTable,
    current: &ResolvedTable,
    changes: &mut Vec<CompatibilityChange>,
) {
    let table = baseline.identity.clone();
    let baseline_by_name = baseline
        .references
        .iter()
        .map(|reference| (reference.name.clone(), reference))
        .collect::<BTreeMap<_, _>>();
    let current_by_name = current
        .references
        .iter()
        .map(|reference| (reference.name.clone(), reference))
        .collect::<BTreeMap<_, _>>();
    let names = baseline_by_name
        .keys()
        .chain(current_by_name.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    for name in names {
        match (baseline_by_name.get(&name), current_by_name.get(&name)) {
            (None, Some(_)) => push_change(
                changes,
                subject(
                    CompatibilitySubjectKind::Reference,
                    table.clone(),
                    Some(name.clone()),
                ),
                CompatibilityChangeKind::ReferenceAdded,
                None,
                Some(locator("reference", &table, Some(&name))),
                GeneratedApiImpact::Additive,
                SourceMigrationImpact::SupportedOperation,
                Some("AddReference"),
                ArtifactBinaryImpact::CrossSchemaInteroperabilityNotGuaranteed,
                ExternalContractImpact::NotAssessed,
                "Reference declaration was added.",
                &[
                    "COMPAT-RELEASED-004",
                    "COMPAT-RELEASED-005",
                    "COMPAT-RELEASED-007",
                ],
            ),
            (Some(_), None) => push_change(
                changes,
                subject(
                    CompatibilitySubjectKind::Reference,
                    table.clone(),
                    Some(name.clone()),
                ),
                CompatibilityChangeKind::ReferenceRemoved,
                Some(locator("reference", &table, Some(&name))),
                None,
                GeneratedApiImpact::Breaking,
                SourceMigrationImpact::ManualActionRequired,
                Some("RemoveReference"),
                ArtifactBinaryImpact::CrossSchemaInteroperabilityNotGuaranteed,
                ExternalContractImpact::NotAssessed,
                "Reference declaration was removed; no rename lineage is inferred.",
                &[
                    "COMPAT-RELEASED-004",
                    "COMPAT-RELEASED-007",
                    "COMPAT-RELEASED-008",
                ],
            ),
            (Some(left), Some(right)) => compare_reference(&table, left, right, changes),
            (None, None) => unreachable!(),
        }
    }
}

fn compare_reference(
    table: &str,
    baseline: &ResolvedReference,
    current: &ResolvedReference,
    changes: &mut Vec<CompatibilityChange>,
) {
    let member = Some(baseline.name.clone());
    let baseline_locator = Some(locator("reference", table, member.as_deref()));
    let current_locator = baseline_locator.clone();
    if reference_csharp_name(&baseline.name, baseline.csharp_name.as_deref())
        != reference_csharp_name(&current.name, current.csharp_name.as_deref())
    {
        push_change(
            changes,
            subject(
                CompatibilitySubjectKind::Reference,
                table.to_owned(),
                member.clone(),
            ),
            CompatibilityChangeKind::ReferencePresentationChanged,
            baseline_locator.clone(),
            current_locator.clone(),
            GeneratedApiImpact::Breaking,
            SourceMigrationImpact::NotRequired,
            None,
            ArtifactBinaryImpact::RebuildRequired,
            ExternalContractImpact::NotAssessed,
            "Reference generated C# helper identifier changed.",
            &["COMPAT-RELEASED-005", "COMPAT-RELEASED-007"],
        );
    }
    if baseline.source_fields != current.source_fields {
        push_change(
            changes,
            subject(
                CompatibilitySubjectKind::Reference,
                table.to_owned(),
                member.clone(),
            ),
            CompatibilityChangeKind::ReferenceSourceChanged,
            baseline_locator.clone(),
            current_locator.clone(),
            GeneratedApiImpact::ReviewRequired,
            SourceMigrationImpact::ManualActionRequired,
            Some("EditReference"),
            ArtifactBinaryImpact::CrossSchemaInteroperabilityNotGuaranteed,
            ExternalContractImpact::NotAssessed,
            "Reference source field sequence changed.",
            &[
                "COMPAT-RELEASED-005",
                "COMPAT-RELEASED-007",
                "COMPAT-RELEASED-008",
            ],
        );
    }
    if baseline.target_table != current.target_table
        || baseline.target_fields != current.target_fields
        || baseline.target_key_kind != current.target_key_kind
    {
        push_change(
            changes,
            subject(
                CompatibilitySubjectKind::Reference,
                table.to_owned(),
                member.clone(),
            ),
            CompatibilityChangeKind::ReferenceTargetChanged,
            baseline_locator.clone(),
            current_locator.clone(),
            GeneratedApiImpact::Breaking,
            SourceMigrationImpact::ManualActionRequired,
            Some("EditReference"),
            ArtifactBinaryImpact::CrossSchemaInteroperabilityNotGuaranteed,
            ExternalContractImpact::NotAssessed,
            "Reference target table or ordered target key changed.",
            &[
                "COMPAT-RELEASED-005",
                "COMPAT-RELEASED-007",
                "COMPAT-RELEASED-008",
            ],
        );
    }
    if baseline.optionality != current.optionality {
        push_change(
            changes,
            subject(
                CompatibilitySubjectKind::Reference,
                table.to_owned(),
                member.clone(),
            ),
            CompatibilityChangeKind::ReferenceOptionalityChanged,
            baseline_locator.clone(),
            current_locator.clone(),
            GeneratedApiImpact::Breaking,
            SourceMigrationImpact::ManualActionRequired,
            Some("EditReference"),
            ArtifactBinaryImpact::CrossSchemaInteroperabilityNotGuaranteed,
            ExternalContractImpact::NotAssessed,
            "Reference Required/Nullable contract changed.",
            &[
                "COMPAT-RELEASED-005",
                "COMPAT-RELEASED-007",
                "COMPAT-RELEASED-008",
            ],
        );
    }
    if baseline.cardinality != current.cardinality {
        push_change(
            changes,
            subject(
                CompatibilitySubjectKind::Reference,
                table.to_owned(),
                member,
            ),
            CompatibilityChangeKind::ReferenceCardinalityChanged,
            baseline_locator,
            current_locator,
            GeneratedApiImpact::Breaking,
            SourceMigrationImpact::ManualActionRequired,
            Some("EditReference"),
            ArtifactBinaryImpact::CrossSchemaInteroperabilityNotGuaranteed,
            ExternalContractImpact::NotAssessed,
            "Reference single/multi return contract changed.",
            &[
                "COMPAT-RELEASED-005",
                "COMPAT-RELEASED-007",
                "COMPAT-RELEASED-008",
            ],
        );
    }
}

fn compare_data(
    baseline: &ResolvedCompatibilitySnapshot,
    current: &ResolvedCompatibilitySnapshot,
    changes: &mut Vec<CompatibilityChange>,
) {
    let table_names = baseline
        .data
        .keys()
        .chain(current.data.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    for table in table_names {
        if !baseline.tables.contains_key(&table) || !current.tables.contains_key(&table) {
            continue;
        }
        if baseline.data.get(&table) != current.data.get(&table) {
            push_change(
                changes,
                subject(CompatibilitySubjectKind::Data, table.clone(), None),
                CompatibilityChangeKind::DataChanged,
                Some(locator("data", &table, None)),
                Some(locator("data", &table, None)),
                GeneratedApiImpact::Unchanged,
                SourceMigrationImpact::NotRequired,
                None,
                ArtifactBinaryImpact::RebuildRequired,
                ExternalContractImpact::NotAssessed,
                "Canonical record values changed without a schema/API change.",
                &["COMPAT-RELEASED-003", "COMPAT-RELEASED-009"],
            );
        }
    }
}

fn subject(
    kind: CompatibilitySubjectKind,
    owner: String,
    member: Option<String>,
) -> CompatibilitySubject {
    CompatibilitySubject {
        kind,
        owner,
        member,
    }
}

fn locator(kind: &str, owner: &str, member: Option<&str>) -> String {
    match member {
        Some(member) => format!("{kind}:{owner}:{member}"),
        None => format!("{kind}:{owner}"),
    }
}

#[allow(clippy::too_many_arguments)]
fn push_change(
    changes: &mut Vec<CompatibilityChange>,
    subject: CompatibilitySubject,
    kind: CompatibilityChangeKind,
    baseline_locator: Option<String>,
    current_locator: Option<String>,
    generated_api: GeneratedApiImpact,
    source_migration: SourceMigrationImpact,
    source_operation: Option<&str>,
    artifact_binary: ArtifactBinaryImpact,
    external_contract: ExternalContractImpact,
    reason: &str,
    requirements: &[&str],
) {
    changes.push(CompatibilityChange {
        subject,
        kind,
        baseline_locator,
        current_locator,
        generated_api,
        source_migration,
        source_operation: source_operation.map(str::to_owned),
        artifact_binary,
        external_contract,
        reason: reason.to_owned(),
        related_requirements: requirements
            .iter()
            .map(|value| (*value).to_owned())
            .collect(),
    });
}

fn summarize(changes: &[CompatibilityChange]) -> CompatibilitySummary {
    let generated_api = changes
        .iter()
        .map(|change| generated_api_name(change.generated_api).to_owned())
        .collect::<Vec<_>>();
    let source_migration = changes
        .iter()
        .map(|change| source_migration_name(change.source_migration).to_owned())
        .collect::<Vec<_>>();
    let artifact_binary = changes
        .iter()
        .map(|change| artifact_binary_name(change.artifact_binary).to_owned())
        .collect::<Vec<_>>();
    let external_contract = changes
        .iter()
        .map(|change| external_contract_name(change.external_contract).to_owned())
        .collect::<Vec<_>>();
    CompatibilitySummary {
        change_count: changes.len(),
        generated_api: classification_counts(generated_api),
        source_migration: classification_counts(source_migration),
        artifact_binary: classification_counts(artifact_binary),
        external_contract: classification_counts(external_contract),
    }
}

fn generated_api_name(value: GeneratedApiImpact) -> &'static str {
    match value {
        GeneratedApiImpact::Unchanged => "unchanged",
        GeneratedApiImpact::Additive => "additive",
        GeneratedApiImpact::Breaking => "breaking",
        GeneratedApiImpact::ReviewRequired => "review_required",
    }
}

fn source_migration_name(value: SourceMigrationImpact) -> &'static str {
    match value {
        SourceMigrationImpact::NotRequired => "not_required",
        SourceMigrationImpact::SupportedOperation => "supported_operation",
        SourceMigrationImpact::DestructiveAuthorizationRequired => {
            "destructive_authorization_required"
        }
        SourceMigrationImpact::ManualActionRequired => "manual_action_required",
        SourceMigrationImpact::ReviewRequired => "review_required",
    }
}

fn artifact_binary_name(value: ArtifactBinaryImpact) -> &'static str {
    match value {
        ArtifactBinaryImpact::Unchanged => "unchanged",
        ArtifactBinaryImpact::RebuildRequired => "rebuild_required",
        ArtifactBinaryImpact::CrossSchemaInteroperabilityNotGuaranteed => {
            "cross_schema_interoperability_not_guaranteed"
        }
    }
}

fn external_contract_name(value: ExternalContractImpact) -> &'static str {
    match value {
        ExternalContractImpact::NotAssessed => "not_assessed",
        ExternalContractImpact::ExternalPolicyRequired => "external_policy_required",
    }
}

fn classification_counts(values: Vec<String>) -> Vec<CompatibilityClassificationCount> {
    let mut counts = BTreeMap::<String, usize>::new();
    for value in values {
        *counts.entry(value).or_default() += 1;
    }
    counts
        .into_iter()
        .map(|(classification, count)| CompatibilityClassificationCount {
            classification,
            count,
        })
        .collect()
}
