//! Shared domain and application boundary for the masterdata product.
//!
//! CLI and Tauri are intentionally thin adapters around this crate. Filesystem
//! discovery, configuration loading, YAML classification, and validation live
//! here so that the two frontends cannot gradually grow different semantics.

mod application;
mod config;
pub mod diagnostics;
mod document;
mod error;
mod pipeline;
mod project;
mod table;
mod type_system;
mod validation;

pub use application::ProjectService;
pub use config::{
    BuildConfig, ProjectConfig, ProjectMetadata, PublishConfig, PublishTarget, PublishTargetKind,
    SourceConfig,
};
pub use document::{
    ConversionDefinition, CustomTypeDefinition, DataDocument, EnumDefinition, EnumMember,
    FieldDefinition, FlagsDefinition, IntegerLiteral, LoadedDocument, PrimaryKeyDefinition,
    ProjectDocuments, SchemaDocument, SecondaryKeyDefinition, SourceDocument, TypeDocument,
    TypeFieldDefinition, ValueObjectDefinition, parse_yaml_document,
};
pub use error::{Diagnostic, ErrorKind, MasterdataError, Result};
pub use pipeline::{
    BuildPlan, BuildStatus, compute_schema_source_content_hash, prepare_build_with_selection,
};
pub use project::{
    InitOptions, PROJECT_CONFIG_FILENAME, Project, ProjectInfo, PublishTargetInfo,
    initialize_project,
};
pub use table::{
    BuildSelection, ResolvedPrimaryKey, ResolvedRecord, ResolvedSecondaryKey, ResolvedTable,
    TableBuild, resolve_tables,
};
pub use type_system::{
    FieldModifier, NormalizedValue, PrimitiveType, ResolvedConversions, ResolvedEnumMember,
    ResolvedField, ResolvedType, TypeCategory, TypeReference, TypeSystem, TypeSystemBuild,
    build_type_system, csharp_property_name, is_csharp_reserved_keyword, resolve_type_system,
};
pub use validation::{ValidationReport, validate_documents, validate_documents_with_selection};
