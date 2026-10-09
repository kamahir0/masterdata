//! Temporary typed constant input. It shares scalar authoring and final semantic
//! interpretation with the data editor; it never becomes a physical draft.
use crate::{
    Error, Result,
    semantic::{self, Field, Shape, Types},
    source::Value,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Input {
    Unset,
    Null,
    Scalar { text: String },
    Sequence { value: Vec<Input> },
    Mapping { value: Vec<(String, Input)> },
}
fn invalid(message: impl Into<String>) -> Error {
    Error::new("E-INITIALIZER", message)
}
fn build(shape: &Shape, input: &Input) -> Result<Value> {
    match input {
        Input::Unset => Err(invalid("explicit constant initializer is not entered")),
        Input::Null => Ok(Value::Null),
        Input::Scalar { text }
            if !shape.array && !matches!(shape.category.as_str(), "custom" | "flags") =>
        {
            Ok(semantic::authoring_input(shape, text))
        }
        Input::Sequence { value } if shape.array => {
            let mut element = shape.clone();
            element.array = false;
            element.nullable = false;
            Ok(Value::Sequence(
                value
                    .iter()
                    .map(|input| build(&element, input))
                    .collect::<Result<_>>()?,
            ))
        }
        Input::Sequence { value } if shape.category == "flags" => Ok(Value::Sequence(
            value
                .iter()
                .map(|input| match input {
                    Input::Scalar { text } => Ok(Value::Text(text.clone())),
                    _ => Err(invalid("Flags initializer needs member symbols")),
                })
                .collect::<Result<_>>()?,
        )),
        Input::Mapping { value } if !shape.array && shape.category == "custom" => {
            let mut names = std::collections::BTreeSet::new();
            let mut fields = vec![];
            for (name, input) in value {
                if !names.insert(name) {
                    return Err(invalid("duplicate initializer member"));
                }
                let (_, child) = shape
                    .fields
                    .iter()
                    .find(|(field, _)| field.name == *name)
                    .ok_or_else(|| invalid(format!("{name}: unknown initializer member")))?;
                fields.push((name.clone(), build(child, input)?));
            }
            Ok(Value::Mapping(fields))
        }
        _ => Err(invalid(
            "initializer input shape does not match the resolved field",
        )),
    }
}
pub fn resolve(field: &Field, input: &Input, types: &Types) -> Result<Value> {
    let value = build(&semantic::shape(field, types)?, input)?;
    crate::migration::constant(field, &value, types)?;
    Ok(value)
}
