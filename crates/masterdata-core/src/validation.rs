use std::collections::{BTreeMap, BTreeSet};

use crate::Diagnostic;
use crate::document::{ProjectDocuments, SourceDocument, schema_diagnostic};
use crate::type_system::{
    PrimitiveType, TypeSystem, build_type_system, is_csharp_reserved_keyword,
};

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
    let mut diagnostics = Vec::new();
    let type_build = build_type_system(documents);
    diagnostics.extend(type_build.diagnostics.iter().cloned());
    let type_system = type_build.model.as_ref();
    let mut schemas = BTreeMap::new();
    let mut tables = BTreeSet::new();
    let mut data_counts = 0;
    let mut type_names = BTreeSet::new();

    for loaded in &documents.files {
        if let Some(table) = loaded.document.table_identity() {
            tables.insert(table.to_owned());
        }
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
                validate_schema_fields(schema, type_system, &loaded.path, &mut diagnostics);
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
            SourceDocument::Type(document) => {
                type_names.insert(document.name.clone());
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
        type_documents: type_names.len(),
        tables: tables
            .into_iter()
            .filter(|table| !table.is_empty())
            .collect(),
        types: type_names.into_iter().collect(),
        diagnostics,
    }
}

fn validate_schema_fields(
    schema: &crate::SchemaDocument,
    type_system: Option<&TypeSystem>,
    path: &std::path::Path,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut field_keys = BTreeSet::new();
    let mut field_names = BTreeSet::new();
    for field in &schema.fields {
        if !field_keys.insert(field.key) {
            diagnostics.push(schema_diagnostic(
                "E-SCHEMA-DUPLICATE-FIELD-KEY",
                format!(
                    "duplicate MessagePack key {} in table `{}`",
                    field.key, schema.table
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
        if !is_table_field_name(&field.name) || is_csharp_reserved_keyword(&field.name) {
            diagnostics.push(schema_diagnostic(
                "E-SCHEMA-INVALID-FIELD-NAME",
                format!(
                    "field name `{}` is not a valid Table source name",
                    field.name
                ),
                path,
            ));
        }
        if field.nullable && field.array {
            diagnostics.push(schema_diagnostic(
                "E-SCHEMA-INVALID-FIELD-MODIFIERS",
                format!("field `{}` cannot be both nullable and array", field.name),
                path,
            ));
        }
        if let Some(type_system) = type_system
            && type_system.resolve_reference(&field.type_name).is_none()
        {
            diagnostics.push(schema_diagnostic(
                "E-TYPE-UNKNOWN-REFERENCE",
                format!("unknown type reference `{}`", field.type_name),
                path,
            ));
        } else if type_system.is_none() && PrimitiveType::parse(&field.type_name).is_none() {
            diagnostics.push(schema_diagnostic(
                "E-TYPE-UNKNOWN-REFERENCE",
                format!("unknown type reference `{}`", field.type_name),
                path,
            ));
        }
    }
}

fn is_table_field_name(value: &str) -> bool {
    let mut chars = value.chars();
    matches!(chars.next(), Some(first) if first.is_ascii_lowercase())
        && chars.all(|character| character.is_ascii_alphanumeric())
}
