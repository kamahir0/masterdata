//! The shared, lossless TSV codec. Clipboard syntax never selects YAML semantics.
use crate::{Error, Result};

pub fn decode(text: &str) -> Result<Vec<Vec<String>>> {
    let chars: Vec<char> = text.chars().collect();
    let mut at = 0;
    let mut rows = vec![];
    let mut row = vec![];
    loop {
        let mut field = String::new();
        if chars.get(at) == Some(&'"') {
            at += 1;
            loop {
                match chars.get(at) {
                    None => return Err(codec_error("unclosed quoted field")),
                    Some('"') if chars.get(at + 1) == Some(&'"') => {
                        field.push('"');
                        at += 2;
                    }
                    Some('"') => {
                        at += 1;
                        break;
                    }
                    Some(c) => {
                        field.push(*c);
                        at += 1;
                    }
                }
            }
        } else {
            while let Some(c) = chars.get(at) {
                match c {
                    '\t' | '\n' | '\r' => break,
                    '"' => return Err(codec_error("quote inside unquoted field")),
                    _ => {
                        field.push(*c);
                        at += 1;
                    }
                }
            }
        }
        row.push(field);
        match chars.get(at) {
            None => {
                rows.push(row);
                break;
            }
            Some('\t') => {
                at += 1;
            }
            Some('\n') => {
                at += 1;
                rows.push(std::mem::take(&mut row));
                if at == chars.len() {
                    break;
                }
            }
            Some('\r') if chars.get(at + 1) == Some(&'\n') => {
                at += 2;
                rows.push(std::mem::take(&mut row));
                if at == chars.len() {
                    break;
                }
            }
            _ => return Err(codec_error("invalid delimiter or text after closing quote")),
        }
    }
    if rows.iter().any(|r| r.len() != rows[0].len()) {
        return Err(codec_error("ragged rectangle"));
    }
    Ok(rows)
}
pub fn encode(rows: &[Vec<String>]) -> Result<String> {
    let Some(first) = rows.first() else {
        return Err(codec_error("empty rectangle"));
    };
    if first.is_empty() || rows.iter().any(|r| r.len() != first.len()) {
        return Err(codec_error("ragged rectangle"));
    }
    Ok(rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|v| format!("\"{}\"", v.replace('"', "\"\"")))
                .collect::<Vec<_>>()
                .join("\t")
        })
        .collect::<Vec<_>>()
        .join("\n"))
}
fn codec_error(message: &str) -> Error {
    Error::new("E-CLIPBOARD-CODEC", message)
}
