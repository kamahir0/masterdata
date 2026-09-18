//! Shared domain and application boundary for the masterdata product.
//!
//! CLI and Tauri are intentionally thin adapters around this crate. Filesystem
//! discovery, configuration loading, YAML classification, and validation live
//! here so that the two frontends cannot gradually grow different semantics.

mod application;
mod authoring_batch;
mod authoring_query;
mod authoring_value;
mod config;
mod config_edit;
pub mod diagnostics;
mod document;
mod error;
mod migration;
mod migration_commit;
mod pipeline;
mod project;
mod source_creation;
mod source_edit;
mod table;
mod type_system;
mod validation;

pub use application::{NativeProjectService, ProjectService};
pub use authoring_batch::{
    authoring_value_from_clipboard, authoring_value_to_clipboard, decode_clipboard_tsv,
    encode_clipboard_tsv, is_scalar_batch_field,
};
pub use authoring_query::{
    AuthoringQuery, ColumnFilter, QueryOperator, QueryRow, QuerySort, SortDirection,
    apply_authoring_query,
};
pub use authoring_value::{
    AuthoringMember, AuthoringSequenceItem, AuthoringValue, project_source_value,
    project_typed_source_value,
};
pub use config::{
    BuildConfig, BuildProfile, ProjectConfig, ProjectMetadata, PublishConfig, PublishTarget,
    PublishTargetKind, SourceConfig,
};
pub use config_edit::{
    ProjectConfigEditOperation, ProjectConfigEditPreview, preview_project_config_edit,
};
pub use document::{
    ConversionDefinition, CustomTypeDefinition, DataDocument, EnumDefinition, EnumMember,
    FieldDefinition, FlagsDefinition, IntegerLiteral, LoadedDocument, PrimaryKeyDefinition,
    ProjectDocuments, SchemaDocument, SecondaryKeyDefinition, SourceDocument, TypeDocument,
    TypeFieldDefinition, ValueObjectDefinition, parse_yaml_document,
};
pub use error::{Diagnostic, ErrorKind, MasterdataError, Result};
pub use migration::{
    AddFieldCommand, DropFieldCommand, MigrationCommand, MigrationDryRun, MigrationFilePlan,
    MigrationOperation, MigrationPatch, MigrationPlan, MigrationValidation, RenameFieldCommand,
    dry_run_migration, migration_table_schema, plan_migration,
};
pub use migration_commit::{
    MigrationCommitFailure, MigrationCommitFailureInjection, MigrationCommitFailurePoint,
    MigrationCommitReport, MigrationCommitState, MigrationFileCommitState,
    MigrationFileCommitStatus, commit_migration, commit_migration_authorized,
    commit_migration_authorized_with_failures, commit_migration_with_failures,
};
pub use pipeline::{
    BuildPlan, BuildStatus, SemanticBuildPreparation, compute_schema_source_content_hash,
    prepare_build_from_documents, prepare_build_with_selection, prepare_semantic_build,
};
pub use project::{
    BuildProfileInfo, InitOptions, PROJECT_CONFIG_FILENAME, Project, ProjectInfo,
    PublishTargetInfo, initialize_gui_project, initialize_project,
};
pub use source_edit::{
    AddedRecordDraft, AddedRecordField, RecordTagEdit, RecordValueEdit, SourceEditDryRun,
    SourceEditPlan, SourceRecordMutation, dry_run_source_edit, dry_run_source_record_mutation,
    source_content_identity,
};
pub use table::{
    BuildSelection, ResolvedPrimaryKey, ResolvedRecord, ResolvedSecondaryKey, ResolvedTable,
    TableBuild, is_tag_name, record_tags, resolve_tables, table_csharp_name,
};
pub use type_system::{
    FieldModifier, NormalizedValue, PrimitiveType, ResolvedAuthoringField, ResolvedAuthoringType,
    ResolvedConversions, ResolvedEnumMember, ResolvedField, ResolvedType, TypeCategory,
    TypeReference, TypeSystem, TypeSystemBuild, build_type_system, csharp_property_name,
    is_csharp_reserved_keyword, resolve_authoring_field_shape, resolve_type_system,
};
pub use validation::{ValidationReport, validate_documents, validate_documents_with_selection};

pub use source_creation::{
    CreationChoices, CreationMember, SourceCreation, SourceCreationPlan, creation_choices,
    prepare_source_creation,
};

pub use migration_commit::{SourceCommitCandidate, commit_source_candidate_with_failures};

pub use migration::type_mutation::{
    TypeMigrationCommand, TypeMigrationDryRun, TypeMigrationOperation, dry_run_type_migration,
    migration_type_declaration,
};
