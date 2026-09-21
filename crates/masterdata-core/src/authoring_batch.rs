//! Shared clipboard and scalar batch-authoring semantics.
//!
//! The editor deliberately exchanges clipboard values as text.  Keeping the
//! codec and the type-directed conversion here prevents the Tauri frontend
//! from accidentally applying spreadsheet or JavaScript semantics to
//! source values.

use crate::authoring_value::AuthoringValue;
use crate::error::{ErrorKind, MasterdataError, Result};
use crate::type_system::{
    FieldModifier, PrimitiveType, ResolvedAuthoringField, ResolvedAuthoringType,
};

/// Decode the v1 rectangular TSV clipboard format.
pub fn decode_clipboard_tsv(input: &str) -> Result<Vec<Vec<String>>> {
    if input.is_empty() {
        return Ok(vec![vec![String::new()]]);
    }

    #[derive(Clone, Copy, PartialEq, Eq)]
    enum State {
        FieldStart,
        Unquoted,
        Quoted,
        AfterQuote,
    }

    let bytes = input.as_bytes();
    let mut state = State::FieldStart;
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut index = 0;

    while index < bytes.len() {
        match state {
            State::FieldStart => match bytes[index] {
                b'"' => {
                    state = State::Quoted;
                    index += 1;
                }
                b'\t' => {
                    row.push(String::new());
                    index += 1;
                }
                b'\n' => {
                    row.push(String::new());
                    rows.push(std::mem::take(&mut row));
                    index += 1;
                }
                b'\r' => {
                    if bytes.get(index + 1) != Some(&b'\n') {
                        return Err(codec_error("bare CR is only valid inside a quoted field"));
                    }
                    row.push(String::new());
                    rows.push(std::mem::take(&mut row));
                    index += 2;
                }
                _ => {
                    let (character, width) = next_character(input, index)?;
                    field.push(character);
                    state = State::Unquoted;
                    index += width;
                }
            },
            State::Unquoted => match bytes[index] {
                b'"' => return Err(codec_error("a quote in an unquoted field is malformed")),
                b'\t' => {
                    row.push(std::mem::take(&mut field));
                    state = State::FieldStart;
                    index += 1;
                }
                b'\n' => {
                    row.push(std::mem::take(&mut field));
                    rows.push(std::mem::take(&mut row));
                    state = State::FieldStart;
                    index += 1;
                }
                b'\r' => {
                    if bytes.get(index + 1) != Some(&b'\n') {
                        return Err(codec_error("bare CR is only valid inside a quoted field"));
                    }
                    row.push(std::mem::take(&mut field));
                    rows.push(std::mem::take(&mut row));
                    state = State::FieldStart;
                    index += 2;
                }
                _ => {
                    let (character, width) = next_character(input, index)?;
                    field.push(character);
                    index += width;
                }
            },
            State::Quoted => match bytes[index] {
                b'"' => {
                    if bytes.get(index + 1) == Some(&b'"') {
                        field.push('"');
                        index += 2;
                    } else {
                        state = State::AfterQuote;
                        index += 1;
                    }
                }
                _ => {
                    let (character, width) = next_character(input, index)?;
                    field.push(character);
                    index += width;
                }
            },
            State::AfterQuote => match bytes[index] {
                b'\t' => {
                    row.push(std::mem::take(&mut field));
                    state = State::FieldStart;
                    index += 1;
                }
                b'\n' => {
                    row.push(std::mem::take(&mut field));
                    rows.push(std::mem::take(&mut row));
                    state = State::FieldStart;
                    index += 1;
                }
                b'\r' if bytes.get(index + 1) == Some(&b'\n') => {
                    row.push(std::mem::take(&mut field));
                    rows.push(std::mem::take(&mut row));
                    state = State::FieldStart;
                    index += 2;
                }
                _ => {
                    return Err(codec_error(
                        "characters after a closing quote are malformed",
                    ));
                }
            },
        }
    }

    if state == State::Quoted {
        return Err(codec_error("quoted field was not terminated"));
    }
    // A single final row delimiter terminates the last row.  FieldStart with
    // an empty row means the delimiter was already consumed, so do not create
    // the forbidden extra empty row.  A trailing TAB is different: it leaves
    // an empty final field and must be retained.
    if state != State::FieldStart || !row.is_empty() {
        row.push(field);
        rows.push(row);
    } else if rows.is_empty() {
        rows.push(vec![String::new()]);
    }
    Ok(rows)
}

