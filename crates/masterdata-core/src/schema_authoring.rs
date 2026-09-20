use std::{collections::BTreeMap, path::Path};

use serde::Serialize;

use crate::{
    ConversionDefinition, ErrorKind, FieldDefinition, MasterdataError, ProjectDocuments,
    ResolvedAuthoringType, Result, SchemaDocument, SourceDocument, TypeFieldDefinition,
    creation_choices, migration_table_schema, migration_type_declaration,
    resolve_authoring_field_shape,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TableSnapshot {
    pub path: String,
    pub schema: SchemaDocument,
    pub field_types: Vec<String>,
    pub initializer_shapes: BTreeMap<String, ResolvedAuthoringType>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TypeSnapshot {
    pub path: String,
    pub name: String,
    pub category: String,
    pub underlying: Option<String>,
    pub conversions: Option<ConversionDefinition>,
    pub members: Vec<TypeMemberView>,
    pub fields: Vec<TypeFieldDefinition>,
    pub field_types: Vec<String>,
    pub initializer_shapes: BTreeMap<String, ResolvedAuthoringType>,
}

#[derive(Debug, Serialize)]
pub struct TypeMemberView {
    pub name: String,
    pub value: String,
}

pub fn initializer_shapes(
    documents: &ProjectDocuments,
    field_types: &[String],
) -> BTreeMap<String, ResolvedAuthoringType> {
    field_types
        .iter()
        .filter_map(|type_name| {
            let probe = FieldDefinition {
                key: 0,
                name: "initializer".to_owned(),
                type_name: type_name.clone(),
                nullable: false,
                array: false,
            };
            resolve_authoring_field_shape(documents, &probe)
                .map(|field| (type_name.clone(), field.shape))
        })
        .collect()
}

pub fn table_snapshot(
    documents: &ProjectDocuments,
    path: &Path,
    display_path: &str,
) -> Result<TableSnapshot> {
    let file = documents
        .files
        .iter()
        .find(|file| file.path == path)
        .ok_or_else(|| editor_error("E-TABLE-EDITOR-SOURCE", "schema source not found"))?;
    let SourceDocument::Schema(schema) = &file.document else {
        return Err(editor_error(
            "E-TABLE-EDITOR-KIND",
            "source is not a Table schema",
        ));
    };
    let field_types = creation_choices(documents).field_types;
    Ok(TableSnapshot {
        path: display_path.into(),
        schema: migration_table_schema(documents, &schema.table)?,
        initializer_shapes: initializer_shapes(documents, &field_types),
        field_types,
    })
}

pub fn type_snapshot(
    documents: &ProjectDocuments,
    path: &Path,
    display_path: &str,
) -> Result<TypeSnapshot> {
    let file = documents
        .files
        .iter()
        .find(|file| file.path == path)
        .ok_or_else(|| editor_error("E-TYPE-EDITOR-INPUT", "type source not found"))?;
    let SourceDocument::Type(ty) = &file.document else {
        return Err(editor_error("E-TYPE-EDITOR-INPUT", "source is not a type"));
    };
    let ty = migration_type_declaration(documents, &ty.name)?;
    let field_types = creation_choices(documents).field_types;
    let mut result = TypeSnapshot {
        path: display_path.into(),
        name: ty.name,
        category: String::new(),
        underlying: None,
        conversions: None,
        members: vec![],
        fields: vec![],
        initializer_shapes: initializer_shapes(documents, &field_types),
        field_types,
    };
    if let Some(vo) = ty.value_object {
        result.category = "Value Object".into();
        result.underlying = Some(vo.underlying);
        result.conversions = Some(vo.conversions);
    } else if let Some(custom) = ty.custom {
        result.category = "Custom Type".into();
        result.fields = custom.fields;
    } else {
        let (category, underlying, members) = if let Some(e) = ty.enum_definition {
            ("Enum", e.underlying, e.members)
        } else {
            let f = ty
                .flags
                .ok_or_else(|| editor_error("E-TYPE-EDITOR-INPUT", "unresolved category"))?;
            ("Flags Enum", f.underlying, f.members)
        };
        result.category = category.into();
        result.underlying = Some(underlying);
        result.members = members
            .into_iter()
            .map(|m| TypeMemberView {
                name: m.name,
                value: m.value.0.to_string(),
            })
            .collect();
    }
    Ok(result)
}

fn editor_error(code: &str, message: &str) -> MasterdataError {
    MasterdataError::new(code, ErrorKind::Validation, message)
}
