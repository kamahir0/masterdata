use serde::{Deserialize, Serialize};
use serde_yaml::Value;

use crate::type_system::{FieldModifier, ResolvedAuthoringField, ResolvedAuthoringType};
use crate::{ErrorKind, MasterdataError, Result};

/// A source value passed between the shared application and an authoring
/// surface. Numbers are decimal text so JavaScript never rounds 64-bit values.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AuthoringValue {
    Null,
    Invalid {
        diagnostic: Box<crate::Diagnostic>,
    },
    Bool {
        value: bool,
    },
    Number {
        value: String,
    },
    String {
        value: String,
    },
    Sequence {
        items: Vec<AuthoringSequenceItem>,
        // WHY: sequence-level identity distinguishes an explicit empty result from an
        // older value-only request, even when no item remains to carry sourceIndex.
        // EVIDENCE: sequence_requests_without_identity_metadata_keep_legacy_semantics.
        // Regression: clearing_duplicate_array_items_uses_tracked_empty_sequence_identity.
        #[serde(rename = "sourceIdentity", default)]
        source_identity: bool,
    },
    Mapping {
        entries: Vec<AuthoringMember>,
    },
}

/// An authoring sequence item retains its source occurrence while the UI
/// edits or reorders the value tree. Added items have no source index.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AuthoringSequenceItem {
    #[serde(default)]
    pub source_index: Option<usize>,
    pub value: AuthoringValue,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AuthoringMember {
    pub name: String,
    pub value: AuthoringValue,
}

pub fn project_source_value(value: &Value) -> Result<AuthoringValue> {
    match value {
        Value::Null => Ok(AuthoringValue::Null),
        Value::Bool(value) => Ok(AuthoringValue::Bool { value: *value }),
        Value::Number(value) => Ok(AuthoringValue::Number {
            value: value.to_string(),
        }),
        Value::String(value) => Ok(AuthoringValue::String {
            value: value.clone(),
        }),
        Value::Sequence(items) => Ok(AuthoringValue::Sequence {
            items: items
                .iter()
                .enumerate()
                .map(|(source_index, value)| {
                    Ok(AuthoringSequenceItem {
                        source_index: Some(source_index),
                        value: project_source_value(value)?,
                    })
                })
                .collect::<Result<Vec<_>>>()?,
            source_identity: true,
        }),
        Value::Mapping(entries) => {
            let mut projected = Vec::with_capacity(entries.len());
            for (key, value) in entries {
                let Some(name) = key.as_str() else {
                    return Err(unsupported_authoring_value(
                        "Custom Type mappings must use string member names",
                    ));
                };
                projected.push(AuthoringMember {
                    name: name.to_owned(),
                    value: project_source_value(value)?,
                });
            }
            Ok(AuthoringValue::Mapping { entries: projected })
        }
        Value::Tagged(_) => Err(unsupported_authoring_value(
            "tagged YAML values cannot be projected as typed authoring values",
        )),
    }
}

pub fn project_typed_source_value(
    shape: &ResolvedAuthoringField,
    value: &Value,
) -> Result<AuthoringValue> {
    if value.is_null() {
        return Ok(AuthoringValue::Null);
    }
    match shape.modifier {
        FieldModifier::Array => {
            let Some(items) = value.as_sequence() else {
                return Err(unsupported_authoring_value(
                    "Array source value is not a sequence",
                ));
            };
            Ok(AuthoringValue::Sequence {
                items: items
                    .iter()
                    .enumerate()
                    .map(|(source_index, item)| {
                        Ok(AuthoringSequenceItem {
                            source_index: Some(source_index),
                            value: project_typed_source_type(&shape.shape, item)?,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?,
                source_identity: true,
            })
        }
        FieldModifier::Required | FieldModifier::Nullable => {
            project_typed_source_type(&shape.shape, value)
        }
    }
}

fn project_typed_source_type(
    shape: &ResolvedAuthoringType,
    value: &Value,
) -> Result<AuthoringValue> {
    match shape {
        ResolvedAuthoringType::Primitive { .. }
        | ResolvedAuthoringType::ValueObject { .. }
        | ResolvedAuthoringType::Enum { .. } => {
            if value.is_sequence() || value.is_mapping() || matches!(value, Value::Tagged(_)) {
                return Err(unsupported_authoring_value(
                    "scalar source value has a collection shape",
                ));
            }
            project_source_value(value)
        }
        ResolvedAuthoringType::Flags { .. } => {
            let Some(items) = value.as_sequence() else {
                return Err(unsupported_authoring_value(
                    "Flags source value is not a sequence",
                ));
            };
            let mut projected = Vec::with_capacity(items.len());
            for (source_index, item) in items.iter().enumerate() {
                if item.is_sequence() || item.is_mapping() || matches!(item, Value::Tagged(_)) {
                    return Err(unsupported_authoring_value(
                        "Flags source member is not a scalar",
                    ));
                }
                projected.push(AuthoringSequenceItem {
                    source_index: Some(source_index),
                    value: project_source_value(item)?,
                });
            }
            Ok(AuthoringValue::Sequence {
                items: projected,
                source_identity: true,
            })
        }
        ResolvedAuthoringType::Custom { fields, .. } => {
            let Some(mapping) = value.as_mapping() else {
                return Err(unsupported_authoring_value(
                    "Custom Type source value is not a mapping",
                ));
            };
            let mut projected = Vec::with_capacity(mapping.len());
            for (key, value) in mapping {
                let Some(name) = key.as_str() else {
                    return Err(unsupported_authoring_value(
                        "Custom Type mappings must use string member names",
                    ));
                };
                let value = match fields.iter().find(|field| field.name == name) {
                    Some(field) => project_typed_source_value(field, value)?,
                    None => project_source_value(value)?,
                };
                projected.push(AuthoringMember {
                    name: name.to_owned(),
                    value,
                });
            }
            Ok(AuthoringValue::Mapping { entries: projected })
        }
    }
}

fn unsupported_authoring_value(message: &str) -> MasterdataError {
    MasterdataError::new(
        "E-SOURCE-EDIT-VALUE-UNSUPPORTED",
        ErrorKind::Validation,
        message,
    )
    .with_related_requirement("SOURCE-EDIT-015")
}

#[cfg(test)]
mod tests {
    use super::{AuthoringValue, project_source_value};
    use serde_yaml::Value;

    #[test]
    fn sequence_projection_serializes_stable_source_occurrences_for_the_ui() {
        let projected = project_source_value(&Value::Sequence(vec![Value::String("first".into())]))
            .expect("plain sequence values can be projected");

        assert_eq!(
            serde_json::to_value(projected).expect("authoring values serialize to JSON"),
            serde_json::json!({
                "kind": "sequence",
                "sourceIdentity": true,
                "items": [{
                    "sourceIndex": 0,
                    "value": { "kind": "string", "value": "first" }
                }]
            })
        );
    }

    #[test]
    fn sequence_requests_without_identity_metadata_keep_legacy_semantics() {
        let request = serde_json::json!({ "kind": "sequence", "items": [] });
        let decoded: AuthoringValue =
            serde_json::from_value(request).expect("legacy sequence requests still deserialize");

        assert_eq!(
            decoded,
            AuthoringValue::Sequence {
                items: Vec::new(),
                source_identity: false,
            }
        );
    }
}
