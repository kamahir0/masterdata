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
mod validation;

pub use application::ProjectService;
pub use config::{BuildConfig, ProjectConfig, ProjectMetadata, SourceConfig};
pub use document::{
    DataDocument, FieldDefinition, LoadedDocument, ProjectDocuments, ReservedField, SchemaDocument,
    SourceDocument,
};
pub use error::{Diagnostic, ErrorKind, MasterdataError, Result};
pub use pipeline::{BuildPlan, BuildStatus, compute_schema_hash};
pub use project::{InitOptions, PROJECT_CONFIG_FILENAME, Project, ProjectInfo, initialize_project};
pub use validation::{ValidationReport, validate_documents};
