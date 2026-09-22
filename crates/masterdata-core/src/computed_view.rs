//! Shared semantics for authoring-only Computed Views.
//!
//! A view is intentionally resolved beside the existing Table/Type model but
//! never lowered into a build artifact.  The parser and evaluator live here so
//! Overview, query, migration, and future adapters cannot grow independent
//! expression semantics.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use crate::authoring_value::AuthoringValue;
use crate::document::ProjectDocuments;
use crate::error::{Diagnostic, ErrorKind, MasterdataError, Result};
use crate::type_system::{
    FieldModifier, PrimitiveType, ResolvedAuthoringField, ResolvedAuthoringType, ResolvedType,
    build_type_system,
};

#[derive(Debug, Clone, PartialEq)]
pub struct ComputedViewBuild {
    pub views: Vec<ResolvedComputedView>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedComputedView {
    pub name: String,
    pub table: String,
    pub source: PathBuf,
    pub columns: Vec<ResolvedComputedColumn>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedComputedColumn {
    pub name: String,
    pub expression: String,
    pub shape: ResolvedAuthoringField,
    ast: TypedExpression,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpressionSpan {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq)]
struct TypedExpression {
    span: ExpressionSpan,
    ty: ExpressionType,
    kind: TypedExpressionKind,
}

#[derive(Debug, Clone, PartialEq)]
enum TypedExpressionKind {
    Null,
    Bool(bool),
    Integer(String),
    Float(String),
    String(String),
    Field(String),
    EnumLiteral {
        type_name: String,
        member: String,
    },
    Unary {
        operator: UnaryOperator,
        operand: Box<TypedExpression>,
    },
    Binary {
        operator: BinaryOperator,
        left: Box<TypedExpression>,
        right: Box<TypedExpression>,
    },
    Conditional {
        condition: Box<TypedExpression>,
        when_true: Box<TypedExpression>,
        when_false: Box<TypedExpression>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UnaryOperator {
    Not,
    Plus,
    Minus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BinaryOperator {
    Coalesce,
    Or,
    And,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ScalarType {
    Primitive(PrimitiveType),
    ValueObject {
        name: String,
        underlying: PrimitiveType,
    },
    Enum {
        name: String,
        underlying: PrimitiveType,
        members: BTreeSet<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ExpressionType {
    Null,
    IntegerLiteral,
    FloatLiteral,
    Scalar { scalar: ScalarType, nullable: bool },
}

#[derive(Debug, Clone)]
struct FieldExpressionInfo {
    ty: Option<ExpressionType>,
}

#[derive(Debug, Clone)]
struct ExpressionError {
    code: &'static str,
    message: String,
    span: ExpressionSpan,
    kind: ErrorKind,
}

impl ExpressionError {
    fn parse(message: impl Into<String>, span: ExpressionSpan) -> Self {
        Self {
            code: "E-VIEW-EXPRESSION-SYNTAX",
            message: message.into(),
            span,
            kind: ErrorKind::Parse,
        }
    }

    fn type_error(code: &'static str, message: impl Into<String>, span: ExpressionSpan) -> Self {
        Self {
            code,
            message: message.into(),
            span,
            kind: ErrorKind::Validation,
        }
    }
}

/// Resolve all persisted view documents without touching filesystem state.
/// Invalid view documents remain isolated in diagnostics while valid views
/// continue to be available to an Overview of another target table.
pub fn resolve_computed_views(documents: &ProjectDocuments) -> ComputedViewBuild {
    let mut loaded = documents
        .views()
        .map(|(path, view)| (path.clone(), view.clone()))
        .collect::<Vec<_>>();
    loaded.sort_by(|left, right| left.0.cmp(&right.0));

    let mut diagnostics = Vec::new();
    let mut names = BTreeMap::<String, PathBuf>::new();
    let mut views = Vec::new();
    let enum_types = enum_types(documents);
    for (path, view) in loaded {
        if view.kind != "view" {
            diagnostics.push(view_diagnostic(
                &path,
                "E-VIEW-PARSE",
                format!(
                    "view document must declare kind `view`, found `{}`",
                    view.kind
                ),
                "ADV-VIEW-001",
                None,
            ));
            continue;
        }
        if !is_lower_camel_identifier(&view.name) {
            diagnostics.push(view_diagnostic(
                &path,
                "E-VIEW-DUPLICATE-NAME",
                format!(
                    "view name `{}` is not a lowerCamel ASCII identifier",
                    view.name
                ),
                "ADV-VIEW-002",
                Some("name"),
            ));
        }
        if let Some(previous) = names.insert(view.name.clone(), path.clone()) {
            diagnostics.push(
                view_diagnostic(
                    &path,
                    "E-VIEW-DUPLICATE-NAME",
                    format!(
                        "view `{}` is declared more than once (also in {})",
                        view.name,
                        previous.display()
                    ),
                    "ADV-VIEW-002",
                    Some("name"),
                )
                .with_suggestion("give each project-local view a unique name"),
            );
            continue;
        }
        let schemas = documents
            .schemas()
            .filter(|(_, schema)| schema.table == view.table)
            .collect::<Vec<_>>();
        let Some((_, schema)) = schemas.first().copied() else {
            diagnostics.push(view_diagnostic(
                &path,
                "E-VIEW-TARGET-TABLE",
                format!(
                    "view `{}` refers to unknown Table `{}`",
                    view.name, view.table
                ),
                "ADV-VIEW-002",
                Some("table"),
            ));
            continue;
        };
        if schemas.len() != 1 {
            diagnostics.push(view_diagnostic(
                &path,
                "E-VIEW-TARGET-TABLE",
                format!(
                    "view `{}` target Table `{}` is ambiguous",
                    view.name, view.table
                ),
                "ADV-VIEW-002",
                Some("table"),
            ));
            continue;
        }
        if view.columns.is_empty() {
            diagnostics.push(view_diagnostic(
                &path,
                "E-VIEW-PARSE",
                format!("view `{}` must declare at least one column", view.name),
                "ADV-VIEW-001",
                Some("columns"),
            ));
            continue;
        }

        let mut fields = BTreeMap::new();
        for field in &schema.fields {
            let shape = crate::type_system::resolve_authoring_field_shape(documents, field);
            let ty = shape.as_ref().and_then(expression_type_for_shape);
            fields.insert(field.name.clone(), FieldExpressionInfo { ty });
        }
        let mut column_names = BTreeSet::new();
        let mut columns = Vec::new();
        for (index, column) in view.columns.iter().enumerate() {
            let schema_path = format!("columns[{index}]");
            if !is_lower_camel_identifier(&column.name) {
                diagnostics.push(view_diagnostic(
                    &path,
                    "E-VIEW-DUPLICATE-COLUMN",
                    format!(
                        "computed column `{}` is not a lowerCamel ASCII identifier",
                        column.name
                    ),
                    "ADV-VIEW-002",
                    Some(&schema_path),
                ));
                continue;
            }
            if !column_names.insert(column.name.clone()) {
                diagnostics.push(view_diagnostic(
                    &path,
                    "E-VIEW-DUPLICATE-COLUMN",
                    format!(
                        "computed column `{}` is declared more than once",
                        column.name
                    ),
                    "ADV-VIEW-002",
                    Some(&schema_path),
                ));
                continue;
            }
            if fields.contains_key(&column.name) {
                diagnostics.push(view_diagnostic(
                    &path,
                    "E-VIEW-COLUMN-COLLISION",
                    format!(
                        "computed column `{}` collides with a source field",
                        column.name
                    ),
                    "ADV-VIEW-002",
                    Some(&schema_path),
                ));
                continue;
            }
            let parsed = match Parser::new(&column.expression).parse() {
                Ok(expression) => expression,
                Err(error) => {
                    diagnostics.push(expression_diagnostic(&path, &schema_path, error));
                    continue;
                }
            };
            let typed = match type_check(&parsed, &fields, &enum_types) {
                Ok(expression) => expression,
                Err(error) => {
                    diagnostics.push(expression_diagnostic(&path, &schema_path, error));
                    continue;
                }
            };
            let Some(shape) = output_shape(&column.name, &typed.ty) else {
                diagnostics.push(view_diagnostic(
                    &path,
                    "E-VIEW-UNSUPPORTED-OPERAND",
                    format!(
                        "computed column `{}` does not produce a supported scalar value",
                        column.name
                    ),
                    "ADV-VIEW-003",
                    Some(&schema_path),
                ));
                continue;
            };
            columns.push(ResolvedComputedColumn {
                name: column.name.clone(),
                expression: column.expression.clone(),
                shape,
                ast: typed,
            });
        }
        if columns.len() == view.columns.len() {
            views.push(ResolvedComputedView {
                name: view.name,
                table: view.table,
                source: path,
                columns,
            });
        }
    }
    views.sort_by(|left, right| {
        left.table
            .cmp(&right.table)
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.source.cmp(&right.source))
    });
    diagnostics.sort_by(diagnostic_order);
    ComputedViewBuild { views, diagnostics }
}

pub fn validate_computed_views(documents: &ProjectDocuments) -> Vec<Diagnostic> {
    resolve_computed_views(documents).diagnostics
}

pub fn computed_views_for_table<'a>(
    build: &'a ComputedViewBuild,
    table: &str,
) -> impl Iterator<Item = &'a ResolvedComputedView> {
    build.views.iter().filter(move |view| view.table == table)
}

/// Evaluate one computed column against a typed source row.  An invalid source
/// value or row-dependent arithmetic problem becomes an Invalid authoring
/// value; it is never silently replaced with a default scalar.
pub fn evaluate_computed_column(
    column: &ResolvedComputedColumn,
    values: &BTreeMap<String, AuthoringValue>,
    source: &Path,
    record_index: usize,
) -> AuthoringValue {
    match evaluate_expression(&column.ast, values, source, record_index) {
        Ok(value) => value,
        Err(diagnostic) => AuthoringValue::Invalid { diagnostic },
    }
}

/// Return resolved source-field references in expression source order. The
/// parser distinguishes field nodes from Enum literal type/member nodes, so
/// the source-preserving RenameField patcher never guesses from raw text.
pub fn field_reference_spans(expression: &str, field: &str) -> Vec<ExpressionSpan> {
    let mut result = Vec::new();
    let Ok(parsed) = Parser::new(expression).parse() else {
        return result;
    };
    collect_field_reference_spans(&parsed.root, field, &mut result);
    result.sort_by_key(|span| span.start);
    result
}

fn collect_field_reference_spans(
    expression: &RawExpression,
    field: &str,
    result: &mut Vec<ExpressionSpan>,
) {
    match &expression.kind {
        RawExpressionKind::Field(name) if name == field => result.push(expression.span),
        RawExpressionKind::Unary { operand, .. } => {
            collect_field_reference_spans(operand, field, result)
        }
        RawExpressionKind::Binary { left, right, .. } => {
            collect_field_reference_spans(left, field, result);
            collect_field_reference_spans(right, field, result);
        }
        RawExpressionKind::Conditional {
            condition,
            when_true,
            when_false,
        } => {
            collect_field_reference_spans(condition, field, result);
            collect_field_reference_spans(when_true, field, result);
            collect_field_reference_spans(when_false, field, result);
        }
        RawExpressionKind::Null
        | RawExpressionKind::Bool(_)
        | RawExpressionKind::Integer(_)
        | RawExpressionKind::Float(_)
        | RawExpressionKind::String(_)
        | RawExpressionKind::Field(_)
        | RawExpressionKind::EnumLiteral { .. } => {}
    }
}

pub fn rename_field_references(expression: &str, old: &str, new: &str) -> Option<String> {
    let spans = field_reference_spans(expression, old);
    if spans.is_empty() {
        return None;
    }
    let mut result = expression.to_owned();
    for span in spans.into_iter().rev() {
        result.replace_range(span.start..span.end, new);
    }
    Some(result)
}

pub fn parse_expression(expression: &str) -> Result<()> {
    Parser::new(expression)
        .parse()
        .map(|_| ())
        .map_err(|error| MasterdataError::new(error.code, error.kind, error.message))
}

pub(crate) fn expression_type_names(expression: &str) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    let Ok(parsed) = Parser::new(expression).parse() else {
        return names;
    };
    collect_expression_type_names(&parsed.root, &mut names);
    names
}

fn collect_expression_type_names(expression: &RawExpression, names: &mut BTreeSet<String>) {
    match &expression.kind {
        RawExpressionKind::EnumLiteral { type_name, .. } => {
            names.insert(type_name.clone());
        }
        RawExpressionKind::Unary { operand, .. } => collect_expression_type_names(operand, names),
        RawExpressionKind::Binary { left, right, .. } => {
            collect_expression_type_names(left, names);
            collect_expression_type_names(right, names);
        }
        RawExpressionKind::Conditional {
            condition,
            when_true,
            when_false,
        } => {
            collect_expression_type_names(condition, names);
            collect_expression_type_names(when_true, names);
            collect_expression_type_names(when_false, names);
        }
        RawExpressionKind::Null
        | RawExpressionKind::Bool(_)
        | RawExpressionKind::Integer(_)
        | RawExpressionKind::Float(_)
        | RawExpressionKind::String(_)
        | RawExpressionKind::Field(_) => {}
    }
}

fn expression_type_for_shape(shape: &ResolvedAuthoringField) -> Option<ExpressionType> {
    let scalar = match &shape.shape {
        ResolvedAuthoringType::Primitive { primitive } => ScalarType::Primitive(*primitive),
        ResolvedAuthoringType::ValueObject { name, underlying } => ScalarType::ValueObject {
            name: name.clone(),
            underlying: *underlying,
        },
        ResolvedAuthoringType::Enum {
            name,
            underlying,
            members,
        } => ScalarType::Enum {
            name: name.clone(),
            underlying: *underlying,
            members: members.iter().cloned().collect(),
        },
        ResolvedAuthoringType::Flags { .. } | ResolvedAuthoringType::Custom { .. } => return None,
    };
    if shape.modifier == FieldModifier::Array {
        return None;
    }
    Some(ExpressionType::Scalar {
        scalar,
        nullable: shape.modifier == FieldModifier::Nullable,
    })
}

fn enum_types(documents: &ProjectDocuments) -> BTreeMap<String, ScalarType> {
    let Some(type_system) = build_type_system(documents).model else {
        return BTreeMap::new();
    };
    type_system
        .iter()
        .filter_map(|(name, resolved)| {
            let ResolvedType::Enum {
                underlying,
                members,
                ..
            } = resolved
            else {
                return None;
            };
            Some((
                name.clone(),
                ScalarType::Enum {
                    name: name.clone(),
                    underlying: *underlying,
                    members: members.iter().map(|member| member.name.clone()).collect(),
                },
            ))
        })
        .collect()
}

fn output_shape(name: &str, ty: &ExpressionType) -> Option<ResolvedAuthoringField> {
    let ty = normalize_type(ty)?;
    let (type_name, shape) = match ty.scalar {
        ScalarType::Primitive(primitive) => (
            primitive.name().to_owned(),
            ResolvedAuthoringType::Primitive { primitive },
        ),
        ScalarType::ValueObject { name, underlying } => (
            name.clone(),
            ResolvedAuthoringType::ValueObject { name, underlying },
        ),
        ScalarType::Enum {
            name,
            underlying,
            members,
        } => (
            name.clone(),
            ResolvedAuthoringType::Enum {
                name,
                underlying,
                members: members.into_iter().collect(),
            },
        ),
    };
    Some(ResolvedAuthoringField {
        name: name.to_owned(),
        type_name,
        modifier: if ty.nullable {
            FieldModifier::Nullable
        } else {
            FieldModifier::Required
        },
        shape,
    })
}

fn normalize_type(ty: &ExpressionType) -> Option<ConcreteType> {
    match ty {
        ExpressionType::Null => None,
        ExpressionType::IntegerLiteral => Some(ConcreteType {
            scalar: ScalarType::Primitive(PrimitiveType::Int),
            nullable: false,
        }),
        ExpressionType::FloatLiteral => Some(ConcreteType {
            scalar: ScalarType::Primitive(PrimitiveType::Double),
            nullable: false,
        }),
        ExpressionType::Scalar { scalar, nullable } => Some(ConcreteType {
            scalar: scalar.clone(),
            nullable: *nullable,
        }),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ConcreteType {
    scalar: ScalarType,
    nullable: bool,
}

fn type_check(
    expression: &Expression,
    fields: &BTreeMap<String, FieldExpressionInfo>,
    enum_types: &BTreeMap<String, ScalarType>,
) -> std::result::Result<TypedExpression, ExpressionError> {
    type_check_node(&expression.root, fields, enum_types)
}

fn type_check_node(
    node: &RawExpression,
    fields: &BTreeMap<String, FieldExpressionInfo>,
    enum_types: &BTreeMap<String, ScalarType>,
) -> std::result::Result<TypedExpression, ExpressionError> {
    let typed = match &node.kind {
        RawExpressionKind::Null => TypedExpression {
            span: node.span,
            ty: ExpressionType::Null,
            kind: TypedExpressionKind::Null,
        },
        RawExpressionKind::Bool(value) => TypedExpression {
            span: node.span,
            ty: scalar_type(PrimitiveType::Bool, false),
            kind: TypedExpressionKind::Bool(*value),
        },
        RawExpressionKind::Integer(value) => TypedExpression {
            span: node.span,
            ty: ExpressionType::IntegerLiteral,
            kind: TypedExpressionKind::Integer(value.clone()),
        },
        RawExpressionKind::Float(value) => TypedExpression {
            span: node.span,
            ty: ExpressionType::FloatLiteral,
            kind: TypedExpressionKind::Float(value.clone()),
        },
        RawExpressionKind::String(value) => TypedExpression {
            span: node.span,
            ty: scalar_type(PrimitiveType::String, false),
            kind: TypedExpressionKind::String(value.clone()),
        },
        RawExpressionKind::Field(name) => {
            let Some(info) = fields.get(name) else {
                return Err(ExpressionError::type_error(
                    "E-VIEW-UNKNOWN-FIELD",
                    format!("expression refers to unknown field `{name}`"),
                    node.span,
                ));
            };
            let Some(ty) = &info.ty else {
                return Err(ExpressionError::type_error(
                    "E-VIEW-UNSUPPORTED-OPERAND",
                    format!("field `{name}` is not a supported scalar expression operand"),
                    node.span,
                ));
            };
            TypedExpression {
                span: node.span,
                ty: ty.clone(),
                kind: TypedExpressionKind::Field(name.clone()),
            }
        }
        RawExpressionKind::EnumLiteral { type_name, member } => {
            let Some(scalar) = enum_types.get(type_name).cloned() else {
                return Err(ExpressionError::type_error(
                    "E-VIEW-TYPE-MISMATCH",
                    format!("unknown normal Enum `{type_name}` in enum literal"),
                    node.span,
                ));
            };
            let ScalarType::Enum { members, .. } = &scalar else {
                return Err(ExpressionError::type_error(
                    "E-VIEW-UNSUPPORTED-OPERAND",
                    format!("`{type_name}` is not a normal Enum"),
                    node.span,
                ));
            };
            if !members.contains(member) {
                return Err(ExpressionError::type_error(
                    "E-VIEW-TYPE-MISMATCH",
                    format!("Enum `{type_name}` has no member `{member}`"),
                    node.span,
                ));
            }
            TypedExpression {
                span: node.span,
                ty: ExpressionType::Scalar {
                    scalar,
                    nullable: false,
                },
                kind: TypedExpressionKind::EnumLiteral {
                    type_name: type_name.clone(),
                    member: member.clone(),
                },
            }
        }
        RawExpressionKind::Unary { operator, operand } => {
            let operand = type_check_node(operand, fields, enum_types)?;
            let ty = match operator {
                UnaryOperator::Not => {
                    require_non_nullable_bool(&operand, node.span)?;
                    scalar_type(PrimitiveType::Bool, false)
                }
                UnaryOperator::Plus | UnaryOperator::Minus => require_numeric(&operand, node.span)?,
            };
            TypedExpression {
                span: node.span,
                ty,
                kind: TypedExpressionKind::Unary {
                    operator: *operator,
                    operand: Box::new(operand),
                },
            }
        }
        RawExpressionKind::Binary {
            operator,
            left,
            right,
        } => {
            let left = type_check_node(left, fields, enum_types)?;
            let right = type_check_node(right, fields, enum_types)?;
            let ty = binary_type(*operator, &left, &right, node.span)?;
            TypedExpression {
                span: node.span,
                ty,
                kind: TypedExpressionKind::Binary {
                    operator: *operator,
                    left: Box::new(left),
                    right: Box::new(right),
                },
            }
        }
        RawExpressionKind::Conditional {
            condition,
            when_true,
            when_false,
        } => {
            let condition = type_check_node(condition, fields, enum_types)?;
            require_non_nullable_bool(&condition, condition.span)?;
            let when_true = type_check_node(when_true, fields, enum_types)?;
            let when_false = type_check_node(when_false, fields, enum_types)?;
            let ty = merge_branch_types(&when_true, &when_false, node.span)?;
            TypedExpression {
                span: node.span,
                ty,
                kind: TypedExpressionKind::Conditional {
                    condition: Box::new(condition),
                    when_true: Box::new(when_true),
                    when_false: Box::new(when_false),
                },
            }
        }
    };
    Ok(typed)
}

fn scalar_type(primitive: PrimitiveType, nullable: bool) -> ExpressionType {
    ExpressionType::Scalar {
        scalar: ScalarType::Primitive(primitive),
        nullable,
    }
}

fn require_non_nullable_bool(
    expression: &TypedExpression,
    span: ExpressionSpan,
) -> std::result::Result<(), ExpressionError> {
    if matches!(
        &expression.ty,
        ExpressionType::Scalar {
            scalar: ScalarType::Primitive(PrimitiveType::Bool),
            nullable: false
        }
    ) {
        Ok(())
    } else {
        Err(ExpressionError::type_error(
            "E-VIEW-TYPE-MISMATCH",
            "condition and boolean operators require a non-null bool",
            span,
        ))
    }
}

fn require_numeric(
    expression: &TypedExpression,
    span: ExpressionSpan,
) -> std::result::Result<ExpressionType, ExpressionError> {
    match &expression.ty {
        ExpressionType::IntegerLiteral => Ok(ExpressionType::Scalar {
            scalar: ScalarType::Primitive(PrimitiveType::Int),
            nullable: false,
        }),
        ExpressionType::FloatLiteral => Ok(ExpressionType::Scalar {
            scalar: ScalarType::Primitive(PrimitiveType::Double),
            nullable: false,
        }),
        ExpressionType::Scalar {
            scalar: ScalarType::Primitive(primitive),
            nullable: false,
        } if is_numeric(*primitive) => Ok(expression.ty.clone()),
        _ => Err(ExpressionError::type_error(
            "E-VIEW-TYPE-MISMATCH",
            "arithmetic requires a non-null numeric primitive",
            span,
        )),
    }
}

fn binary_type(
    operator: BinaryOperator,
    left: &TypedExpression,
    right: &TypedExpression,
    span: ExpressionSpan,
) -> std::result::Result<ExpressionType, ExpressionError> {
    match operator {
        BinaryOperator::And | BinaryOperator::Or => {
            require_non_nullable_bool(left, span)?;
            require_non_nullable_bool(right, span)?;
            Ok(scalar_type(PrimitiveType::Bool, false))
        }
        BinaryOperator::Coalesce => coalesce_type(left, right, span),
        BinaryOperator::Equal | BinaryOperator::NotEqual => {
            equality_type(left, right, span).map(|_| scalar_type(PrimitiveType::Bool, false))
        }
        BinaryOperator::Less
        | BinaryOperator::LessEqual
        | BinaryOperator::Greater
        | BinaryOperator::GreaterEqual => {
            comparable_type(left, right, span).map(|_| scalar_type(PrimitiveType::Bool, false))
        }
        BinaryOperator::Add
        | BinaryOperator::Subtract
        | BinaryOperator::Multiply
        | BinaryOperator::Divide
        | BinaryOperator::Remainder => arithmetic_type(operator, left, right, span),
    }
}

fn coalesce_type(
    left: &TypedExpression,
    right: &TypedExpression,
    span: ExpressionSpan,
) -> std::result::Result<ExpressionType, ExpressionError> {
    let Some(left) = normalize_type(&left.ty) else {
        let Some(right) = normalize_type(&right.ty) else {
            return Err(ExpressionError::type_error(
                "E-VIEW-TYPE-MISMATCH",
                "null coalesce requires a scalar fallback",
                span,
            ));
        };
        return Ok(ExpressionType::Scalar {
            scalar: right.scalar,
            nullable: right.nullable,
        });
    };
    if !left.nullable {
        return Err(ExpressionError::type_error(
            "E-VIEW-TYPE-MISMATCH",
            "left side of `??` must be nullable",
            span,
        ));
    }
    let Some(right) = normalize_type(&right.ty) else {
        return Err(ExpressionError::type_error(
            "E-VIEW-TYPE-MISMATCH",
            "right side of `??` must have a scalar type",
            span,
        ));
    };
    if left.scalar != right.scalar {
        return Err(ExpressionError::type_error(
            "E-VIEW-TYPE-MISMATCH",
            "both sides of `??` must have the same semantic type",
            span,
        ));
    }
    Ok(ExpressionType::Scalar {
        scalar: left.scalar,
        nullable: right.nullable,
    })
}

fn equality_type(
    left: &TypedExpression,
    right: &TypedExpression,
    span: ExpressionSpan,
) -> std::result::Result<(), ExpressionError> {
    if matches!(left.ty, ExpressionType::Null) || matches!(right.ty, ExpressionType::Null) {
        return Ok(());
    }
    let (left, right) = compatible_literal_types(&left.ty, &right.ty, span)?;
    if left.scalar != right.scalar {
        return Err(ExpressionError::type_error(
            "E-VIEW-TYPE-MISMATCH",
            "equality operands must have the same semantic type",
            span,
        ));
    }
    Ok(())
}

fn comparable_type(
    left: &TypedExpression,
    right: &TypedExpression,
    span: ExpressionSpan,
) -> std::result::Result<(), ExpressionError> {
    let (left, right) = compatible_literal_types(&left.ty, &right.ty, span)?;
    if left.nullable || right.nullable {
        return Err(ExpressionError::type_error(
            "E-VIEW-TYPE-MISMATCH",
            "ordering comparison does not accept nullable operands; use `??` first",
            span,
        ));
    }
    match (&left.scalar, &right.scalar) {
        (ScalarType::Primitive(left), ScalarType::Primitive(right))
            if left == right && (*left == PrimitiveType::String || is_numeric(*left)) =>
        {
            Ok(())
        }
        (
            ScalarType::ValueObject {
                name: left_name,
                underlying: left_underlying,
            },
            ScalarType::ValueObject {
                name: right_name,
                underlying: right_underlying,
            },
        ) if left_name == right_name && left_underlying == right_underlying => Ok(()),
        _ => Err(ExpressionError::type_error(
            "E-VIEW-TYPE-MISMATCH",
            "ordering comparison is unsupported for these scalar operands",
            span,
        )),
    }
}

fn arithmetic_type(
    operator: BinaryOperator,
    left: &TypedExpression,
    right: &TypedExpression,
    span: ExpressionSpan,
) -> std::result::Result<ExpressionType, ExpressionError> {
    if operator == BinaryOperator::Add
        && matches!(
            left.ty,
            ExpressionType::Scalar {
                scalar: ScalarType::Primitive(PrimitiveType::String),
                nullable: false
            }
        )
        && matches!(
            right.ty,
            ExpressionType::Scalar {
                scalar: ScalarType::Primitive(PrimitiveType::String),
                nullable: false
            }
        )
    {
        return Ok(scalar_type(PrimitiveType::String, false));
    }
    let (left, right) = compatible_literal_types(&left.ty, &right.ty, span)?;
    match (&left.scalar, &right.scalar) {
        (ScalarType::Primitive(left), ScalarType::Primitive(right))
            if left == right && is_numeric(*left) =>
        {
            Ok(ExpressionType::Scalar {
                scalar: ScalarType::Primitive(*left),
                nullable: false,
            })
        }
        _ => Err(ExpressionError::type_error(
            "E-VIEW-TYPE-MISMATCH",
            "arithmetic operands must be the same non-null numeric primitive",
            span,
        )),
    }
}

fn compatible_literal_types(
    left: &ExpressionType,
    right: &ExpressionType,
    span: ExpressionSpan,
) -> std::result::Result<(ConcreteType, ConcreteType), ExpressionError> {
    let left = normalize_type(left);
    let right = normalize_type(right);
    match (left, right) {
        (Some(left), Some(right)) => Ok((left, right)),
        _ => Err(ExpressionError::type_error(
            "E-VIEW-TYPE-MISMATCH",
            "operator requires scalar operands",
            span,
        )),
    }
}

fn merge_branch_types(
    left: &TypedExpression,
    right: &TypedExpression,
    span: ExpressionSpan,
) -> std::result::Result<ExpressionType, ExpressionError> {
    if matches!(left.ty, ExpressionType::Null) {
        let right = normalize_type(&right.ty).ok_or_else(|| {
            ExpressionError::type_error(
                "E-VIEW-TYPE-MISMATCH",
                "conditional branch has no scalar type",
                span,
            )
        })?;
        return Ok(ExpressionType::Scalar {
            scalar: right.scalar,
            nullable: true,
        });
    }
    if matches!(right.ty, ExpressionType::Null) {
        let left = normalize_type(&left.ty).ok_or_else(|| {
            ExpressionError::type_error(
                "E-VIEW-TYPE-MISMATCH",
                "conditional branch has no scalar type",
                span,
            )
        })?;
        return Ok(ExpressionType::Scalar {
            scalar: left.scalar,
            nullable: true,
        });
    }
    let (left, right) = compatible_literal_types(&left.ty, &right.ty, span)?;
    if left.scalar != right.scalar {
        return Err(ExpressionError::type_error(
            "E-VIEW-TYPE-MISMATCH",
            "conditional branches must have the same semantic type",
            span,
        ));
    }
    Ok(ExpressionType::Scalar {
        scalar: left.scalar,
        nullable: left.nullable || right.nullable,
    })
}

fn evaluate_expression(
    expression: &TypedExpression,
    values: &BTreeMap<String, AuthoringValue>,
    source: &Path,
    record_index: usize,
) -> std::result::Result<AuthoringValue, Box<Diagnostic>> {
    match &expression.kind {
        TypedExpressionKind::Null => Ok(AuthoringValue::Null),
        TypedExpressionKind::Bool(value) => Ok(AuthoringValue::Bool { value: *value }),
        TypedExpressionKind::Integer(value) => {
            let valid = value
                .parse::<i128>()
                .ok()
                .is_some_and(|value| (i32::MIN as i128..=i32::MAX as i128).contains(&value));
            if valid {
                Ok(AuthoringValue::Number {
                    value: value.clone(),
                })
            } else {
                Err(arithmetic_diagnostic(
                    source,
                    record_index,
                    expression.span,
                    "integer literal is outside the v1 int range",
                ))
            }
        }
        TypedExpressionKind::Float(value) => {
            if value.parse::<f64>().ok().is_some_and(f64::is_finite) {
                Ok(AuthoringValue::Number {
                    value: value.clone(),
                })
            } else {
                Err(arithmetic_diagnostic(
                    source,
                    record_index,
                    expression.span,
                    "floating literal is not finite",
                ))
            }
        }
        TypedExpressionKind::String(value) => Ok(AuthoringValue::String {
            value: value.clone(),
        }),
        TypedExpressionKind::Field(name) => match values.get(name) {
            Some(AuthoringValue::Invalid { diagnostic }) => Err(row_diagnostic(
                source,
                record_index,
                expression.span,
                format!("source field `{name}` is invalid: {}", diagnostic.message),
            )),
            Some(AuthoringValue::Null)
                if matches!(
                    expression.ty,
                    ExpressionType::Scalar {
                        nullable: false,
                        ..
                    }
                ) =>
            {
                Err(row_diagnostic(
                    source,
                    record_index,
                    expression.span,
                    format!("required source field `{name}` is null"),
                ))
            }
            Some(value)
                if !matches!(value, AuthoringValue::Null)
                    && !runtime_value_matches_type(&expression.ty, value) =>
            {
                Err(row_diagnostic(
                    source,
                    record_index,
                    expression.span,
                    format!("source field `{name}` has an invalid value shape"),
                ))
            }
            Some(value) => Ok(value.clone()),
            None => Err(row_diagnostic(
                source,
                record_index,
                expression.span,
                format!("source field `{name}` is unavailable for this record"),
            )),
        },
        TypedExpressionKind::EnumLiteral { member, .. } => Ok(AuthoringValue::String {
            value: member.clone(),
        }),
        TypedExpressionKind::Unary { operator, operand } => {
            let value = evaluate_expression(operand, values, source, record_index)?;
            match operator {
                UnaryOperator::Not => match value {
                    AuthoringValue::Bool { value } => Ok(AuthoringValue::Bool { value: !value }),
                    AuthoringValue::Null => Err(row_diagnostic(
                        source,
                        record_index,
                        expression.span,
                        "boolean negation received null",
                    )),
                    _ => Err(row_diagnostic(
                        source,
                        record_index,
                        expression.span,
                        "boolean negation received a non-bool value",
                    )),
                },
                UnaryOperator::Plus => Ok(value),
                UnaryOperator::Minus => numeric_unary(
                    value,
                    true,
                    &expression.ty,
                    source,
                    record_index,
                    expression.span,
                ),
            }
        }
        TypedExpressionKind::Binary {
            operator,
            left,
            right,
        } => evaluate_binary(
            *operator,
            left,
            right,
            values,
            source,
            record_index,
            expression.span,
        ),
        TypedExpressionKind::Conditional {
            condition,
            when_true,
            when_false,
        } => match evaluate_expression(condition, values, source, record_index)? {
            AuthoringValue::Bool { value: true } => {
                evaluate_expression(when_true, values, source, record_index)
            }
            AuthoringValue::Bool { value: false } => {
                evaluate_expression(when_false, values, source, record_index)
            }
            _ => Err(row_diagnostic(
                source,
                record_index,
                expression.span,
                "conditional condition was not a non-null bool",
            )),
        },
    }
}

fn evaluate_binary(
    operator: BinaryOperator,
    left: &TypedExpression,
    right: &TypedExpression,
    values: &BTreeMap<String, AuthoringValue>,
    source: &Path,
    record_index: usize,
    span: ExpressionSpan,
) -> std::result::Result<AuthoringValue, Box<Diagnostic>> {
    let left_value = evaluate_expression(left, values, source, record_index)?;
    if operator == BinaryOperator::And && left_value == (AuthoringValue::Bool { value: false }) {
        return Ok(AuthoringValue::Bool { value: false });
    }
    if operator == BinaryOperator::Or && left_value == (AuthoringValue::Bool { value: true }) {
        return Ok(AuthoringValue::Bool { value: true });
    }
    if operator == BinaryOperator::Coalesce && left_value != AuthoringValue::Null {
        return Ok(left_value);
    }
    let right_value = evaluate_expression(right, values, source, record_index)?;
    match operator {
        BinaryOperator::Coalesce => Ok(right_value),
        BinaryOperator::And | BinaryOperator::Or => match (left_value, right_value) {
            (AuthoringValue::Bool { value: left }, AuthoringValue::Bool { value: right }) => {
                Ok(AuthoringValue::Bool {
                    value: if operator == BinaryOperator::And {
                        left && right
                    } else {
                        left || right
                    },
                })
            }
            _ => Err(row_diagnostic(
                source,
                record_index,
                span,
                "boolean operation received null or a non-bool value",
            )),
        },
        BinaryOperator::Equal | BinaryOperator::NotEqual => {
            let equal = match (&left_value, &right_value) {
                (AuthoringValue::Null, AuthoringValue::Null) => true,
                (AuthoringValue::Null, _) | (_, AuthoringValue::Null) => false,
                _ => left_value == right_value,
            };
            Ok(AuthoringValue::Bool {
                value: if operator == BinaryOperator::Equal {
                    equal
                } else {
                    !equal
                },
            })
        }
        BinaryOperator::Less
        | BinaryOperator::LessEqual
        | BinaryOperator::Greater
        | BinaryOperator::GreaterEqual => {
            if left_value == AuthoringValue::Null || right_value == AuthoringValue::Null {
                return Err(row_diagnostic(
                    source,
                    record_index,
                    span,
                    "ordering comparison received null; use `??` first",
                ));
            }
            let ordering =
                compare_runtime_values(&left_value, &right_value, &left.ty).ok_or_else(|| {
                    row_diagnostic(
                        source,
                        record_index,
                        span,
                        "ordering comparison could not compare these values",
                    )
                })?;
            let result = match operator {
                BinaryOperator::Less => ordering.is_lt(),
                BinaryOperator::LessEqual => !ordering.is_gt(),
                BinaryOperator::Greater => ordering.is_gt(),
                BinaryOperator::GreaterEqual => !ordering.is_lt(),
                _ => unreachable!(),
            };
            Ok(AuthoringValue::Bool { value: result })
        }
        BinaryOperator::Add => {
            if matches!(
                left.ty,
                ExpressionType::Scalar {
                    scalar: ScalarType::Primitive(PrimitiveType::String),
                    ..
                }
            ) {
                match (left_value, right_value) {
                    (
                        AuthoringValue::String { value: left },
                        AuthoringValue::String { value: right },
                    ) => Ok(AuthoringValue::String {
                        value: format!("{left}{right}"),
                    }),
                    _ => Err(row_diagnostic(
                        source,
                        record_index,
                        span,
                        "string composition received a null or non-string value",
                    )),
                }
            } else {
                numeric_binary(
                    left_value,
                    right_value,
                    numeric_primitive(&left.ty, &right.ty),
                    operator,
                    source,
                    record_index,
                    span,
                )
            }
        }
        BinaryOperator::Subtract
        | BinaryOperator::Multiply
        | BinaryOperator::Divide
        | BinaryOperator::Remainder => numeric_binary(
            left_value,
            right_value,
            numeric_primitive(&left.ty, &right.ty),
            operator,
            source,
            record_index,
            span,
        ),
    }
}

fn numeric_primitive(left: &ExpressionType, right: &ExpressionType) -> Option<PrimitiveType> {
    let left = normalize_type(left)?;
    let right = normalize_type(right)?;
    match (&left.scalar, &right.scalar) {
        (ScalarType::Primitive(left), ScalarType::Primitive(right))
            if left == right && is_numeric(*left) =>
        {
            Some(*left)
        }
        _ => None,
    }
}

fn numeric_unary(
    value: AuthoringValue,
    negative: bool,
    ty: &ExpressionType,
    source: &Path,
    record_index: usize,
    span: ExpressionSpan,
) -> std::result::Result<AuthoringValue, Box<Diagnostic>> {
    let Some(primitive) = normalize_type(ty).and_then(|ty| match ty.scalar {
        ScalarType::Primitive(primitive) if is_numeric(primitive) => Some(primitive),
        _ => None,
    }) else {
        return Err(arithmetic_diagnostic(
            source,
            record_index,
            span,
            "unary numeric operation has no numeric type",
        ));
    };
    if !negative {
        return Ok(value);
    }
    numeric_binary(
        AuthoringValue::Number {
            value: "0".to_owned(),
        },
        value,
        Some(primitive),
        BinaryOperator::Subtract,
        source,
        record_index,
        span,
    )
}

fn numeric_binary(
    left: AuthoringValue,
    right: AuthoringValue,
    primitive: Option<PrimitiveType>,
    operator: BinaryOperator,
    source: &Path,
    record_index: usize,
    span: ExpressionSpan,
) -> std::result::Result<AuthoringValue, Box<Diagnostic>> {
    let Some(primitive) = primitive else {
        return Err(arithmetic_diagnostic(
            source,
            record_index,
            span,
            "numeric operation has no numeric type",
        ));
    };
    let (AuthoringValue::Number { value: left }, AuthoringValue::Number { value: right }) =
        (left, right)
    else {
        return Err(arithmetic_diagnostic(
            source,
            record_index,
            span,
            "numeric operation received null or a non-numeric value",
        ));
    };
    let result = match primitive {
        PrimitiveType::Int => {
            checked_signed(&left, &right, operator, i32::MIN as i128, i32::MAX as i128)
        }
        PrimitiveType::Long => {
            checked_signed(&left, &right, operator, i64::MIN as i128, i64::MAX as i128)
        }
        PrimitiveType::UInt => checked_unsigned(&left, &right, operator, u32::MAX as u128),
        PrimitiveType::ULong => checked_unsigned(&left, &right, operator, u64::MAX as u128),
        PrimitiveType::Float => checked_float(&left, &right, operator, true),
        PrimitiveType::Double => checked_float(&left, &right, operator, false),
        _ => Err("unsupported numeric primitive".to_owned()),
    };
    result
        .map(|value| AuthoringValue::Number { value })
        .map_err(|message| arithmetic_diagnostic(source, record_index, span, message))
}

fn checked_signed(
    left: &str,
    right: &str,
    operator: BinaryOperator,
    min: i128,
    max: i128,
) -> std::result::Result<String, String> {
    let left = left
        .parse::<i128>()
        .map_err(|_| "invalid signed integer source value".to_owned())?;
    let right = right
        .parse::<i128>()
        .map_err(|_| "invalid signed integer source value".to_owned())?;
    let result = match operator {
        BinaryOperator::Add => left.checked_add(right),
        BinaryOperator::Subtract => left.checked_sub(right),
        BinaryOperator::Multiply => left.checked_mul(right),
        BinaryOperator::Divide => {
            if right == 0 {
                None
            } else {
                left.checked_div(right)
            }
        }
        BinaryOperator::Remainder => {
            if right == 0 {
                None
            } else {
                left.checked_rem(right)
            }
        }
        _ => None,
    }
    .ok_or_else(|| "integer arithmetic overflow or division by zero".to_owned())?;
    (min..=max)
        .contains(&result)
        .then_some(result.to_string())
        .ok_or_else(|| "integer arithmetic overflow".to_owned())
}

fn checked_unsigned(
    left: &str,
    right: &str,
    operator: BinaryOperator,
    max: u128,
) -> std::result::Result<String, String> {
    let left = left
        .parse::<u128>()
        .map_err(|_| "invalid unsigned integer source value".to_owned())?;
    let right = right
        .parse::<u128>()
        .map_err(|_| "invalid unsigned integer source value".to_owned())?;
    let result = match operator {
        BinaryOperator::Add => left.checked_add(right),
        BinaryOperator::Subtract => left.checked_sub(right),
        BinaryOperator::Multiply => left.checked_mul(right),
        BinaryOperator::Divide => {
            if right == 0 {
                None
            } else {
                left.checked_div(right)
            }
        }
        BinaryOperator::Remainder => {
            if right == 0 {
                None
            } else {
                left.checked_rem(right)
            }
        }
        _ => None,
    }
    .ok_or_else(|| "unsigned arithmetic overflow or division by zero".to_owned())?;
    (result <= max)
        .then_some(result.to_string())
        .ok_or_else(|| "unsigned arithmetic overflow".to_owned())
}

fn checked_float(
    left: &str,
    right: &str,
    operator: BinaryOperator,
    single: bool,
) -> std::result::Result<String, String> {
    if single {
        let left = left
            .parse::<f32>()
            .map_err(|_| "invalid float source value".to_owned())?;
        let right = right
            .parse::<f32>()
            .map_err(|_| "invalid float source value".to_owned())?;
        let result = match operator {
            BinaryOperator::Add => left + right,
            BinaryOperator::Subtract => left - right,
            BinaryOperator::Multiply => left * right,
            BinaryOperator::Divide => left / right,
            BinaryOperator::Remainder => left % right,
            _ => return Err("invalid floating operation".to_owned()),
        };
        result
            .is_finite()
            .then_some(result.to_string())
            .ok_or_else(|| "floating arithmetic produced a non-finite result".to_owned())
    } else {
        let left = left
            .parse::<f64>()
            .map_err(|_| "invalid double source value".to_owned())?;
        let right = right
            .parse::<f64>()
            .map_err(|_| "invalid double source value".to_owned())?;
        let result = match operator {
            BinaryOperator::Add => left + right,
            BinaryOperator::Subtract => left - right,
            BinaryOperator::Multiply => left * right,
            BinaryOperator::Divide => left / right,
            BinaryOperator::Remainder => left % right,
            _ => return Err("invalid floating operation".to_owned()),
        };
        result
            .is_finite()
            .then_some(result.to_string())
            .ok_or_else(|| "floating arithmetic produced a non-finite result".to_owned())
    }
}

fn compare_runtime_values(
    left: &AuthoringValue,
    right: &AuthoringValue,
    ty: &ExpressionType,
) -> Option<std::cmp::Ordering> {
    match normalize_type(ty)?.scalar {
        ScalarType::Primitive(primitive) if is_numeric(primitive) => {
            let (AuthoringValue::Number { value: left }, AuthoringValue::Number { value: right }) =
                (left, right)
            else {
                return None;
            };
            match primitive {
                PrimitiveType::Int | PrimitiveType::Long => {
                    left.parse::<i128>().ok()?.partial_cmp(&right.parse().ok()?)
                }
                PrimitiveType::UInt | PrimitiveType::ULong => {
                    left.parse::<u128>().ok()?.partial_cmp(&right.parse().ok()?)
                }
                PrimitiveType::Float => left.parse::<f32>().ok()?.partial_cmp(&right.parse().ok()?),
                PrimitiveType::Double => {
                    left.parse::<f64>().ok()?.partial_cmp(&right.parse().ok()?)
                }
                _ => None,
            }
        }
        ScalarType::Primitive(PrimitiveType::String)
        | ScalarType::ValueObject {
            underlying: PrimitiveType::String,
            ..
        } => match (left, right) {
            (AuthoringValue::String { value: left }, AuthoringValue::String { value: right }) => {
                Some(left.cmp(right))
            }
            _ => None,
        },
        _ => None,
    }
}

fn runtime_value_matches_type(ty: &ExpressionType, value: &AuthoringValue) -> bool {
    let ExpressionType::Scalar { scalar, .. } = ty else {
        return false;
    };
    match (scalar, value) {
        (ScalarType::Primitive(primitive), AuthoringValue::Bool { .. }) => {
            *primitive == PrimitiveType::Bool
        }
        (ScalarType::Primitive(PrimitiveType::String), AuthoringValue::String { .. }) => true,
        (ScalarType::Primitive(primitive), AuthoringValue::Number { value })
            if primitive.is_integer() =>
        {
            value
                .parse::<i128>()
                .ok()
                .and_then(|number| {
                    primitive
                        .integer_range()
                        .map(|(min, max)| (min..=max).contains(&number))
                })
                .unwrap_or(false)
        }
        (
            ScalarType::Primitive(PrimitiveType::Float | PrimitiveType::Double),
            AuthoringValue::Number { value },
        ) => value.parse::<f64>().ok().is_some_and(f64::is_finite),
        (ScalarType::ValueObject { underlying, .. }, value) => {
            runtime_value_matches_type(&scalar_type(*underlying, false), value)
        }
        (ScalarType::Enum { members, .. }, AuthoringValue::String { value }) => {
            members.contains(value)
        }
        _ => false,
    }
}

fn is_numeric(primitive: PrimitiveType) -> bool {
    primitive.is_integer() || matches!(primitive, PrimitiveType::Float | PrimitiveType::Double)
}

fn row_diagnostic(
    source: &Path,
    record_index: usize,
    span: ExpressionSpan,
    message: impl Into<String>,
) -> Box<Diagnostic> {
    row_diagnostic_with_code("E-VIEW-ROW-EVALUATION", source, record_index, span, message)
}

fn arithmetic_diagnostic(
    source: &Path,
    record_index: usize,
    span: ExpressionSpan,
    message: impl Into<String>,
) -> Box<Diagnostic> {
    row_diagnostic_with_code("E-VIEW-ARITHMETIC", source, record_index, span, message)
}

fn row_diagnostic_with_code(
    code: &str,
    source: &Path,
    record_index: usize,
    span: ExpressionSpan,
    message: impl Into<String>,
) -> Box<Diagnostic> {
    let mut diagnostic = Diagnostic::new(code, ErrorKind::Validation, message)
        .with_source(source.to_path_buf())
        .with_record_identity(format!("record[{record_index}]"))
        .with_related_requirement("ADV-VIEW-003");
    diagnostic.line = Some(1);
    diagnostic.column = Some(span.start + 1);
    Box::new(diagnostic)
}

fn expression_diagnostic(path: &Path, schema_path: &str, error: ExpressionError) -> Diagnostic {
    let mut diagnostic = Diagnostic::new(error.code, error.kind, error.message)
        .with_source(path.to_path_buf())
        .with_schema_path(schema_path.to_owned())
        .with_related_requirement("ADV-VIEW-002");
    diagnostic.line = Some(1);
    diagnostic.column = Some(error.span.start + 1);
    diagnostic
}

fn view_diagnostic(
    path: &Path,
    code: &str,
    message: impl Into<String>,
    requirement: &str,
    schema_path: Option<&str>,
) -> Diagnostic {
    let mut diagnostic = Diagnostic::new(code, ErrorKind::Validation, message)
        .with_source(path.to_path_buf())
        .with_related_requirement(requirement);
    if let Some(schema_path) = schema_path {
        diagnostic.schema_path = Some(schema_path.to_owned());
    }
    diagnostic
}

fn diagnostic_order(left: &Diagnostic, right: &Diagnostic) -> std::cmp::Ordering {
    left.source
        .cmp(&right.source)
        .then_with(|| schema_path_order(&left.schema_path, &right.schema_path))
        .then_with(|| left.line.cmp(&right.line))
        .then_with(|| left.column.cmp(&right.column))
        .then_with(|| left.code.cmp(&right.code))
}

fn schema_path_order(left: &Option<String>, right: &Option<String>) -> std::cmp::Ordering {
    fn key(path: &Option<String>) -> (Option<usize>, &str) {
        let Some(path) = path.as_deref() else {
            return (None, "");
        };
        let Some(index) = path
            .strip_prefix("columns[")
            .and_then(|rest| rest.strip_suffix(']'))
            .and_then(|index| index.parse::<usize>().ok())
        else {
            return (None, path);
        };
        (Some(index), path)
    }

    key(left).cmp(&key(right))
}

fn is_lower_camel_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    matches!(chars.next(), Some(first) if first.is_ascii_lowercase())
        && chars.all(|character| character.is_ascii_alphanumeric())
}

fn is_identifier_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

fn is_identifier_continue(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

#[derive(Debug, Clone, PartialEq)]
struct Expression {
    root: RawExpression,
}

#[derive(Debug, Clone, PartialEq)]
struct RawExpression {
    span: ExpressionSpan,
    kind: RawExpressionKind,
}

#[derive(Debug, Clone, PartialEq)]
enum RawExpressionKind {
    Null,
    Bool(bool),
    Integer(String),
    Float(String),
    String(String),
    Field(String),
    EnumLiteral {
        type_name: String,
        member: String,
    },
    Unary {
        operator: UnaryOperator,
        operand: Box<RawExpression>,
    },
    Binary {
        operator: BinaryOperator,
        left: Box<RawExpression>,
        right: Box<RawExpression>,
    },
    Conditional {
        condition: Box<RawExpression>,
        when_true: Box<RawExpression>,
        when_false: Box<RawExpression>,
    },
}

#[derive(Debug, Clone, PartialEq)]
enum TokenKind {
    Identifier(String),
    Number { value: String, float: bool },
    String(String),
    Null,
    Bool(bool),
    Question,
    Colon,
    Dot,
    LeftParen,
    RightParen,
    Operator(&'static str),
    End,
}

#[derive(Debug, Clone, PartialEq)]
struct Token {
    kind: TokenKind,
    span: ExpressionSpan,
}

struct Lexer<'a> {
    source: &'a str,
    index: usize,
}

impl<'a> Lexer<'a> {
    fn new(source: &'a str) -> Self {
        Self { source, index: 0 }
    }

    fn lex(mut self) -> std::result::Result<Vec<Token>, ExpressionError> {
        let mut tokens = Vec::new();
        while self.index < self.source.len() {
            let byte = self.source.as_bytes()[self.index];
            if byte.is_ascii_whitespace() {
                self.index += 1;
                continue;
            }
            let start = self.index;
            let kind = if is_identifier_start(byte) {
                self.index += 1;
                while self.index < self.source.len()
                    && is_identifier_continue(self.source.as_bytes()[self.index])
                {
                    self.index += 1;
                }
                let value = &self.source[start..self.index];
                match value {
                    "null" => TokenKind::Null,
                    "true" => TokenKind::Bool(true),
                    "false" => TokenKind::Bool(false),
                    _ => TokenKind::Identifier(value.to_owned()),
                }
            } else if byte.is_ascii_digit() {
                self.index += 1;
                while self.index < self.source.len()
                    && self.source.as_bytes()[self.index].is_ascii_digit()
                {
                    self.index += 1;
                }
                let mut float = false;
                if self.source.as_bytes().get(self.index) == Some(&b'.') {
                    float = true;
                    self.index += 1;
                    while self.index < self.source.len()
                        && self.source.as_bytes()[self.index].is_ascii_digit()
                    {
                        self.index += 1;
                    }
                }
                if matches!(self.source.as_bytes().get(self.index), Some(b'e' | b'E')) {
                    float = true;
                    self.index += 1;
                    if matches!(self.source.as_bytes().get(self.index), Some(b'+' | b'-')) {
                        self.index += 1;
                    }
                    let exponent_start = self.index;
                    while self.index < self.source.len()
                        && self.source.as_bytes()[self.index].is_ascii_digit()
                    {
                        self.index += 1;
                    }
                    if exponent_start == self.index {
                        return Err(ExpressionError::parse(
                            "numeric exponent requires digits",
                            ExpressionSpan {
                                start,
                                end: self.index,
                            },
                        ));
                    }
                }
                TokenKind::Number {
                    value: self.source[start..self.index].to_owned(),
                    float,
                }
            } else if byte == b'"' {
                self.index += 1;
                let mut escaped = false;
                while self.index < self.source.len() {
                    let current = self.source.as_bytes()[self.index];
                    self.index += 1;
                    if escaped {
                        escaped = false;
                    } else if current == b'\\' {
                        escaped = true;
                    } else if current == b'"' {
                        break;
                    }
                }
                if self.source.as_bytes().get(self.index.saturating_sub(1)) != Some(&b'"') {
                    return Err(ExpressionError::parse(
                        "unterminated string literal",
                        ExpressionSpan {
                            start,
                            end: self.index,
                        },
                    ));
                }
                let raw = &self.source[start..self.index];
                let value = serde_json::from_str::<String>(raw).map_err(|_| {
                    ExpressionError::parse(
                        "invalid JSON-compatible string escape",
                        ExpressionSpan {
                            start,
                            end: self.index,
                        },
                    )
                })?;
                TokenKind::String(value)
            } else {
                let two = self
                    .source
                    .get(self.index..self.index + 2)
                    .unwrap_or_default();
                if matches!(two, "??" | "||" | "&&" | "==" | "!=" | "<=" | ">=") {
                    self.index += 2;
                    TokenKind::Operator(match two {
                        "??" => "??",
                        "||" => "||",
                        "&&" => "&&",
                        "==" => "==",
                        "!=" => "!=",
                        "<=" => "<=",
                        ">=" => ">=",
                        _ => unreachable!(),
                    })
                } else {
                    self.index += 1;
                    match byte {
                        b'?' => TokenKind::Question,
                        b':' => TokenKind::Colon,
                        b'.' => TokenKind::Dot,
                        b'(' => TokenKind::LeftParen,
                        b')' => TokenKind::RightParen,
                        b'+' => TokenKind::Operator("+"),
                        b'-' => TokenKind::Operator("-"),
                        b'*' => TokenKind::Operator("*"),
                        b'/' => TokenKind::Operator("/"),
                        b'%' => TokenKind::Operator("%"),
                        b'!' => TokenKind::Operator("!"),
                        b'<' => TokenKind::Operator("<"),
                        b'>' => TokenKind::Operator(">"),
                        _ => {
                            return Err(ExpressionError::parse(
                                "unexpected character in expression",
                                ExpressionSpan {
                                    start,
                                    end: self.index,
                                },
                            ));
                        }
                    }
                }
            };
            tokens.push(Token {
                kind,
                span: ExpressionSpan {
                    start,
                    end: self.index,
                },
            });
        }
        tokens.push(Token {
            kind: TokenKind::End,
            span: ExpressionSpan {
                start: self.source.len(),
                end: self.source.len(),
            },
        });
        Ok(tokens)
    }
}

struct Parser {
    tokens: Vec<Token>,
    index: usize,
    lex_error: Option<ExpressionError>,
}

impl Parser {
    fn new(source: &str) -> Self {
        let (tokens, lex_error) = match Lexer::new(source).lex() {
            Ok(tokens) => (tokens, None),
            Err(error) => (
                vec![Token {
                    kind: TokenKind::End,
                    span: error.span,
                }],
                Some(error),
            ),
        };
        Self {
            tokens,
            index: 0,
            lex_error,
        }
    }

    fn parse(mut self) -> std::result::Result<Expression, ExpressionError> {
        if let Some(error) = self.lex_error {
            return Err(error);
        }
        let root = self.parse_conditional()?;
        if !matches!(self.current().kind, TokenKind::End) {
            return Err(ExpressionError::parse(
                "unexpected token after expression",
                self.current().span,
            ));
        }
        Ok(Expression { root })
    }

    fn parse_conditional(&mut self) -> std::result::Result<RawExpression, ExpressionError> {
        let condition = self.parse_coalesce()?;
        if !matches!(self.current().kind, TokenKind::Question) {
            return Ok(condition);
        }
        self.advance();
        let when_true = self.parse_conditional()?;
        self.expect(TokenKind::Colon, "conditional expression requires `:`")?;
        let when_false = self.parse_conditional()?;
        Ok(RawExpression {
            span: ExpressionSpan {
                start: condition.span.start,
                end: when_false.span.end,
            },
            kind: RawExpressionKind::Conditional {
                condition: Box::new(condition),
                when_true: Box::new(when_true),
                when_false: Box::new(when_false),
            },
        })
    }

    fn parse_coalesce(&mut self) -> std::result::Result<RawExpression, ExpressionError> {
        self.parse_left_associative(Self::parse_or, &[("??", BinaryOperator::Coalesce)])
    }

    fn parse_or(&mut self) -> std::result::Result<RawExpression, ExpressionError> {
        self.parse_left_associative(Self::parse_and, &[("||", BinaryOperator::Or)])
    }

    fn parse_and(&mut self) -> std::result::Result<RawExpression, ExpressionError> {
        self.parse_left_associative(Self::parse_equality, &[("&&", BinaryOperator::And)])
    }

    fn parse_equality(&mut self) -> std::result::Result<RawExpression, ExpressionError> {
        self.parse_left_associative(
            Self::parse_relation,
            &[
                ("==", BinaryOperator::Equal),
                ("!=", BinaryOperator::NotEqual),
            ],
        )
    }

    fn parse_relation(&mut self) -> std::result::Result<RawExpression, ExpressionError> {
        self.parse_left_associative(
            Self::parse_additive,
            &[
                ("<", BinaryOperator::Less),
                ("<=", BinaryOperator::LessEqual),
                (">", BinaryOperator::Greater),
                (">=", BinaryOperator::GreaterEqual),
            ],
        )
    }

    fn parse_additive(&mut self) -> std::result::Result<RawExpression, ExpressionError> {
        self.parse_left_associative(
            Self::parse_multiplicative,
            &[("+", BinaryOperator::Add), ("-", BinaryOperator::Subtract)],
        )
    }

    fn parse_multiplicative(&mut self) -> std::result::Result<RawExpression, ExpressionError> {
        self.parse_left_associative(
            Self::parse_unary,
            &[
                ("*", BinaryOperator::Multiply),
                ("/", BinaryOperator::Divide),
                ("%", BinaryOperator::Remainder),
            ],
        )
    }

    fn parse_left_associative(
        &mut self,
        operand: fn(&mut Self) -> std::result::Result<RawExpression, ExpressionError>,
        operators: &[(&'static str, BinaryOperator)],
    ) -> std::result::Result<RawExpression, ExpressionError> {
        let mut left = operand(self)?;
        while let Some((_, operator)) = operators
            .iter()
            .find(|(token, _)| matches!(&self.current().kind, TokenKind::Operator(current) if current == token))
        {
            let operator = *operator;
            self.advance();
            let right = operand(self)?;
            let span = ExpressionSpan {
                start: left.span.start,
                end: right.span.end,
            };
            left = RawExpression {
                span,
                kind: RawExpressionKind::Binary {
                    operator,
                    left: Box::new(left),
                    right: Box::new(right),
                },
            };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> std::result::Result<RawExpression, ExpressionError> {
        let operator = match self.current().kind {
            TokenKind::Operator("!") => Some(UnaryOperator::Not),
            TokenKind::Operator("+") => Some(UnaryOperator::Plus),
            TokenKind::Operator("-") => Some(UnaryOperator::Minus),
            _ => None,
        };
        let Some(operator) = operator else {
            return self.parse_primary();
        };
        let start = self.current().span.start;
        self.advance();
        let operand = self.parse_unary()?;
        Ok(RawExpression {
            span: ExpressionSpan {
                start,
                end: operand.span.end,
            },
            kind: RawExpressionKind::Unary {
                operator,
                operand: Box::new(operand),
            },
        })
    }

    fn parse_primary(&mut self) -> std::result::Result<RawExpression, ExpressionError> {
        let token = self.current().clone();
        match token.kind {
            TokenKind::Null => {
                self.advance();
                Ok(RawExpression {
                    span: token.span,
                    kind: RawExpressionKind::Null,
                })
            }
            TokenKind::Bool(value) => {
                self.advance();
                Ok(RawExpression {
                    span: token.span,
                    kind: RawExpressionKind::Bool(value),
                })
            }
            TokenKind::Number { value, float } => {
                self.advance();
                Ok(RawExpression {
                    span: token.span,
                    kind: if float {
                        RawExpressionKind::Float(value)
                    } else {
                        RawExpressionKind::Integer(value)
                    },
                })
            }
            TokenKind::String(value) => {
                self.advance();
                Ok(RawExpression {
                    span: token.span,
                    kind: RawExpressionKind::String(value),
                })
            }
            TokenKind::Identifier(name) => {
                self.advance();
                if matches!(self.current().kind, TokenKind::Dot) {
                    self.advance();
                    let member = match self.current().kind.clone() {
                        TokenKind::Identifier(member) => member,
                        _ => {
                            return Err(ExpressionError::parse(
                                "Enum literal requires a member name after `.`",
                                self.current().span,
                            ));
                        }
                    };
                    let end = self.current().span.end;
                    self.advance();
                    Ok(RawExpression {
                        span: ExpressionSpan {
                            start: token.span.start,
                            end,
                        },
                        kind: RawExpressionKind::EnumLiteral {
                            type_name: name,
                            member,
                        },
                    })
                } else {
                    Ok(RawExpression {
                        span: token.span,
                        kind: RawExpressionKind::Field(name),
                    })
                }
            }
            TokenKind::LeftParen => {
                self.advance();
                let expression = self.parse_conditional()?;
                self.expect(TokenKind::RightParen, "grouped expression requires `)`")?;
                Ok(expression)
            }
            _ => Err(ExpressionError::parse(
                "expected literal, field, Enum literal, or grouped expression",
                token.span,
            )),
        }
    }

    fn current(&self) -> &Token {
        &self.tokens[self.index]
    }

    fn advance(&mut self) {
        self.index = (self.index + 1).min(self.tokens.len() - 1);
    }

    fn expect(
        &mut self,
        expected: TokenKind,
        message: &str,
    ) -> std::result::Result<(), ExpressionError> {
        if std::mem::discriminant(&self.current().kind) == std::mem::discriminant(&expected) {
            self.advance();
            Ok(())
        } else {
            Err(ExpressionError::parse(message, self.current().span))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{evaluate_computed_column, field_reference_spans, resolve_computed_views};
    use crate::{AuthoringValue, ProjectDocuments, parse_yaml_document};
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    fn documents(view: &str, schema: &str) -> ProjectDocuments {
        let mut documents = ProjectDocuments::default();
        documents
            .files
            .push(parse_yaml_document(PathBuf::from("schema.yaml"), schema).unwrap());
        documents
            .files
            .push(parse_yaml_document(PathBuf::from("view.yaml"), view).unwrap());
        documents
    }

    #[test]
    fn resolves_typed_scalar_expression_and_evaluates_it() {
        let build = resolve_computed_views(&documents(
            "kind: view\nname: itemDisplay\ntable: item\ncolumns:\n  - name: label\n    expression: 'name + \"!\"'\n",
            "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: name\n    type: string\nprimaryKey:\n  fields: [name]\n",
        ));
        assert!(build.diagnostics.is_empty(), "{:?}", build.diagnostics);
        let column = &build.views[0].columns[0];
        let mut row = BTreeMap::new();
        row.insert(
            "name".to_owned(),
            AuthoringValue::String {
                value: "Potion".into(),
            },
        );
        assert_eq!(
            evaluate_computed_column(column, &row, PathBuf::from("data.yaml").as_path(), 0),
            AuthoringValue::String {
                value: "Potion!".into()
            }
        );
    }

    #[test]
    fn nullable_coalesce_and_partial_expression_are_deterministic() {
        let build = resolve_computed_views(&documents(
            "kind: view\nname: itemDisplay\ntable: item\ncolumns:\n  - name: label\n    expression: 'name ?? \"unknown\"'\n",
            "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: name\n    type: string\n    nullable: true\nprimaryKey:\n  fields: [name]\n",
        ));
        assert!(build.diagnostics.is_empty(), "{:?}", build.diagnostics);
        let column = &build.views[0].columns[0];
        let mut row = BTreeMap::new();
        row.insert("name".to_owned(), AuthoringValue::Null);
        assert_eq!(
            evaluate_computed_column(column, &row, PathBuf::from("data.yaml").as_path(), 0),
            AuthoringValue::String {
                value: "unknown".into()
            }
        );
    }

    #[test]
    fn rename_spans_skip_strings_and_enum_members() {
        assert_eq!(
            field_reference_spans("old + \"old\" + Rarity.Rare", "old").len(),
            1
        );
    }

    #[test]
    fn type_diagnostics_reject_unknown_fields_and_implicit_coercion() {
        let build = resolve_computed_views(&documents(
            "kind: view\nname: invalid\ntable: item\ncolumns:\n  - name: value\n    expression: missing + 1\n  - name: other\n    expression: 1 + \"x\"\n  - name: fractional\n    expression: id + 0.5\n",
            "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\nprimaryKey:\n  fields: [id]\n",
        ));
        assert_eq!(build.views.len(), 0);
        assert!(
            build
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "E-VIEW-UNKNOWN-FIELD")
        );
        assert!(
            build
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code == "E-VIEW-TYPE-MISMATCH")
        );
        assert!(
            build
                .diagnostics
                .iter()
                .all(|diagnostic| diagnostic.column.is_some())
        );
    }

    #[test]
    fn checked_arithmetic_and_required_nulls_become_row_diagnostics() {
        let build = resolve_computed_views(&documents(
            "kind: view\nname: math\ntable: item\ncolumns:\n  - name: quotient\n    expression: quantity / divisor\n  - name: sum\n    expression: quantity + 1\n",
            "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: quantity\n    type: int\n  - key: 1\n    name: divisor\n    type: int\nprimaryKey:\n  fields: [quantity]\n",
        ));
        assert!(build.diagnostics.is_empty(), "{:?}", build.diagnostics);
        let mut row = BTreeMap::new();
        row.insert(
            "quantity".to_owned(),
            AuthoringValue::Number {
                value: i32::MAX.to_string(),
            },
        );
        row.insert(
            "divisor".to_owned(),
            AuthoringValue::Number { value: "0".into() },
        );
        let source = PathBuf::from("data.yaml");
        let quotient = evaluate_computed_column(&build.views[0].columns[0], &row, &source, 2);
        assert!(
            matches!(&quotient, AuthoringValue::Invalid { diagnostic } if diagnostic.code == "E-VIEW-ARITHMETIC")
        );
        assert!(matches!(
            evaluate_computed_column(&build.views[0].columns[1], &row, &source, 2),
            AuthoringValue::Invalid { .. }
        ));
        row.insert("quantity".to_owned(), AuthoringValue::Null);
        assert!(matches!(
            evaluate_computed_column(&build.views[0].columns[1], &row, &source, 3),
            AuthoringValue::Invalid { .. }
        ));
    }

    #[test]
    fn lexer_diagnostics_keep_the_unexpected_character_span() {
        let build = resolve_computed_views(&documents(
            "kind: view\nname: invalid\ntable: item\ncolumns:\n  - name: value\n    expression: 'name @ 1'\n",
            "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: name\n    type: string\nprimaryKey:\n  fields: [name]\n",
        ));
        let diagnostic = build
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == "E-VIEW-EXPRESSION-SYNTAX")
            .expect("syntax diagnostic");
        assert_eq!(diagnostic.column, Some(6));
    }

    #[test]
    fn conditional_and_enum_literal_are_typed_without_cross_column_dependencies() {
        let mut documents = documents(
            "kind: view\nname: label\ntable: item\ncolumns:\n  - name: display\n    expression: 'status == Kind.Active ? \"active\" : \"other\"'\n",
            "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: status\n    type: Kind\nprimaryKey:\n  fields: [status]\n",
        );
        documents.files.push(crate::parse_yaml_document(
            PathBuf::from("kind.yaml"),
            "kind: type\nname: Kind\nenum:\n  underlying: int\n  members:\n    - name: Active\n      value: 1\n",
        ).unwrap());
        let build = resolve_computed_views(&documents);
        assert!(build.diagnostics.is_empty(), "{:?}", build.diagnostics);
        let mut row = BTreeMap::new();
        row.insert(
            "status".to_owned(),
            AuthoringValue::String {
                value: "Active".into(),
            },
        );
        assert_eq!(
            evaluate_computed_column(
                &build.views[0].columns[0],
                &row,
                PathBuf::from("data.yaml").as_path(),
                0
            ),
            AuthoringValue::String {
                value: "active".into()
            }
        );
    }
}