/// Encode every clipboard field as a quoted TSV field.  No final row
/// delimiter is emitted, which makes empty trailing fields unambiguous.
pub fn encode_clipboard_tsv(rows: &[Vec<String>]) -> Result<String> {
    if rows.is_empty() {
        return Err(codec_error("cannot encode an empty clipboard rectangle"));
    }
    if rows.iter().any(Vec::is_empty) {
        return Err(codec_error(
            "clipboard rows must contain at least one field",
        ));
    }
    Ok(rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|value| quote_tsv_field(value))
                .collect::<Vec<_>>()
                .join("\t")
        })
        .collect::<Vec<_>>()
        .join("\n"))
}

/// Whether a top-level field belongs to the scalar range-edit capability.
pub fn is_scalar_batch_field(shape: &ResolvedAuthoringField) -> bool {
    matches!(
        shape.modifier,
        FieldModifier::Required | FieldModifier::Nullable
    ) && matches!(
        shape.shape,
        ResolvedAuthoringType::Primitive { .. }
            | ResolvedAuthoringType::ValueObject { .. }
            | ResolvedAuthoringType::Enum { .. }
    )
}

/// Convert one decoded clipboard field according to the resolved scalar shape.
/// Invalid domain input is intentionally returned as a string so the normal
/// validation path can report it without turning a batch operation into a
/// partial mutation.
pub fn authoring_value_from_clipboard(
    shape: &ResolvedAuthoringField,
    text: &str,
) -> Result<AuthoringValue> {
    if !is_scalar_batch_field(shape) {
        return Err(MasterdataError::new(
            "E-AUTHORING-BATCH-UNSUPPORTED-FIELD",
            ErrorKind::Validation,
            format!("field `{}` is outside scalar batch authoring", shape.name),
        )
        .with_related_requirement("AUTHORING-BATCH-001"));
    }
    if text.is_empty() {
        return Ok(AuthoringValue::String {
            value: String::new(),
        });
    }

    let primitive = match &shape.shape {
        ResolvedAuthoringType::Primitive { primitive } => *primitive,
        ResolvedAuthoringType::ValueObject { underlying, .. } => *underlying,
        ResolvedAuthoringType::Enum { .. } => PrimitiveType::String,
        ResolvedAuthoringType::Flags { .. } | ResolvedAuthoringType::Custom { .. } => {
            unreachable!("is_scalar_batch_field checked the shape")
        }
    };
    match primitive {
        PrimitiveType::String => Ok(AuthoringValue::String {
            value: text.to_owned(),
        }),
        PrimitiveType::Bool if matches!(text, "true" | "false") => Ok(AuthoringValue::Bool {
            value: text == "true",
        }),
        PrimitiveType::Bool => Ok(literal_text(text)),
        primitive if primitive.is_integer() && valid_integer_token(text, primitive) => {
            Ok(AuthoringValue::Number {
                value: text.to_owned(),
            })
        }
        primitive
            if matches!(primitive, PrimitiveType::Float | PrimitiveType::Double)
                && valid_float_token(text) =>
        {
            Ok(AuthoringValue::Number {
                value: text.to_owned(),
            })
        }
        _ => Ok(literal_text(text)),
    }
}

