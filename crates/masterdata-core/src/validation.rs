use std::collections::{BTreeMap, BTreeSet};

use serde_yaml::Value;

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
                        "SCHEMA-001",
                        "schema table must not be empty",
                        &loaded.path,
                    ));
                }
                if let Some(previous) = schemas.insert(schema.table.clone(), loaded.path.clone()) {
                    diagnostics.push(
                        schema_diagnostic(
                            "SCHEMA-002",
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
                        "DATA-001",
                        "data table must not be empty",
                        &loaded.path,
                    ));
                }
            }
        }
    }

    let mut record_keys: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for loaded in &documents.files {
        let SourceDocument::Data(data) = &loaded.document else {
            continue;
        };
        let keys = record_keys.entry(data.table.clone()).or_default();
        for (index, record) in data.records.iter().enumerate() {
            let Some(id) = record.get("id") else {
                continue;
            };
            let key = yaml_key(id);
            if !keys.insert(key.clone()) {
                diagnostics.push(
                    schema_diagnostic(
                        "DATA-PRIMARY-001",
                        format!("duplicate `id` value in table `{}`", data.table),
                        &loaded.path,
                    )
                    .with_record_identity(format!(
                        "table:{} id:{} record:{}",
                        data.table, key, index
                    )),
                );
            }
        }
    }

    if documents.files.is_empty() {
        diagnostics.push(Diagnostic::new(
            "PROJECT-DOC-001",
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
                "SCHEMA-FIELD-001",
                format!(
                    "duplicate field id {} in table `{}`",
                    field.id, schema.table
                ),
                path,
            ));
        }
        if !field_names.insert(field.name.clone()) {
            diagnostics.push(schema_diagnostic(
                "SCHEMA-FIELD-002",
                format!(
                    "duplicate field name `{}` in table `{}`",
                    field.name, schema.table
                ),
                path,
            ));
        }
        if reserved_ids.contains(&field.id) {
            diagnostics.push(schema_diagnostic(
                "SCHEMA-FIELD-003",
                format!("field id {} is both active and reserved", field.id),
                path,
            ));
        }
    }
}

fn yaml_key(value: &Value) -> String {
    serde_yaml::to_string(value).unwrap_or_else(|_| format!("{value:?}"))
}
