use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde_yaml::Value;

use crate::document::{
    ConversionDefinition, EnumDefinition, EnumMember, FlagsDefinition, IntegerLiteral,
    ProjectDocuments, TypeDocument, TypeFieldDefinition, ValueObjectDefinition,
};
use crate::{Diagnostic, ErrorKind, MasterdataError, Result};

/// The exact primitive vocabulary owned by the Approved Primitive Types
/// specification.  In particular, legacy width aliases are not represented
/// here and therefore cannot accidentally become canonical type references.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PrimitiveType {
    Bool,
    Int,
    UInt,
    Long,
    ULong,
    Float,
    Double,
    String,
}

impl PrimitiveType {
    pub fn parse(name: &str) -> Option<Self> {
        Some(match name {
            "bool" => Self::Bool,
            "int" => Self::Int,
            "uint" => Self::UInt,
            "long" => Self::Long,
            "ulong" => Self::ULong,
            "float" => Self::Float,
            "double" => Self::Double,
            "string" => Self::String,
            _ => return None,
        })
    }

    pub const fn name(self) -> &'static str {
        match self {
            Self::Bool => "bool",
            Self::Int => "int",
            Self::UInt => "uint",
            Self::Long => "long",
            Self::ULong => "ulong",
            Self::Float => "float",
            Self::Double => "double",
            Self::String => "string",
        }
    }

    pub const fn csharp_name(self) -> &'static str {
        self.name()
    }

    pub const fn is_integer(self) -> bool {
        matches!(self, Self::Int | Self::UInt | Self::Long | Self::ULong)
    }

    pub const fn is_key_compatible(self) -> bool {
        matches!(
            self,
            Self::Int | Self::UInt | Self::Long | Self::ULong | Self::String
        )
    }

    pub const fn is_comparison_capable(self) -> bool {
        self.is_key_compatible()
    }

    pub const fn integer_width(self) -> Option<u32> {
        match self {
            Self::Int | Self::UInt => Some(32),
            Self::Long | Self::ULong => Some(64),
            Self::Bool | Self::Float | Self::Double | Self::String => None,
        }
    }

    pub const fn is_signed_integer(self) -> bool {
        matches!(self, Self::Int | Self::Long)
    }

    pub const fn integer_range(self) -> Option<(i128, i128)> {
        match self {
            Self::Int => Some((i32::MIN as i128, i32::MAX as i128)),
            Self::UInt => Some((0, u32::MAX as i128)),
            Self::Long => Some((i64::MIN as i128, i64::MAX as i128)),
            Self::ULong => Some((0, u64::MAX as i128)),
            Self::Bool | Self::Float | Self::Double | Self::String => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeCategory {
    ValueObject,
    Custom,
    Enum,
    Flags,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TypeReference {
    Primitive(PrimitiveType),
    Named(String),
}

impl TypeReference {
    pub fn source_name(&self) -> &str {
        match self {
            Self::Primitive(primitive) => primitive.name(),
            Self::Named(name) => name,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldModifier {
    Required,
    Nullable,
    Array,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedField {
    pub key: u32,
    pub name: String,
    pub base_type: TypeReference,
    pub modifier: FieldModifier,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolvedConversions {
    pub from_underlying_implicit: bool,
    pub to_underlying_implicit: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedEnumMember {
    pub name: String,
    pub value: IntegerLiteral,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolvedType {
    ValueObject {
        name: String,
        underlying: PrimitiveType,
        conversions: ResolvedConversions,
    },
    Custom {
        name: String,
        fields: Vec<ResolvedField>,
    },
    Enum {
        name: String,
        underlying: PrimitiveType,
        members: Vec<ResolvedEnumMember>,
    },
    Flags {
        name: String,
        underlying: PrimitiveType,
        members: Vec<ResolvedEnumMember>,
    },
}

impl ResolvedType {
    pub fn name(&self) -> &str {
        match self {
            Self::ValueObject { name, .. }
            | Self::Custom { name, .. }
            | Self::Enum { name, .. }
            | Self::Flags { name, .. } => name,
        }
    }

    pub fn category(&self) -> TypeCategory {
        match self {
            Self::ValueObject { .. } => TypeCategory::ValueObject,
            Self::Custom { .. } => TypeCategory::Custom,
            Self::Enum { .. } => TypeCategory::Enum,
            Self::Flags { .. } => TypeCategory::Flags,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TypeSystem {
    pub types: BTreeMap<String, ResolvedType>,
}

impl TypeSystem {
    pub fn get(&self, name: &str) -> Option<&ResolvedType> {
        self.types.get(name)
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &ResolvedType)> {
        self.types.iter()
    }

    pub fn resolve_reference(&self, name: &str) -> Option<TypeReference> {
        PrimitiveType::parse(name)
            .map(TypeReference::Primitive)
            .or_else(|| {
                self.types
                    .contains_key(name)
                    .then(|| TypeReference::Named(name.to_owned()))
            })
    }

    pub fn is_key_compatible(&self, reference: &TypeReference) -> bool {
        match reference {
            TypeReference::Primitive(primitive) => primitive.is_key_compatible(),
            TypeReference::Named(name) => self.types.get(name).is_some_and(|resolved| {
                matches!(
                    resolved.category(),
                    TypeCategory::ValueObject | TypeCategory::Enum
                )
            }),
        }
    }

    pub fn is_comparison_capable(&self, reference: &TypeReference) -> bool {
        match reference {
            TypeReference::Primitive(primitive) => primitive.is_comparison_capable(),
            TypeReference::Named(name) => self.types.get(name).is_some_and(|resolved| {
                matches!(
                    resolved.category(),
                    TypeCategory::ValueObject | TypeCategory::Enum
                )
            }),
        }
    }

    pub fn is_field_key_compatible(
        &self,
        reference: &TypeReference,
        modifier: FieldModifier,
    ) -> bool {
        modifier == FieldModifier::Required && self.is_key_compatible(reference)
    }

    pub fn is_field_comparison_capable(
        &self,
        reference: &TypeReference,
        modifier: FieldModifier,
    ) -> bool {
        modifier == FieldModifier::Required && self.is_comparison_capable(reference)
    }

    pub fn resolve_flags_value(&self, type_name: &str, value: &Value) -> Result<u128> {
        let reference = self.resolve_reference(type_name).ok_or_else(|| {
            type_error(
                "E-TYPE-UNKNOWN-REFERENCE",
                format!("unknown type `{type_name}`"),
            )
        })?;
        match reference {
            TypeReference::Named(name) => match self.types.get(&name) {
                Some(ResolvedType::Flags {
                    underlying,
                    members,
                    ..
                }) => resolve_flags_value(*underlying, members, value),
                _ => Err(type_error(
                    "E-FLAGS-NOT-FLAGS-TYPE",
                    format!("`{type_name}` is not a Flags Enum"),
                )),
            },
            TypeReference::Primitive(_) => Err(type_error(
                "E-FLAGS-NOT-FLAGS-TYPE",
                format!("`{type_name}` is not a Flags Enum"),
            )),
        }
    }

    /// Validate a value against a resolved type. Table record integration is
    /// deliberately a later slice, but this reusable validator gives the
    /// Type System its own strict scalar, Enum, Flags, and Custom semantics.
    pub fn validate_value(&self, type_name: &str, value: &Value) -> Result<()> {
        let reference = self.resolve_reference(type_name).ok_or_else(|| {
            type_error(
                "E-TYPE-UNKNOWN-REFERENCE",
                format!("unknown type `{type_name}`"),
            )
        })?;
        self.validate_reference_value(&reference, value)
    }

    pub fn validate_reference_value(&self, reference: &TypeReference, value: &Value) -> Result<()> {
        match reference {
            TypeReference::Primitive(primitive) => validate_primitive_value(*primitive, value),
            TypeReference::Named(name) => match self.types.get(name) {
                Some(ResolvedType::ValueObject { underlying, .. }) => {
                    validate_primitive_value(*underlying, value)
                }
                Some(ResolvedType::Enum { members, .. }) => validate_enum_value(members, value),
                Some(ResolvedType::Flags {
                    underlying,
                    members,
                    ..
                }) => resolve_flags_value(*underlying, members, value).map(|_| ()),
                Some(ResolvedType::Custom { fields, .. }) => {
                    self.validate_custom_mapping(fields, value)
                }
                None => Err(type_error(
                    "E-TYPE-UNKNOWN-REFERENCE",
                    format!("unknown type `{name}`"),
                )),
            },
        }
    }

    fn validate_custom_mapping(&self, fields: &[ResolvedField], value: &Value) -> Result<()> {
        let mapping = value.as_mapping().ok_or_else(|| {
            type_error(
                "E-TYPE-CUSTOM-DATA-SHAPE",
                "Custom Type data must be a mapping",
            )
        })?;

        let mut string_keys = BTreeSet::new();
        for (key, _) in mapping {
            let key = key.as_str().ok_or_else(|| {
                type_error(
                    "E-TYPE-CUSTOM-DATA-KEY",
                    "Custom Type data mapping keys must be strings",
                )
            })?;
            string_keys.insert(key);
        }

        for key in &string_keys {
            if !fields.iter().any(|field| field.name == *key) {
                return Err(type_error(
                    "E-TYPE-CUSTOM-UNKNOWN-MEMBER",
                    format!("unknown Custom Type member `{key}`"),
                ));
            }
        }

        for field in fields {
            let value = mapping
                .get(Value::String(field.name.clone()))
                .ok_or_else(|| {
                    type_error(
                        "E-TYPE-CUSTOM-MISSING-MEMBER",
                        format!("missing Custom Type member `{}`", field.name),
                    )
                })?;
            self.validate_field_value(field, value)?;
        }
        Ok(())
    }

    pub fn validate_field_value(&self, field: &ResolvedField, value: &Value) -> Result<()> {
        if value.is_null() {
            return match field.modifier {
                FieldModifier::Nullable => Ok(()),
                FieldModifier::Required | FieldModifier::Array => Err(type_error(
                    "E-TYPE-NULL-NOT-ALLOWED",
                    format!("field `{}` does not allow null", field.name),
                )),
            };
        }

        match field.modifier {
            FieldModifier::Required | FieldModifier::Nullable => {
                self.validate_reference_value(&field.base_type, value)
            }
            FieldModifier::Array => {
                let values = value.as_sequence().ok_or_else(|| {
                    type_error(
                        "E-TYPE-ARRAY-DATA-SHAPE",
                        format!("array field `{}` must be a sequence", field.name),
                    )
                })?;
                for value in values {
                    self.validate_reference_value(&field.base_type, value)?;
                }
                Ok(())
            }
        }
    }
}

#[derive(Debug)]
pub struct TypeSystemBuild {
    pub model: Option<TypeSystem>,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn resolve_type_system(documents: &ProjectDocuments) -> Result<TypeSystem> {
    let build = build_type_system(documents);
    if let Some(model) = build.model {
        return Ok(model);
    }
    let diagnostic = build.diagnostics.into_iter().next().unwrap_or_else(|| {
        Diagnostic::new(
            "E-TYPE-RESOLUTION",
            ErrorKind::Validation,
            "type system resolution failed",
        )
    });
    Err(MasterdataError {
        diagnostic: Box::new(diagnostic),
    })
}

pub fn build_type_system(documents: &ProjectDocuments) -> TypeSystemBuild {
    let type_documents: Vec<_> = documents.types().collect();
    let mut diagnostics = Vec::new();
    let mut declarations: BTreeMap<String, (&Path, &TypeDocument)> = BTreeMap::new();

    for (path, document) in &type_documents {
        if let Some(previous) = declarations.insert(document.name.clone(), (path, *document)) {
            diagnostics.push(type_diagnostic(
                "E-TYPE-DUPLICATE-NAME",
                format!(
                    "type `{}` is declared more than once (also in {})",
                    document.name,
                    previous.0.display()
                ),
                path,
                "SCHEMA-VO-004",
            ));
        }
    }

    let names: BTreeSet<String> = declarations.keys().cloned().collect();
    let mut resolved = BTreeMap::new();
    for (name, (path, document)) in &declarations {
        if document.kind != "type" {
            diagnostics.push(type_diagnostic(
                "E-TYPE-INVALID-KIND",
                format!("type declaration `{name}` must use kind `type`"),
                path,
                "SCHEMA-VO-004",
            ));
        }
        if !is_type_name(name) {
            diagnostics.push(type_diagnostic(
                "E-TYPE-INVALID-NAME",
                format!("type name `{name}` does not match the approved ASCII grammar"),
                path,
                "TYPE-NAMING-002",
            ));
        }

        let category_count = [
            document.value_object.is_some(),
            document.custom.is_some(),
            document.enum_definition.is_some(),
            document.flags.is_some(),
        ]
        .into_iter()
        .filter(|present| *present)
        .count();
        if category_count == 0 {
            diagnostics.push(type_diagnostic(
                "E-TYPE-MISSING-CATEGORY",
                format!("type `{name}` must declare one type category"),
                path,
                "SCHEMA-VO-004",
            ));
            continue;
        }
        if category_count > 1 {
            diagnostics.push(type_diagnostic(
                "E-TYPE-MULTIPLE-CATEGORIES",
                format!("type `{name}` must not declare multiple type categories"),
                path,
                "SCHEMA-VO-004",
            ));
            continue;
        }

        validate_type_name_member_collision(
            name,
            if document.value_object.is_some() {
                TypeCategory::ValueObject
            } else if document.custom.is_some() {
                TypeCategory::Custom
            } else if document.enum_definition.is_some() {
                TypeCategory::Enum
            } else {
                TypeCategory::Flags
            },
            path,
            &mut diagnostics,
        );

        let candidate = if let Some(value_object) = &document.value_object {
            resolve_value_object(name, value_object, path, &mut diagnostics)
        } else if let Some(custom) = &document.custom {
            resolve_custom(name, custom, &names, path, &mut diagnostics)
        } else if let Some(enum_definition) = &document.enum_definition {
            resolve_enum(name, enum_definition, path, &mut diagnostics).map(
                |(underlying, members)| ResolvedType::Enum {
                    name: name.clone(),
                    underlying,
                    members,
                },
            )
        } else if let Some(flags) = &document.flags {
            resolve_flags(name, flags, path, &mut diagnostics).map(|(underlying, members)| {
                ResolvedType::Flags {
                    name: name.clone(),
                    underlying,
                    members,
                }
            })
        } else {
            None
        };
        if let Some(candidate) = candidate {
            resolved.insert(name.clone(), candidate);
        }
    }

    validate_custom_cycles(&resolved, &declarations, &mut diagnostics);

    if diagnostics.is_empty() {
        TypeSystemBuild {
            model: Some(TypeSystem { types: resolved }),
            diagnostics,
        }
    } else {
        TypeSystemBuild {
            model: None,
            diagnostics,
        }
    }
}

fn validate_type_name_member_collision(
    name: &str,
    category: TypeCategory,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let collides = match category {
        TypeCategory::ValueObject => {
            matches!(
                name,
                "Value" | "Equals" | "GetHashCode" | "ToString" | "CompareTo"
            )
        }
        TypeCategory::Custom => matches!(name, "Equals" | "GetHashCode" | "ToString"),
        TypeCategory::Enum | TypeCategory::Flags => false,
    };
    if collides {
        diagnostics.push(type_diagnostic(
            "E-TYPE-GENERATED-MEMBER-COLLISION",
            format!("type `{name}` collides with a generated member of its declaration"),
            path,
            "TYPE-NAMING-007",
        ));
    }
}

fn resolve_value_object(
    name: &str,
    value_object: &ValueObjectDefinition,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<ResolvedType> {
    let underlying = PrimitiveType::parse(&value_object.underlying);
    let Some(underlying) = underlying else {
        diagnostics.push(type_diagnostic(
            "E-VO-INVALID-UNDERLYING",
            format!(
                "Value Object `{name}` has unsupported underlying `{}`",
                value_object.underlying
            ),
            path,
            "SCHEMA-VO-002",
        ));
        return None;
    };
    if !underlying.is_key_compatible() {
        diagnostics.push(type_diagnostic(
            "E-VO-INVALID-UNDERLYING",
            format!(
                "Value Object `{name}` underlying `{}` is not key-compatible",
                underlying.name()
            ),
            path,
            "SCHEMA-VO-002",
        ));
        return None;
    }
    Some(ResolvedType::ValueObject {
        name: name.to_owned(),
        underlying,
        conversions: resolve_conversions(&value_object.conversions),
    })
}

fn resolve_conversions(conversions: &ConversionDefinition) -> ResolvedConversions {
    ResolvedConversions {
        from_underlying_implicit: conversions.from_underlying_implicit,
        to_underlying_implicit: conversions.to_underlying_implicit,
    }
}

fn resolve_custom(
    name: &str,
    custom: &crate::document::CustomTypeDefinition,
    known_names: &BTreeSet<String>,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<ResolvedType> {
    if custom.fields.is_empty() {
        diagnostics.push(type_diagnostic(
            "E-CUSTOM-NO-FIELDS",
            format!("Custom Type `{name}` must have at least one field"),
            path,
            "SCHEMA-CUSTOM-002",
        ));
        return None;
    }

    let mut keys = BTreeSet::new();
    let mut field_names = BTreeSet::new();
    let mut fields = Vec::with_capacity(custom.fields.len());
    for field in &custom.fields {
        if !keys.insert(field.key) {
            diagnostics.push(type_diagnostic(
                "E-TYPE-DUPLICATE-FIELD-KEY",
                format!(
                    "Custom Type `{name}` has duplicate MessagePack key {}",
                    field.key
                ),
                path,
                "SCHEMA-KEY-001",
            ));
        }
        if !field_names.insert(field.name.clone()) {
            diagnostics.push(type_diagnostic(
                "E-CUSTOM-DUPLICATE-FIELD-NAME",
                format!("Custom Type `{name}` has duplicate field `{}`", field.name),
                path,
                "SCHEMA-CUSTOM-003",
            ));
        }
        validate_field_name(&field.name, name, path, diagnostics);
        let base_type = resolve_field_reference(&field.type_name, known_names, path, diagnostics);
        if field.nullable && field.array {
            diagnostics.push(type_diagnostic(
                "E-TYPE-INVALID-MODIFIERS",
                format!("field `{}` cannot be both nullable and array", field.name),
                path,
                "TYPE-FIELD-002",
            ));
        }
        if let Some(base_type) = base_type {
            let property = uppercase_first_ascii(&field.name);
            if property == name
                || matches!(property.as_str(), "Equals" | "GetHashCode" | "ToString")
            {
                diagnostics.push(type_diagnostic(
                    "E-TYPE-GENERATED-MEMBER-COLLISION",
                    format!(
                        "Custom Type `{name}` field `{}` collides with generated member `{property}`",
                        field.name
                    ),
                    path,
                    "TYPE-NAMING-007",
                ));
            }
            fields.push(ResolvedField {
                key: field.key,
                name: field.name.clone(),
                base_type,
                modifier: modifier(field),
            });
        }
    }

    Some(ResolvedType::Custom {
        name: name.to_owned(),
        fields,
    })
}

fn modifier(field: &TypeFieldDefinition) -> FieldModifier {
    if field.nullable {
        FieldModifier::Nullable
    } else if field.array {
        FieldModifier::Array
    } else {
        FieldModifier::Required
    }
}

fn resolve_field_reference(
    type_name: &str,
    known_names: &BTreeSet<String>,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<TypeReference> {
    if let Some(primitive) = PrimitiveType::parse(type_name) {
        return Some(TypeReference::Primitive(primitive));
    }
    if known_names.contains(type_name) {
        return Some(TypeReference::Named(type_name.to_owned()));
    }
    diagnostics.push(type_diagnostic(
        "E-TYPE-UNKNOWN-REFERENCE",
        format!("unknown type reference `{type_name}`"),
        path,
        "SCHEMA-CUSTOM-004",
    ));
    None
}

fn resolve_enum(
    name: &str,
    definition: &EnumDefinition,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<(PrimitiveType, Vec<ResolvedEnumMember>)> {
    resolve_integer_members(
        name,
        &definition.underlying,
        &definition.members,
        path,
        diagnostics,
        false,
    )
}

fn resolve_flags(
    name: &str,
    definition: &FlagsDefinition,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<(PrimitiveType, Vec<ResolvedEnumMember>)> {
    let result = resolve_integer_members(
        name,
        &definition.underlying,
        &definition.members,
        path,
        diagnostics,
        true,
    );
    let (underlying, members) = result?;

    let zero_members: Vec<_> = members
        .iter()
        .filter(|member| member.value.0 == 0)
        .collect();
    if zero_members.len() != 1
        || zero_members[0].name != "None"
        || members
            .iter()
            .filter(|member| member.name == "None")
            .count()
            != 1
    {
        diagnostics.push(type_diagnostic(
            "E-FLAGS-NONE",
            format!("Flags Enum `{name}` must contain exactly `None = 0`"),
            path,
            "SCHEMA-FLAGS-002",
        ));
    }

    for member in &members {
        if member.value.0 == 0 {
            continue;
        }
        if !is_single_set_bit(member.value.0, underlying) {
            diagnostics.push(type_diagnostic(
                "E-FLAGS-NON_ATOMIC-MEMBER",
                format!(
                    "Flags member `{}` in `{name}` is not exactly one set bit",
                    member.name
                ),
                path,
                "SCHEMA-FLAGS-002",
            ));
        }
    }
    Some((underlying, members))
}

fn resolve_integer_members(
    name: &str,
    underlying_name: &str,
    members: &[EnumMember],
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
    flags: bool,
) -> Option<(PrimitiveType, Vec<ResolvedEnumMember>)> {
    let underlying = PrimitiveType::parse(underlying_name);
    let Some(underlying) = underlying else {
        diagnostics.push(type_diagnostic(
            "E-ENUM-INVALID-UNDERLYING",
            format!("Enum `{name}` has unsupported underlying `{underlying_name}`"),
            path,
            "SCHEMA-ENUM-002",
        ));
        return None;
    };
    if !underlying.is_integer() {
        diagnostics.push(type_diagnostic(
            "E-ENUM-INVALID-UNDERLYING",
            format!("Enum `{name}` underlying must be int, uint, long, or ulong"),
            path,
            "SCHEMA-ENUM-002",
        ));
        return None;
    }
    if members.is_empty() && !flags {
        diagnostics.push(type_diagnostic(
            "E-ENUM-NO-MEMBERS",
            format!("Enum `{name}` must have at least one member"),
            path,
            "SCHEMA-ENUM-004",
        ));
    }

    let mut names = BTreeSet::new();
    let mut values = BTreeSet::new();
    let mut resolved = Vec::with_capacity(members.len());
    for member in members {
        if !names.insert(member.name.clone()) {
            diagnostics.push(type_diagnostic(
                "E-ENUM-DUPLICATE-MEMBER-NAME",
                format!("Enum `{name}` has duplicate member `{}`", member.name),
                path,
                "SCHEMA-ENUM-003",
            ));
        }
        if !values.insert(member.value) {
            diagnostics.push(type_diagnostic(
                "E-ENUM-DUPLICATE-VALUE",
                format!(
                    "Enum `{name}` has duplicate numeric value {}",
                    member.value.0
                ),
                path,
                "SCHEMA-ENUM-003",
            ));
        }
        validate_member_name(&member.name, name, path, diagnostics);
        if let Some((minimum, maximum)) = underlying.integer_range()
            && !(minimum..=maximum).contains(&member.value.0)
        {
            diagnostics.push(type_diagnostic(
                "E-ENUM-VALUE-OUT-OF-RANGE",
                format!(
                    "Enum member `{}` value {} is outside `{}` range",
                    member.name,
                    member.value.0,
                    underlying.name()
                ),
                path,
                "SCHEMA-ENUM-003",
            ));
        }
        if member.name == name {
            diagnostics.push(type_diagnostic(
                "E-TYPE-GENERATED-MEMBER-COLLISION",
                format!(
                    "Enum member `{}` collides with its enclosing type",
                    member.name
                ),
                path,
                "SCHEMA-ENUM-006",
            ));
        }
        resolved.push(ResolvedEnumMember {
            name: member.name.clone(),
            value: member.value,
        });
    }
    Some((underlying, resolved))
}

fn validate_custom_cycles(
    resolved: &BTreeMap<String, ResolvedType>,
    declarations: &BTreeMap<String, (&Path, &TypeDocument)>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let mut state = BTreeMap::<String, VisitState>::new();
    for name in resolved.keys() {
        if !matches!(state.get(name), Some(VisitState::Done)) {
            visit_custom(name, resolved, declarations, &mut state, diagnostics);
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VisitState {
    Visiting,
    Done,
}

fn visit_custom(
    name: &str,
    resolved: &BTreeMap<String, ResolvedType>,
    declarations: &BTreeMap<String, (&Path, &TypeDocument)>,
    state: &mut BTreeMap<String, VisitState>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if matches!(state.get(name), Some(VisitState::Visiting)) {
        let path = declarations
            .get(name)
            .map(|(path, _)| *path)
            .unwrap_or_else(|| Path::new("<type>"));
        diagnostics.push(type_diagnostic(
            "E-CUSTOM-RECURSION",
            format!("Custom Type dependency cycle includes `{name}`"),
            path,
            "SCHEMA-CUSTOM-009",
        ));
        return;
    }
    if matches!(state.get(name), Some(VisitState::Done)) {
        return;
    }
    state.insert(name.to_owned(), VisitState::Visiting);
    if let Some(ResolvedType::Custom { fields, .. }) = resolved.get(name) {
        for field in fields {
            if let TypeReference::Named(target) = &field.base_type
                && matches!(resolved.get(target), Some(ResolvedType::Custom { .. }))
            {
                visit_custom(target, resolved, declarations, state, diagnostics);
            }
        }
    }
    state.insert(name.to_owned(), VisitState::Done);
}

fn validate_field_name(
    field_name: &str,
    owner: &str,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !is_field_name(field_name) || is_csharp_reserved_keyword(field_name) {
        diagnostics.push(type_diagnostic(
            "E-TYPE-INVALID-FIELD-NAME",
            format!("field name `{field_name}` in `{owner}` is not a valid C# source name"),
            path,
            "TYPE-NAMING-003",
        ));
    }
}

fn validate_member_name(
    member_name: &str,
    owner: &str,
    path: &Path,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !is_type_name(member_name) || is_csharp_reserved_keyword(member_name) {
        diagnostics.push(type_diagnostic(
            "E-TYPE-INVALID-MEMBER-NAME",
            format!("member name `{member_name}` in `{owner}` is not a valid C# identifier"),
            path,
            "SCHEMA-ENUM-006",
        ));
    }
}

fn is_type_name(value: &str) -> bool {
    let mut chars = value.chars();
    matches!(chars.next(), Some(first) if first.is_ascii_uppercase())
        && chars.all(|character| character.is_ascii_alphanumeric())
}

fn is_field_name(value: &str) -> bool {
    let mut chars = value.chars();
    matches!(chars.next(), Some(first) if first.is_ascii_lowercase())
        && chars.all(|character| character.is_ascii_alphanumeric())
}

fn uppercase_first_ascii(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => first.to_ascii_uppercase().to_string() + chars.as_str(),
        None => String::new(),
    }
}

/// This is intentionally the C# reserved-keyword set, not the contextual
/// keyword set. Contextual words may be legal source identifiers in this
/// contract; generated-member collision validation remains independent.
pub fn is_csharp_reserved_keyword(value: &str) -> bool {
    matches!(
        value,
        "abstract"
            | "as"
            | "base"
            | "bool"
            | "break"
            | "byte"
            | "case"
            | "catch"
            | "char"
            | "checked"
            | "class"
            | "const"
            | "continue"
            | "decimal"
            | "default"
            | "delegate"
            | "do"
            | "double"
            | "else"
            | "enum"
            | "event"
            | "explicit"
            | "extern"
            | "false"
            | "finally"
            | "fixed"
            | "float"
            | "for"
            | "foreach"
            | "goto"
            | "if"
            | "implicit"
            | "in"
            | "int"
            | "interface"
            | "internal"
            | "is"
            | "lock"
            | "long"
            | "namespace"
            | "new"
            | "null"
            | "object"
            | "operator"
            | "out"
            | "override"
            | "params"
            | "private"
            | "protected"
            | "public"
            | "readonly"
            | "ref"
            | "return"
            | "sbyte"
            | "sealed"
            | "short"
            | "sizeof"
            | "stackalloc"
            | "static"
            | "string"
            | "struct"
            | "switch"
            | "this"
            | "throw"
            | "true"
            | "try"
            | "typeof"
            | "uint"
            | "ulong"
            | "unchecked"
            | "unsafe"
            | "ushort"
            | "using"
            | "virtual"
            | "void"
            | "volatile"
            | "while"
    )
}

fn is_single_set_bit(value: i128, underlying: PrimitiveType) -> bool {
    // Convert signed values to the declared fixed-width bit pattern so the
    // signed highest bit remains a valid atomic flag (SCHEMA-FLAGS-002).
    let Some(width) = underlying.integer_width() else {
        return false;
    };
    let bits = if value < 0 {
        let modulus = 1_i128 << width;
        (value + modulus) as u128
    } else {
        value as u128
    };
    bits.count_ones() == 1
}

fn validate_primitive_value(primitive: PrimitiveType, value: &Value) -> Result<()> {
    match primitive {
        PrimitiveType::Bool if matches!(value, Value::Bool(_)) => Ok(()),
        PrimitiveType::String if matches!(value, Value::String(_)) => Ok(()),
        PrimitiveType::Int | PrimitiveType::UInt | PrimitiveType::Long | PrimitiveType::ULong => {
            let number = match value {
                Value::Number(number) if number.is_i64() || number.is_u64() => number,
                _ => {
                    return Err(type_error(
                        "E-TYPE-INVALID-SCALAR",
                        format!("`{}` requires an integer scalar", primitive.name()),
                    ));
                }
            };
            let integer = number
                .as_i64()
                .map(i128::from)
                .or_else(|| number.as_u64().map(i128::from))
                .expect("integer number was checked above");
            let (minimum, maximum) = primitive.integer_range().expect("integer primitive");
            if (minimum..=maximum).contains(&integer) {
                Ok(())
            } else {
                Err(type_error(
                    "E-TYPE-INTEGER-OUT-OF-RANGE",
                    format!("value {integer} is outside `{}` range", primitive.name()),
                ))
            }
        }
        PrimitiveType::Float | PrimitiveType::Double => match value {
            Value::Number(number) if number.is_f64() => {
                let value = number.as_f64().expect("f64 number was checked above");
                if value.is_finite() {
                    Ok(())
                } else {
                    Err(type_error(
                        "E-TYPE-NONFINITE-FLOAT",
                        format!("`{}` requires a finite value", primitive.name()),
                    ))
                }
            }
            _ => Err(type_error(
                "E-TYPE-INVALID-SCALAR",
                format!("`{}` requires a floating-point scalar", primitive.name()),
            )),
        },
        PrimitiveType::Bool | PrimitiveType::String => Err(type_error(
            "E-TYPE-INVALID-SCALAR",
            format!("value does not match `{}`", primitive.name()),
        )),
    }
}

fn validate_enum_value(members: &[ResolvedEnumMember], value: &Value) -> Result<()> {
    let name = value.as_str().ok_or_else(|| {
        type_error(
            "E-ENUM-DATA-NOT-SYMBOLIC",
            "normal Enum data must use a symbolic member name",
        )
    })?;
    if members.iter().any(|member| member.name == name) {
        Ok(())
    } else {
        Err(type_error(
            "E-ENUM-UNKNOWN-MEMBER",
            format!("unknown Enum member `{name}`"),
        ))
    }
}

fn resolve_flags_value(
    underlying: PrimitiveType,
    members: &[ResolvedEnumMember],
    value: &Value,
) -> Result<u128> {
    let values = value.as_sequence().ok_or_else(|| {
        type_error(
            "E-FLAGS-DATA-NOT-SEQUENCE",
            "Flags data must be a sequence of symbolic member names",
        )
    })?;
    if values.is_empty() {
        return Err(type_error(
            "E-FLAGS-EMPTY-DATA",
            "Flags data must use [None] for its zero value",
        ));
    }
    let mut seen = BTreeSet::new();
    let mut has_none = false;
    let mut has_nonzero = false;
    let mut bits = 0_u128;
    for value in values {
        let name = value.as_str().ok_or_else(|| {
            type_error(
                "E-FLAGS-DATA-NOT-SYMBOLIC",
                "Flags data members must be symbolic strings",
            )
        })?;
        let member = members
            .iter()
            .find(|member| member.name == name)
            .ok_or_else(|| {
                type_error(
                    "E-FLAGS-UNKNOWN-MEMBER",
                    format!("unknown Flags member `{name}`"),
                )
            })?;
        if !seen.insert(name.to_owned()) {
            return Err(type_error(
                "E-FLAGS-DUPLICATE-MEMBER",
                format!("Flags member `{name}` occurs more than once"),
            ));
        }
        if name == "None" {
            has_none = true;
        } else if member.value.0 != 0 {
            has_nonzero = true;
            bits |= integer_bits(member.value.0, underlying);
        }
    }
    if !has_none && !has_nonzero {
        return Err(type_error(
            "E-FLAGS-EMPTY-DATA",
            "Flags data must contain [None] or at least one nonzero member",
        ));
    }
    if has_none && has_nonzero {
        return Err(type_error(
            "E-FLAGS-NONE-MIXED",
            "None cannot be combined with a nonzero Flags member",
        ));
    }
    Ok(bits)
}

fn integer_bits(value: i128, underlying: PrimitiveType) -> u128 {
    let width = underlying
        .integer_width()
        .expect("integer bits require an integer primitive");
    if value < 0 {
        (value + (1_i128 << width)) as u128
    } else {
        value as u128
    }
}

fn type_error(code: &str, message: impl Into<String>) -> MasterdataError {
    MasterdataError::new(code, ErrorKind::Validation, message)
}

fn type_diagnostic(
    code: &str,
    message: impl Into<String>,
    path: &Path,
    requirement: &str,
) -> Diagnostic {
    Diagnostic::new(code, ErrorKind::Validation, message)
        .with_source(path.to_path_buf())
        .with_related_requirement(requirement)
}