/// Render one valid, non-null scalar value for the shared clipboard codec.
/// The caller may still copy key fields, but invalid, nullable-null, and
/// complex values fail closed instead of becoming an ambiguous empty cell.
pub fn authoring_value_to_clipboard(
    shape: &ResolvedAuthoringField,
    value: &AuthoringValue,
) -> Result<String> {
    if !is_scalar_batch_field(shape) {
        return Err(copy_error(
            "E-AUTHORING-BATCH-COPY-UNSUPPORTED",
            format!("field `{}` is outside scalar batch authoring", shape.name),
        ));
    }
    let primitive = match &shape.shape {
        ResolvedAuthoringType::Primitive { primitive } => *primitive,
        ResolvedAuthoringType::ValueObject { underlying, .. } => *underlying,
        ResolvedAuthoringType::Enum { .. } => PrimitiveType::String,
        ResolvedAuthoringType::Flags { .. } | ResolvedAuthoringType::Custom { .. } => {
            unreachable!("is_scalar_batch_field checked the shape")
        }
    };
    match (primitive, &shape.shape, value) {
        (_, _, AuthoringValue::Null)
        | (_, _, AuthoringValue::Sequence { .. })
        | (_, _, AuthoringValue::Mapping { .. }) => Err(copy_error(
            "E-AUTHORING-BATCH-COPY-INVALID",
            format!("field `{}` is null, invalid, or complex", shape.name),
        )),
        (
            PrimitiveType::String,
            ResolvedAuthoringType::Enum { members, .. },
            AuthoringValue::String { value },
        ) => {
            if members.iter().any(|member| member == value) {
                Ok(value.clone())
            } else {
                Err(copy_error(
                    "E-AUTHORING-BATCH-COPY-INVALID",
                    format!("field `{}` contains an unknown enum member", shape.name),
                ))
            }
        }
        (PrimitiveType::String, _, AuthoringValue::String { value }) => Ok(value.clone()),
        (PrimitiveType::Bool, _, AuthoringValue::Bool { value }) => Ok(value.to_string()),
        (primitive, _, AuthoringValue::Number { value }) if primitive.is_integer() => {
            if valid_integer_token(value, primitive) {
                Ok(value.clone())
            } else {
                Err(copy_error(
                    "E-AUTHORING-BATCH-COPY-INVALID",
                    format!("field `{}` contains an invalid integer", shape.name),
                ))
            }
        }
        (PrimitiveType::Float | PrimitiveType::Double, _, AuthoringValue::Number { value }) => {
            if valid_float_token(value) {
                Ok(value.clone())
            } else if let Ok(number) = value.parse::<f64>() {
                if number.is_finite() {
                    // A float that came from YAML as an integer still needs a
                    // float lexical form when pasted back into a scalar cell.
                    Ok(format!("{value}.0"))
                } else {
                    Err(copy_error(
                        "E-AUTHORING-BATCH-COPY-INVALID",
                        format!("field `{}` contains a non-finite float", shape.name),
                    ))
                }
            } else {
                Err(copy_error(
                    "E-AUTHORING-BATCH-COPY-INVALID",
                    format!("field `{}` contains an invalid float", shape.name),
                ))
            }
        }
        (_, _, _) => Err(copy_error(
            "E-AUTHORING-BATCH-COPY-INVALID",
            format!(
                "field `{}` contains a value outside its resolved type",
                shape.name
            ),
        )),
    }
}

fn literal_text(text: &str) -> AuthoringValue {
    AuthoringValue::String {
        value: text.to_owned(),
    }
}

fn valid_integer_token(text: &str, primitive: PrimitiveType) -> bool {
    let digits = text.strip_prefix('-').unwrap_or(text);
    if digits.is_empty()
        || (digits.len() > 1 && digits.starts_with('0'))
        || !digits.bytes().all(|byte| byte.is_ascii_digit())
    {
        return false;
    }
    if !primitive.is_signed_integer() && text.starts_with('-') {
        return false;
    }
    let Ok(value) = text.parse::<i128>() else {
        return false;
    };
    primitive
        .integer_range()
        .is_some_and(|(minimum, maximum)| (minimum..=maximum).contains(&value))
}

fn valid_float_token(text: &str) -> bool {
    if text.starts_with('+') || text.contains(['_', ' ']) || text.is_empty() {
        return false;
    }
    let text = text.strip_prefix('-').unwrap_or(text);
    let (mantissa, exponent) = match text.find(['e', 'E']) {
        Some(index) if text[index + 1..].find(['e', 'E']).is_none() => {
            (&text[..index], Some(&text[index + 1..]))
        }
        Some(_) => return false,
        None => (text, None),
    };
    if let Some(exponent) = exponent {
        let exponent = exponent.strip_prefix(['+', '-']).unwrap_or(exponent);
        if exponent.is_empty() || !exponent.bytes().all(|byte| byte.is_ascii_digit()) {
            return false;
        }
    }
    let valid_mantissa = match mantissa.split_once('.') {
        Some((whole, fraction)) => {
            !whole.is_empty()
                && !fraction.is_empty()
                && whole.bytes().all(|byte| byte.is_ascii_digit())
                && fraction.bytes().all(|byte| byte.is_ascii_digit())
        }
        None => exponent.is_some() && mantissa.bytes().all(|byte| byte.is_ascii_digit()),
    };
    valid_mantissa && text.parse::<f64>().is_ok_and(f64::is_finite)
}

fn next_character(input: &str, index: usize) -> Result<(char, usize)> {
    input[index..]
        .chars()
        .next()
        .map(|character| (character, character.len_utf8()))
        .ok_or_else(|| codec_error("clipboard contains invalid UTF-8"))
}

