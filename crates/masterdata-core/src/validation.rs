use std::collections::{BTreeMap, BTreeSet};

use crate::Diagnostic;
use crate::document::{ProjectDocuments, SourceDocument, schema_diagnostic};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct ValidationReport {
    pub valid: bool,
    pub files_scanned: usize,
    pub schema_documents: usize,
    pub data_documents: usize,
    pub tables: Vec<String>,
    pub diagnostics: Vec<Diagnostic>,
}

impl ValidationReport {
    pub fn is_valid(&self) -> bool {
        self.valid
    }
}

pub fn validate_documents(documents: &ProjectDocuments) -> ValidationReport {
    let mut diagnostics = Vec::new();
    let mut schemas = BTreeMap::new();
    let mut tables = BTreeSet::new();
    let mut data_counts = 0;

    for loaded in &documents.files {
        tables.insert(loaded.document.table().to_owned());
        match &loaded.document {
            SourceDocument::Schema(schema) => {
                if schema.table.trim().is_empty() {
                    diagnostics.push(schema_diagnostic(
                        "E-SCHEMA-EMPTY-TABLE",
                        "schema table must not be empty",
                        &loaded.path,
                    ));
                }
                if let Some(previous) = schemas.insert(schema.table.clone(), loaded.path.clone()) {
                    diagnostics.push(
                        schema_diagnostic(
                            "E-SCHEMA-DUPLICATE-TABLE",
                            format!(
                                "table `{}` has more than one schema document (also declared in {})",
                                schema.table,
                                previous.display()
                            ),
                            &loaded.path,
                        )
                        .with_schema_path(format!("table:{}", schema.table)),
                    );
                }
                validate_schema_fields(schema, &loaded.path, &mut diagnostics);
            }
            SourceDocument::Data(data) => {
                data_counts += 1;
                if data.table.trim().is_empty() {
                    diagnostics.push(schema_diagnostic(
                        "E-DATA-EMPTY-TABLE",
                        "data table must not be empty",
                        &loaded.path,
                    ));
                }
            }
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
    ValidationReport {
        valid,
        files_scanned: documents.files.len(),
        schema_documents: schemas.len(),
        data_documents: data_counts,
        tables: tables
            .into_iter()
            .filter(|table| !table.is_empty())
            .collect(),
        diagnostics,
    }
}

fn validate_schema_fields(
    schema: &crate::SchemaDocument,
    path: &std::path::Path,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut field_ids = BTreeSet::new();
    let mut field_names = BTreeSet::new();
    let reserved_ids: BTreeSet<_> = schema
        .reserved_fields
        .iter()
        .map(|field| field.id)
        .collect();
    for field in &schema.fields {
        if !field_ids.insert(field.id) {
            diagnostics.push(schema_diagnostic(
                "E-SCHEMA-DUPLICATE-FIELD-ID",
                format!(
                    "duplicate field id {} in table `{}`",
                    field.id, schema.table
                ),
                path,
            ));
        }
        if !field_names.insert(field.name.clone()) {
            diagnostics.push(schema_diagnostic(
                "E-SCHEMA-DUPLICATE-FIELD-NAME",
                format!(
                    "duplicate field name `{}` in table `{}`",
                    field.name, schema.table
                ),
                path,
            ));
        }
        if reserved_ids.contains(&field.id) {
            diagnostics.push(schema_diagnostic(
                "E-SCHEMA-ACTIVE-RESERVED-FIELD-ID-COLLISION",
                format!("field id {} is both active and reserved", field.id),
                path,
            ));
        }
    }
}
