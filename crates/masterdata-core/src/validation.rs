use std::collections::{BTreeMap, BTreeSet};

use crate::Diagnostic;
use crate::document::{ProjectDocuments, SourceDocument};
use crate::table::{BuildSelection, resolve_tables};
use crate::type_system::build_type_system;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct ValidationReport {
    pub valid: bool,
    pub files_scanned: usize,
    pub schema_documents: usize,
    pub data_documents: usize,
    pub type_documents: usize,
    pub tables: Vec<String>,
    pub types: Vec<String>,
    pub diagnostics: Vec<Diagnostic>,
}

impl ValidationReport {
    pub fn is_valid(&self) -> bool {
        self.valid
    }
}

pub fn validate_documents(documents: &ProjectDocuments) -> ValidationReport {
    validate_documents_with_selection(documents, &BuildSelection::unfiltered())
}

/// Shared validation report plus resolved Reference metadata for authoring views.
pub struct AuthoringValidation {
    pub report: ValidationReport,
    pub references: BTreeMap<String, Vec<crate::ResolvedReference>>,
    pub reference_diagnostics: Vec<Diagnostic>,
}

pub fn validate_authoring_documents(documents: &ProjectDocuments) -> AuthoringValidation {
    validation_model(documents, &BuildSelection::unfiltered())
}

/// Selection-aware constraints follow the profile-independent structure checks.
pub fn validate_documents_with_selection(
    documents: &ProjectDocuments,
    selection: &BuildSelection,
) -> ValidationReport {
    validation_model(documents, selection).report
}

fn validation_model(
    documents: &ProjectDocuments,
    selection: &BuildSelection,
) -> AuthoringValidation {
    let mut references = BTreeMap::new();
    let mut reference_diagnostics = Vec::new();
    let mut diagnostics = Vec::new();
    let _read_span = crate::read_trace::read_span("validation");
    let type_build = build_type_system(documents);
    diagnostics.extend(type_build.diagnostics.iter().cloned());

    let mut tables = BTreeSet::new();
    let mut type_names = BTreeSet::new();
    for loaded in &documents.files {
        if matches!(
            loaded.document,
            SourceDocument::Schema(_) | SourceDocument::Data(_)
        ) && let Some(table) = loaded.document.table_identity()
            && !table.is_empty()
        {
            tables.insert(table.to_owned());
        }
        if let SourceDocument::Type(document) = &loaded.document {
            type_names.insert(document.name.clone());
        }
    }

    if let Some(type_system) = type_build.model.as_ref() {
        let table_build = resolve_tables(documents, type_system, selection);
        reference_diagnostics = table_build.diagnostics;
        diagnostics.extend(reference_diagnostics.iter().cloned());
        if let Some(model) = table_build.model {
            references.extend(
                model
                    .into_iter()
                    .map(|table| (table.identity, table.references)),
            );
        }
    }

    if documents.files.is_empty() {
        diagnostics.push(Diagnostic::new(
            "E-PROJECT-NO-SOURCES",
            crate::ErrorKind::Validation,
            "project has no YAML source documents",
        ));
    }

    let valid = diagnostics.is_empty();
    let report = ValidationReport {
        valid,
        files_scanned: documents.files.len(),
        schema_documents: documents.schemas().count(),
        data_documents: documents.data().count(),
        type_documents: type_names.len(),
        tables: tables.into_iter().collect(),
        types: type_names.into_iter().collect(),
        diagnostics,
    };
    AuthoringValidation {
        report,
        references,
        reference_diagnostics,
    }
}