fn quote_tsv_field(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn codec_error(message: &str) -> MasterdataError {
    MasterdataError::new(
        "E-AUTHORING-CLIPBOARD-CODEC",
        ErrorKind::Validation,
        message,
    )
    .with_related_requirement("AUTHORING-BATCH-003")
}

fn copy_error(code: &str, message: String) -> MasterdataError {
    MasterdataError::new(code, ErrorKind::Validation, message)
        .with_related_requirement("AUTHORING-BATCH-004")
}

#[cfg(test)]
mod tests {
    use super::{
        authoring_value_from_clipboard, authoring_value_to_clipboard, decode_clipboard_tsv,
        encode_clipboard_tsv,
    };
    use crate::{FieldModifier, PrimitiveType, ResolvedAuthoringField, ResolvedAuthoringType};

    fn field(primitive: PrimitiveType) -> ResolvedAuthoringField {
        ResolvedAuthoringField {
            name: "value".into(),
            type_name: primitive.name().into(),
            modifier: FieldModifier::Required,
            shape: ResolvedAuthoringType::Primitive { primitive },
        }
    }

    #[test]
    fn codec_preserves_quotes_tabs_newlines_crlf_and_terminal_fields() {
        let input = "\"a\tb\"\t\"line1\nline2\r\nline3\"\t\"quote \"\"x\"\"\"\t\nnext\t";
        let decoded = decode_clipboard_tsv(input).expect("decode");
        assert_eq!(decoded[0][0], "a\tb");
        assert_eq!(decoded[0][1], "line1\nline2\r\nline3");
        assert_eq!(decoded[0][2], "quote \"x\"");
        assert_eq!(decoded[0][3], "");
        assert_eq!(decoded[1], vec!["next", ""]);
        assert_eq!(decode_clipboard_tsv("a\n").unwrap(), vec![vec!["a"]]);
        assert_eq!(
            decode_clipboard_tsv("a\n\n").unwrap(),
            vec![vec!["a"], vec![""]]
        );
    }

    #[test]
    fn codec_rejects_ragged_is_not_implicit_and_quotes_are_strict() {
        assert!(decode_clipboard_tsv("a\rb").is_err());
        assert!(decode_clipboard_tsv("a\"b").is_err());
        assert!(decode_clipboard_tsv("\"a\"x").is_err());
        assert!(decode_clipboard_tsv("\"a").is_err());
        let rows = decode_clipboard_tsv("a\tb\nc").unwrap();
        assert_ne!(rows[0].len(), rows[1].len());
    }

    #[test]
    fn encode_quotes_every_field_and_round_trips_empty_text() {
        let rows = vec![vec!["".into(), "001".into(), "=SUM(A1)".into()]];
        let encoded = encode_clipboard_tsv(&rows).unwrap();
        assert_eq!(encoded, "\"\"\t\"001\"\t\"=SUM(A1)\"");
        assert_eq!(decode_clipboard_tsv(&encoded).unwrap(), rows);
    }

    #[test]
    fn typed_conversion_keeps_invalid_domain_input_literal() {
        assert_eq!(
            authoring_value_from_clipboard(&field(PrimitiveType::ULong), "18446744073709551615")
                .unwrap(),
            crate::AuthoringValue::Number {
                value: "18446744073709551615".into()
            }
        );
        assert_eq!(
            authoring_value_from_clipboard(&field(PrimitiveType::Int), "1.0").unwrap(),
            crate::AuthoringValue::String {
                value: "1.0".into()
            }
        );
        assert_eq!(
            authoring_value_from_clipboard(&field(PrimitiveType::String), "null").unwrap(),
            crate::AuthoringValue::String {
                value: "null".into()
            }
        );
    }

    #[test]
    fn copy_rejects_invalid_values_and_preserves_float_category() {
        assert_eq!(
            authoring_value_to_clipboard(
                &field(PrimitiveType::Double),
                &crate::AuthoringValue::Number { value: "1".into() },
            )
            .unwrap(),
            "1.0"
        );
        assert!(
            authoring_value_to_clipboard(&field(PrimitiveType::Int), &crate::AuthoringValue::Null)
                .is_err()
        );
        assert!(
            authoring_value_to_clipboard(
                &ResolvedAuthoringField {
                    name: "kind".into(),
                    type_name: "Kind".into(),
                    modifier: FieldModifier::Required,
                    shape: ResolvedAuthoringType::Enum {
                        name: "Kind".into(),
                        underlying: PrimitiveType::Int,
                        members: vec!["Known".into()],
                    },
                },
                &crate::AuthoringValue::String {
                    value: "Unknown".into()
                },
            )
            .is_err()
        );
    }
}
