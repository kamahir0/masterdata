//! Read-only search, filter, and sort semantics for authoring snapshots.

use std::cmp::Ordering;

use serde::{Deserialize, Serialize};

use crate::authoring_value::AuthoringValue;
use crate::error::{ErrorKind, MasterdataError, Result};
use crate::type_system::{
    FieldModifier, PrimitiveType, ResolvedAuthoringField, ResolvedAuthoringType,
};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum QueryOperator {
    Equals,
    NotEquals,
    LessThan,
    GreaterThan,
    Contains,
    IsNull,
    IsInvalid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ColumnFilter {
    pub field: String,
    pub operator: QueryOperator,
    #[serde(default)]
    pub value: Option<AuthoringValue>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SortDirection {
    Ascending,
    Descending,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct QuerySort {
    pub field: String,
    pub direction: SortDirection,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AuthoringQuery {
    #[serde(default)]
    pub search: String,
    #[serde(default)]
    pub filters: Vec<ColumnFilter>,
    pub sort: Option<QuerySort>,
}

/// A small host-neutral row representation.  `source_order` is only a stable
/// tie-breaker; it is never exposed as a domain identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryRow {
    pub source_order: usize,
    pub values: Vec<AuthoringValue>,
}

/// Evaluate a read-only query and return source-order row positions.  The
/// caller supplies columns in schema order and rows in the captured snapshot
/// order, so this function cannot reorder schema columns or mutate source.
pub fn apply_authoring_query(
    columns: &[ResolvedAuthoringField],
    rows: &[QueryRow],
    query: &AuthoringQuery,
) -> Result<Vec<usize>> {
    for filter in &query.filters {
        let index = columns
            .iter()
            .position(|column| column.name == filter.field)
            .ok_or_else(|| {
                query_error(
                    "E-AUTHORING-QUERY-FIELD",
                    format!("unknown query field `{}`", filter.field),
                )
            })?;
        validate_filter_input(&columns[index], filter)?;
    }
    let mut result = Vec::new();
    let search = &query.search;
    for (row_index, row) in rows.iter().enumerate() {
        if row.values.len() != columns.len() {
            return Err(query_error(
                "E-AUTHORING-QUERY-ROW-SHAPE",
                "query row does not have the same number of values as the schema",
            ));
        }
        if !search.is_empty()
            && !columns.iter().zip(&row.values).any(|(column, value)| {
                searchable_text(column, value).is_some_and(|text| text.contains(search))
            })
        {
            continue;
        }
        if !query.filters.iter().all(|filter| {
            let Some(index) = columns
                .iter()
                .position(|column| column.name == filter.field)
            else {
                return false;
            };
            evaluate_filter(&columns[index], &row.values[index], filter).unwrap_or(false)
        }) {
            continue;
        }
        result.push(row_index);
    }

    if let Some(sort) = &query.sort {
        let index = columns
            .iter()
            .position(|column| column.name == sort.field)
            .ok_or_else(|| {
                query_error(
                    "E-AUTHORING-QUERY-FIELD",
                    format!("unknown sort field `{}`", sort.field),
                )
            })?;
        if !is_sort_capable(&columns[index]) {
            return Err(query_error(
                "E-AUTHORING-QUERY-SORT-UNSUPPORTED",
                format!("field `{}` is outside v1 sort capability", sort.field),
            ));
        }
        let column = &columns[index];
        result.sort_by(|left, right| {
            let ordering = compare_sort_values(
                column,
                &rows[*left].values[index],
                &rows[*right].values[index],
            );
            match ordering {
                Ok(ordering) => match sort.direction {
                    SortDirection::Ascending => ordering,
                    SortDirection::Descending
                        if value_state(column, &rows[*left].values[index]) == ValueState::Valid
                            && value_state(column, &rows[*right].values[index])
                                == ValueState::Valid =>
                    {
                        ordering.reverse()
                    }
                    SortDirection::Descending => ordering,
                },
                Err(_) => Ordering::Equal,
            }
        });
    }
    Ok(result)
}

fn validate_filter_input(shape: &ResolvedAuthoringField, filter: &ColumnFilter) -> Result<()> {
    match filter.operator {
        QueryOperator::IsNull | QueryOperator::IsInvalid => {
            if filter.value.is_some() {
                return Err(query_error(
                    "E-AUTHORING-QUERY-INPUT",
                    format!("operator {:?} does not accept a value", filter.operator),
                ));
            }
        }
        QueryOperator::Equals | QueryOperator::NotEquals | QueryOperator::Contains => {
            let expected = filter.value.as_ref().ok_or_else(|| {
                query_error(
                    "E-AUTHORING-QUERY-INPUT",
                    format!("operator {:?} requires a value", filter.operator),
                )
            })?;
            if value_state(shape, expected) != ValueState::Valid {
                return Err(query_error(
                    "E-AUTHORING-QUERY-INPUT",
                    format!(
                        "query input for `{}` is not a valid non-null value",
                        shape.name
                    ),
                ));
            }
            match filter.operator {
                QueryOperator::Contains if !is_string_shape(shape) => {
                    return Err(query_error(
                        "E-AUTHORING-QUERY-OPERATOR-UNSUPPORTED",
                        format!("contains is not supported for `{}`", shape.name),
                    ));
                }
                QueryOperator::Equals | QueryOperator::NotEquals if !is_equality_shape(shape) => {
                    return Err(query_error(
                        "E-AUTHORING-QUERY-OPERATOR-UNSUPPORTED",
                        format!("equality is not supported for `{}`", shape.name),
                    ));
                }
                _ => {}
            }
        }
        QueryOperator::LessThan | QueryOperator::GreaterThan => {
            if !is_integer_shape(shape) {
                return Err(query_error(
                    "E-AUTHORING-QUERY-OPERATOR-UNSUPPORTED",
                    format!("numeric comparison is not supported for `{}`", shape.name),
                ));
            }
            let expected = filter.value.as_ref().ok_or_else(|| {
                query_error(
                    "E-AUTHORING-QUERY-INPUT",
                    format!("operator {:?} requires a value", filter.operator),
                )
            })?;
            if value_state(shape, expected) != ValueState::Valid {
                return Err(query_error(
                    "E-AUTHORING-QUERY-INPUT",
                    format!(
                        "query input for `{}` is not a valid non-null value",
                        shape.name
                    ),
                ));
            }
        }
    }
    Ok(())
}

fn evaluate_filter(
    shape: &ResolvedAuthoringField,
    actual: &AuthoringValue,
    filter: &ColumnFilter,
) -> Result<bool> {
    let state = value_state(shape, actual);
    match filter.operator {
        // A required field containing an explicit null is invalid for typed
        // validation, but it is still visibly null in the authoring buffer.
        // The v1 contract intentionally lets `is-null` and `is-invalid` both
        // explain that same cell.
        QueryOperator::IsNull => Ok(matches!(actual, AuthoringValue::Null)),
        QueryOperator::IsInvalid => Ok(state == ValueState::Invalid),
        QueryOperator::Equals
        | QueryOperator::NotEquals
        | QueryOperator::LessThan
        | QueryOperator::GreaterThan
        | QueryOperator::Contains => {
            if state != ValueState::Valid {
                // In particular, not-equals does not turn invalid values into
                // a match; v1 ordinary operators only evaluate valid values.
                return Ok(false);
            }
            let expected = filter.value.as_ref().ok_or_else(|| {
                query_error(
                    "E-AUTHORING-QUERY-INPUT",
                    format!("operator {:?} requires a value", filter.operator),
                )
            })?;
            if value_state(shape, expected) != ValueState::Valid {
                return Err(query_error(
                    "E-AUTHORING-QUERY-INPUT",
                    format!(
                        "query input for `{}` is not a valid non-null value",
                        shape.name
                    ),
                ));
            }
            let ordering = compare_valid_values(shape, actual, expected)?;
            Ok(match filter.operator {
                QueryOperator::Equals => ordering == Ordering::Equal,
                QueryOperator::NotEquals => ordering != Ordering::Equal,
                QueryOperator::LessThan => ordering == Ordering::Less,
                QueryOperator::GreaterThan => ordering == Ordering::Greater,
                QueryOperator::Contains => {
                    let Some(left) = scalar_string(actual) else {
                        return Ok(false);
                    };
                    let Some(right) = scalar_string(expected) else {
                        return Ok(false);
                    };
                    left.contains(right)
                }
                QueryOperator::IsNull | QueryOperator::IsInvalid => unreachable!(),
            })
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ValueState {
    Valid,
    Null,
    Invalid,
}

fn value_state(shape: &ResolvedAuthoringField, value: &AuthoringValue) -> ValueState {
    match shape.modifier {
        FieldModifier::Nullable if matches!(value, AuthoringValue::Null) => ValueState::Null,
        FieldModifier::Required | FieldModifier::Array if matches!(value, AuthoringValue::Null) => {
            ValueState::Invalid
        }
        FieldModifier::Array => match value {
            AuthoringValue::Sequence { items, .. } => {
                if items
                    .iter()
                    .all(|item| value_state_type(&shape.shape, &item.value) == ValueState::Valid)
                {
                    ValueState::Valid
                } else {
                    ValueState::Invalid
                }
            }
            _ => ValueState::Invalid,
        },
        FieldModifier::Required | FieldModifier::Nullable => value_state_type(&shape.shape, value),
    }
}

fn value_state_type(shape: &ResolvedAuthoringType, value: &AuthoringValue) -> ValueState {
    match shape {
        ResolvedAuthoringType::Primitive { primitive } => match (primitive, value) {
            (PrimitiveType::String, AuthoringValue::String { .. }) => ValueState::Valid,
            (PrimitiveType::Bool, AuthoringValue::Bool { .. }) => ValueState::Valid,
            (primitive, AuthoringValue::Number { value }) if primitive.is_integer() => {
                if value
                    .parse::<i128>()
                    .ok()
                    .and_then(|number| {
                        primitive
                            .integer_range()
                            .map(|(min, max)| (min..=max).contains(&number))
                    })
                    .unwrap_or(false)
                {
                    ValueState::Valid
                } else {
                    ValueState::Invalid
                }
            }
            (PrimitiveType::Float | PrimitiveType::Double, AuthoringValue::Number { value }) => {
                value
                    .parse::<f64>()
                    .ok()
                    .filter(|number| number.is_finite())
                    .map(|_| ValueState::Valid)
                    .unwrap_or(ValueState::Invalid)
            }
            _ => ValueState::Invalid,
        },
        ResolvedAuthoringType::ValueObject { underlying, .. } => value_state_type(
            &ResolvedAuthoringType::Primitive {
                primitive: *underlying,
            },
            value,
        ),
        ResolvedAuthoringType::Enum { members, .. } => match value {
            AuthoringValue::String { value } if members.iter().any(|member| member == value) => {
                ValueState::Valid
            }
            _ => ValueState::Invalid,
        },
        ResolvedAuthoringType::Flags { .. } => ValueState::Invalid,
        ResolvedAuthoringType::Custom { fields, .. } => match value {
            AuthoringValue::Mapping { entries } if entries.len() == fields.len() => {
                if fields.iter().all(|field| {
                    entries
                        .iter()
                        .find(|entry| entry.name == field.name)
                        .is_some_and(|entry| value_state(field, &entry.value) == ValueState::Valid)
                }) {
                    ValueState::Valid
                } else {
                    ValueState::Invalid
                }
            }
            _ => ValueState::Invalid,
        },
    }
}

fn is_equality_shape(shape: &ResolvedAuthoringField) -> bool {
    matches!(
        shape.shape,
        ResolvedAuthoringType::Primitive {
            primitive: PrimitiveType::Bool
                | PrimitiveType::Int
                | PrimitiveType::UInt
                | PrimitiveType::Long
                | PrimitiveType::ULong
                | PrimitiveType::String
        } | ResolvedAuthoringType::ValueObject {
            underlying: PrimitiveType::Int
                | PrimitiveType::UInt
                | PrimitiveType::Long
                | PrimitiveType::ULong
                | PrimitiveType::String,
            ..
        } | ResolvedAuthoringType::Enum { .. }
    )
}

fn searchable_text(shape: &ResolvedAuthoringField, value: &AuthoringValue) -> Option<String> {
    if value_state(shape, value) != ValueState::Valid {
        return None;
    }
    Some(match value {
        AuthoringValue::String { value } | AuthoringValue::Number { value } => value.clone(),
        AuthoringValue::Bool { value } => value.to_string(),
        _ => return None,
    })
}

fn scalar_string(value: &AuthoringValue) -> Option<&str> {
    match value {
        AuthoringValue::String { value } | AuthoringValue::Number { value } => Some(value),
        _ => None,
    }
}

fn is_sort_capable(shape: &ResolvedAuthoringField) -> bool {
    matches!(shape.modifier, FieldModifier::Required)
        && matches!(
            shape.shape,
            ResolvedAuthoringType::Primitive {
                primitive: PrimitiveType::Int
                    | PrimitiveType::UInt
                    | PrimitiveType::Long
                    | PrimitiveType::ULong
                    | PrimitiveType::String
            } | ResolvedAuthoringType::ValueObject {
                underlying: PrimitiveType::Int
                    | PrimitiveType::UInt
                    | PrimitiveType::Long
                    | PrimitiveType::ULong
                    | PrimitiveType::String,
                ..
            }
        )
}

fn is_string_shape(shape: &ResolvedAuthoringField) -> bool {
    matches!(
        shape.shape,
        ResolvedAuthoringType::Primitive {
            primitive: PrimitiveType::String
        } | ResolvedAuthoringType::ValueObject {
            underlying: PrimitiveType::String,
            ..
        }
    )
}

fn is_integer_shape(shape: &ResolvedAuthoringField) -> bool {
    matches!(
        shape.shape,
        ResolvedAuthoringType::Primitive {
            primitive: PrimitiveType::Int
                | PrimitiveType::UInt
                | PrimitiveType::Long
                | PrimitiveType::ULong
        } | ResolvedAuthoringType::ValueObject {
            underlying: PrimitiveType::Int
                | PrimitiveType::UInt
                | PrimitiveType::Long
                | PrimitiveType::ULong,
            ..
        }
    )
}

fn compare_sort_values(
    shape: &ResolvedAuthoringField,
    left: &AuthoringValue,
    right: &AuthoringValue,
) -> Result<Ordering> {
    let left_state = value_state(shape, left);
    let right_state = value_state(shape, right);
    if left_state != right_state {
        return Ok(match (left_state, right_state) {
            (ValueState::Valid, _) => Ordering::Less,
            (_, ValueState::Valid) => Ordering::Greater,
            (ValueState::Null, ValueState::Invalid) => Ordering::Less,
            (ValueState::Invalid, ValueState::Null) => Ordering::Greater,
            _ => Ordering::Equal,
        });
    }
    if left_state != ValueState::Valid {
        return Ok(Ordering::Equal);
    }
    compare_valid_values(shape, left, right)
}

fn compare_valid_values(
    shape: &ResolvedAuthoringField,
    left: &AuthoringValue,
    right: &AuthoringValue,
) -> Result<Ordering> {
    let primitive = match &shape.shape {
        ResolvedAuthoringType::Primitive { primitive } => *primitive,
        ResolvedAuthoringType::ValueObject { underlying, .. } => *underlying,
        ResolvedAuthoringType::Enum { .. } => {
            return Ok(utf16_cmp(
                scalar_string(left).unwrap_or_default(),
                scalar_string(right).unwrap_or_default(),
            ));
        }
        ResolvedAuthoringType::Flags { .. } | ResolvedAuthoringType::Custom { .. } => {
            return Err(query_error(
                "E-AUTHORING-QUERY-OPERATOR-UNSUPPORTED",
                format!("field `{}` is not comparable for this operator", shape.name),
            ));
        }
    };
    match primitive {
        PrimitiveType::Int | PrimitiveType::UInt | PrimitiveType::Long | PrimitiveType::ULong => {
            let left = scalar_string(left)
                .and_then(|value| value.parse::<i128>().ok())
                .ok_or_else(|| {
                    query_error("E-AUTHORING-QUERY-INPUT", "numeric query value is invalid")
                })?;
            let right = scalar_string(right)
                .and_then(|value| value.parse::<i128>().ok())
                .ok_or_else(|| {
                    query_error("E-AUTHORING-QUERY-INPUT", "numeric query value is invalid")
                })?;
            Ok(left.cmp(&right))
        }
        PrimitiveType::String => Ok(utf16_cmp(
            scalar_string(left).unwrap_or_default(),
            scalar_string(right).unwrap_or_default(),
        )),
        PrimitiveType::Bool => match (left, right) {
            (AuthoringValue::Bool { value: left }, AuthoringValue::Bool { value: right }) => {
                Ok(left.cmp(right))
            }
            _ => Err(query_error(
                "E-AUTHORING-QUERY-INPUT",
                "boolean query value is invalid",
            )),
        },
        PrimitiveType::Float | PrimitiveType::Double => Err(query_error(
            "E-AUTHORING-QUERY-OPERATOR-UNSUPPORTED",
            format!(
                "field `{}` does not support this numeric/string query",
                shape.name
            ),
        )),
    }
}

fn utf16_cmp(left: &str, right: &str) -> Ordering {
    left.encode_utf16().cmp(right.encode_utf16())
}

fn query_error(code: &str, message: impl Into<String>) -> MasterdataError {
    MasterdataError::new(code, ErrorKind::Validation, message)
        .with_related_requirement("AUTHORING-QUERY-002")
}

#[cfg(test)]
mod tests {
    use super::{
        AuthoringQuery, ColumnFilter, QueryOperator, QueryRow, QuerySort, SortDirection,
        apply_authoring_query,
    };
    use crate::{
        AuthoringValue, FieldModifier, PrimitiveType, ResolvedAuthoringField, ResolvedAuthoringType,
    };

    fn field(name: &str, primitive: PrimitiveType) -> ResolvedAuthoringField {
        ResolvedAuthoringField {
            name: name.into(),
            type_name: primitive.name().into(),
            modifier: FieldModifier::Required,
            shape: ResolvedAuthoringType::Primitive { primitive },
        }
    }
    fn string_field() -> ResolvedAuthoringField {
        field("name", PrimitiveType::String)
    }
    fn nullable_field(name: &str, primitive: PrimitiveType) -> ResolvedAuthoringField {
        let mut field = field(name, primitive);
        field.modifier = FieldModifier::Nullable;
        field
    }
    fn string(value: &str) -> AuthoringValue {
        AuthoringValue::String {
            value: value.into(),
        }
    }
    fn number(value: &str) -> AuthoringValue {
        AuthoringValue::Number {
            value: value.into(),
        }
    }

    #[test]
    fn search_is_or_and_filters_are_and() {
        let columns = vec![string_field(), field("id", PrimitiveType::ULong)];
        let rows = vec![
            QueryRow {
                source_order: 0,
                values: vec![string("alpha"), number("10")],
            },
            QueryRow {
                source_order: 1,
                values: vec![string("beta"), number("20")],
            },
        ];
        let query = AuthoringQuery {
            search: "a".into(),
            filters: vec![ColumnFilter {
                field: "id".into(),
                operator: QueryOperator::GreaterThan,
                value: Some(number("15")),
            }],
            sort: None,
        };
        assert_eq!(
            apply_authoring_query(&columns, &rows, &query).unwrap(),
            vec![1]
        );
    }

    #[test]
    fn sort_keeps_null_and_invalid_groups_stable_and_reverses_valid_only() {
        let columns = vec![field("id", PrimitiveType::Long)];
        let rows = vec![
            QueryRow {
                source_order: 0,
                values: vec![number("2")],
            },
            QueryRow {
                source_order: 1,
                values: vec![AuthoringValue::Null],
            },
            QueryRow {
                source_order: 2,
                values: vec![string("bad")],
            },
            QueryRow {
                source_order: 3,
                values: vec![number("1")],
            },
        ];
        let asc = AuthoringQuery {
            sort: Some(QuerySort {
                field: "id".into(),
                direction: SortDirection::Ascending,
            }),
            ..Default::default()
        };
        assert_eq!(
            apply_authoring_query(&columns, &rows, &asc).unwrap(),
            vec![3, 0, 1, 2]
        );
        let desc = AuthoringQuery {
            sort: Some(QuerySort {
                field: "id".into(),
                direction: SortDirection::Descending,
            }),
            ..Default::default()
        };
        assert_eq!(
            apply_authoring_query(&columns, &rows, &desc).unwrap(),
            vec![0, 3, 1, 2]
        );
    }

    #[test]
    fn null_and_invalid_filters_are_distinct() {
        let columns = vec![nullable_field("id", PrimitiveType::Int)];
        let rows = vec![
            QueryRow {
                source_order: 0,
                values: vec![AuthoringValue::Null],
            },
            QueryRow {
                source_order: 1,
                values: vec![string("bad")],
            },
        ];
        let null = AuthoringQuery {
            filters: vec![ColumnFilter {
                field: "id".into(),
                operator: QueryOperator::IsNull,
                value: None,
            }],
            ..Default::default()
        };
        let invalid = AuthoringQuery {
            filters: vec![ColumnFilter {
                field: "id".into(),
                operator: QueryOperator::IsInvalid,
                value: None,
            }],
            ..Default::default()
        };
        assert_eq!(
            apply_authoring_query(&columns, &rows, &null).unwrap(),
            vec![0]
        );
        assert_eq!(
            apply_authoring_query(&columns, &rows, &invalid).unwrap(),
            vec![1]
        );
    }

    #[test]
    fn required_null_is_visible_to_both_null_and_invalid_filters() {
        let columns = vec![field("id", PrimitiveType::Int)];
        let rows = vec![QueryRow {
            source_order: 0,
            values: vec![AuthoringValue::Null],
        }];
        for operator in [QueryOperator::IsNull, QueryOperator::IsInvalid] {
            let query = AuthoringQuery {
                filters: vec![ColumnFilter {
                    field: "id".into(),
                    operator,
                    value: None,
                }],
                ..Default::default()
            };
            assert_eq!(
                apply_authoring_query(&columns, &rows, &query).unwrap(),
                vec![0]
            );
        }
    }

    #[test]
    fn boolean_equality_and_search_use_shared_scalar_semantics() {
        let columns = vec![field("enabled", PrimitiveType::Bool)];
        let rows = vec![
            QueryRow {
                source_order: 0,
                values: vec![AuthoringValue::Bool { value: true }],
            },
            QueryRow {
                source_order: 1,
                values: vec![AuthoringValue::Bool { value: false }],
            },
        ];
        let query = AuthoringQuery {
            search: "true".into(),
            filters: vec![ColumnFilter {
                field: "enabled".into(),
                operator: QueryOperator::Equals,
                value: Some(AuthoringValue::Bool { value: true }),
            }],
            ..Default::default()
        };
        assert_eq!(
            apply_authoring_query(&columns, &rows, &query).unwrap(),
            vec![0]
        );
    }
}
