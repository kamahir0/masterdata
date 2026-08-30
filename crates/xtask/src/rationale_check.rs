use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use masterdata_core::{ErrorKind, MasterdataError, Result};

use crate::spec_check::{canonical_requirement_ids, is_requirement_id};

const SOURCE_ROOTS: &[&str] = &["crates", "apps", "dotnet"];
const SOURCE_EXTENSIONS: &[&str] = &["rs", "cs", "ts", "tsx", "js", "jsx", "mjs", "cjs"];

#[derive(Debug, Default)]
pub struct RationaleCheckSummary {
    pub source_files: usize,
    pub rationale_blocks: usize,
    pub references: usize,
}

#[derive(Debug)]
struct CommentBlock {
    start_line: usize,
    text: String,
}

pub fn check_repository(root: &Path) -> Result<RationaleCheckSummary> {
    let paths = source_files(root)?;
    let sources = paths
        .iter()
        .map(|path| read_text(path).map(|contents| (path.clone(), contents)))
        .collect::<Result<Vec<_>>>()?;
    let known_requirements = canonical_requirement_ids(root)?;
    let known_adrs = numbered_documents(&root.join("docs/adr"))?;
    let known_rfcs = numbered_documents(&root.join("docs/rfcs"))?;
    let mut issues = Vec::new();
    let mut summary = RationaleCheckSummary {
        source_files: sources.len(),
        ..RationaleCheckSummary::default()
    };

    for (path, contents) in &sources {
        for block in comment_blocks(contents) {
            let requirement_refs = requirement_references(&block.text);
            let adr_refs = numbered_references(&block.text, "ADR");
            let rfc_refs = numbered_references(&block.text, "RFC");
            let regression_refs = regression_references(&block.text);
            let doc_paths = documentation_paths(&block.text);
            let block_reference_count = requirement_refs.len()
                + adr_refs.len()
                + rfc_refs.len()
                + regression_refs.len()
                + doc_paths.len();
            if block_reference_count > 0 {
                summary.rationale_blocks += 1;
                summary.references += block_reference_count;
            }

            for requirement in requirement_refs {
                if !known_requirements.contains(&requirement) {
                    issues.push(format!(
                        "unknown Requirement ID reference `{requirement}` in {} at line {}",
                        display_path(root, path),
                        block.start_line
                    ));
                }
            }
            for number in adr_refs {
                if !known_adrs.contains(&number) {
                    issues.push(format!(
                        "unknown ADR reference `ADR-{number:04}` in {} at line {}",
                        display_path(root, path),
                        block.start_line
                    ));
                }
            }
            for number in rfc_refs {
                if !known_rfcs.contains(&number) {
                    issues.push(format!(
                        "unknown RFC reference `RFC-{number:04}` in {} at line {}",
                        display_path(root, path),
                        block.start_line
                    ));
                }
            }
            for regression in regression_refs {
                if !sources
                    .iter()
                    .any(|(_, source)| contains_code_identifier(source, &regression))
                {
                    issues.push(format!(
                        "unknown regression test reference `{regression}` in {} at line {}",
                        display_path(root, path),
                        block.start_line
                    ));
                }
            }
            for documentation_path in doc_paths {
                if !is_repository_relative_path(&documentation_path) {
                    issues.push(format!(
                        "invalid documentation reference `{documentation_path}` in {} at line {}",
                        display_path(root, path),
                        block.start_line
                    ));
                    continue;
                }
                let target = root.join(&documentation_path);
                if !target.exists() {
                    issues.push(format!(
                        "unknown documentation reference `{documentation_path}` in {} at line {}",
                        display_path(root, path),
                        block.start_line
                    ));
                }
            }
        }
    }

    if issues.is_empty() {
        Ok(summary)
    } else {
        let details = issues
            .into_iter()
            .map(|issue| format!("- {issue}"))
            .collect::<Vec<_>>()
            .join("\n");
        Err(MasterdataError::new(
            "E-XTASK-RATIONALE-CHECK",
            ErrorKind::Validation,
            format!("implementation rationale reference check failed:\n{details}"),
        ))
    }
}

fn source_files(root: &Path) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for relative_root in SOURCE_ROOTS {
        let path = root.join(relative_root);
        if path.exists() {
            collect_source_files(&path, &mut files)?;
        }
    }
    files.sort();
    Ok(files)
}

fn collect_source_files(path: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    let metadata = fs::symlink_metadata(path).map_err(|error| io_error(path, error))?;
    if metadata.file_type().is_symlink() {
        return Ok(());
    }
    if metadata.is_file() {
        if path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| {
                SOURCE_EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str())
            })
        {
            files.push(path.to_path_buf());
        }
        return Ok(());
    }
    if !metadata.is_dir() {
        return Ok(());
    }

    let entries = fs::read_dir(path).map_err(|error| io_error(path, error))?;
    for entry in entries {
        let entry = entry.map_err(|error| io_error(path, error))?;
        collect_source_files(&entry.path(), files)?;
    }
    Ok(())
}

fn read_text(path: &Path) -> Result<String> {
    fs::read_to_string(path).map_err(|error| io_error(path, error))
}

fn io_error(path: &Path, error: impl std::fmt::Display) -> MasterdataError {
    MasterdataError::new(
        "E-XTASK-RATIONALE-IO",
        ErrorKind::Io,
        format!("could not read implementation rationale input: {error}"),
    )
    .with_source(path.to_path_buf())
}

fn comment_blocks(contents: &str) -> Vec<CommentBlock> {
    let mut blocks = Vec::new();
    let mut line_block: Option<CommentBlock> = None;
    let mut lines = contents.lines().enumerate().peekable();

    while let Some((index, line)) = lines.next() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") {
            let text = trimmed.trim_start_matches('/').trim_start().to_owned();
            let line_number = index + 1;
            match line_block.as_mut() {
                Some(block) if line_number == block.start_line + block.text.lines().count() => {
                    block.text.push('\n');
                    block.text.push_str(&text);
                }
                _ => {
                    if let Some(block) = line_block.take() {
                        blocks.push(block);
                    }
                    line_block = Some(CommentBlock {
                        start_line: line_number,
                        text,
                    });
                }
            }
            continue;
        }

        if trimmed.starts_with("/*") {
            if let Some(block) = line_block.take() {
                blocks.push(block);
            }
            let start_line = index + 1;
            let mut text = trimmed.to_owned();
            while !text.contains("*/") {
                let Some((_, next_line)) = lines.next() else {
                    break;
                };
                text.push('\n');
                text.push_str(next_line.trim());
            }
            blocks.push(CommentBlock { start_line, text });
            continue;
        }

        if let Some(block) = line_block.take() {
            blocks.push(block);
        }
    }

    if let Some(block) = line_block {
        blocks.push(block);
    }
    blocks
}

fn requirement_references(text: &str) -> BTreeSet<String> {
    reference_tokens(text)
        .filter(|token| is_requirement_id(token))
        .collect()
}

fn numbered_references(text: &str, prefix: &str) -> BTreeSet<u16> {
    let marker = format!("{prefix}-");
    reference_tokens(text)
        .filter_map(move |token| token.strip_prefix(&marker).map(str::to_owned))
        .filter(|number| number.len() == 4 && number.bytes().all(|byte| byte.is_ascii_digit()))
        .filter_map(|number| number.parse().ok())
        .collect()
}

fn regression_references(text: &str) -> BTreeSet<String> {
    let mut references = BTreeSet::new();
    let mut remaining = text;
    while let Some(index) = remaining.find("Regression:") {
        let after_marker = &remaining[index + "Regression:".len()..];
        let candidate = after_marker.trim_start();
        let identifier = candidate
            .chars()
            .take_while(|character| character.is_ascii_alphanumeric() || *character == '_')
            .collect::<String>();
        if !identifier.is_empty() {
            references.insert(identifier);
        }
        remaining = after_marker;
    }
    references
}

fn documentation_paths(text: &str) -> BTreeSet<String> {
    let mut paths = BTreeSet::new();
    let mut remaining = text;
    while let Some(index) = remaining.find("docs/") {
        let candidate = &remaining[index..];
        let end = candidate
            .find(|character: char| {
                character.is_ascii_whitespace()
                    || matches!(character, ')' | ']' | '`' | ',' | ';' | ':')
            })
            .unwrap_or(candidate.len());
        let path = candidate[..end].trim_end_matches('.');
        if !path.is_empty() {
            paths.insert(path.to_owned());
        }
        remaining = &candidate[end..];
        if remaining == candidate {
            break;
        }
    }
    paths
}

fn reference_tokens(text: &str) -> impl Iterator<Item = String> + '_ {
    text.split(|character: char| {
        !(character.is_ascii_alphanumeric() || matches!(character, '_' | '-'))
    })
    .filter(|token| !token.is_empty())
    .map(str::to_owned)
}

fn numbered_documents(root: &Path) -> Result<BTreeSet<u16>> {
    let mut numbers = BTreeSet::new();
    if !root.exists() {
        return Ok(numbers);
    }
    let entries = fs::read_dir(root).map_err(|error| io_error(root, error))?;
    for entry in entries {
        let entry = entry.map_err(|error| io_error(root, error))?;
        let path = entry.path();
        let file_type = entry.file_type().map_err(|error| io_error(&path, error))?;
        if file_type.is_symlink() || !file_type.is_file() {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        let Some(number) = stem.get(..4) else {
            continue;
        };
        if number.len() == 4 && number.bytes().all(|byte| byte.is_ascii_digit()) {
            if let Ok(number) = number.parse() {
                numbers.insert(number);
            }
        }
    }
    Ok(numbers)
}

fn contains_code_identifier(contents: &str, identifier: &str) -> bool {
    let mut in_block_comment = false;
    for line in contents.lines() {
        let trimmed = line.trim_start();
        if in_block_comment {
            if trimmed.contains("*/") {
                in_block_comment = false;
            }
            continue;
        }
        if trimmed.starts_with("//") {
            continue;
        }
        if trimmed.starts_with("/*") {
            in_block_comment = !trimmed.contains("*/");
            continue;
        }
        if line
            .split(|character: char| !(character.is_ascii_alphanumeric() || character == '_'))
            .any(|token| token == identifier)
        {
            return true;
        }
    }
    false
}

fn is_repository_relative_path(path: &str) -> bool {
    let path = Path::new(path);
    !path.is_absolute()
        && path
            .components()
            .all(|component| !matches!(component, std::path::Component::ParentDir))
}

fn display_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::check_repository;

    fn setup(directory: &std::path::Path, source: &str) {
        fs::create_dir_all(directory.join("docs/specs")).expect("spec directory");
        fs::create_dir_all(directory.join("docs/adr")).expect("ADR directory");
        fs::create_dir_all(directory.join("crates/example/src")).expect("source directory");
        fs::write(
            directory.join("docs/specs/example.md"),
            "# Example\n\nStatus: Draft\n\n### TEST-001\n",
        )
        .expect("spec");
        fs::write(
            directory.join("docs/adr/0001-example.md"),
            "# ADR\n\nStatus: Accepted\n",
        )
        .expect("ADR");
        fs::create_dir_all(directory.join("docs/rfcs")).expect("RFC directory");
        fs::write(
            directory.join("docs/rfcs/0001-example.md"),
            "# RFC\n\nStatus: Proposed\n",
        )
        .expect("RFC");
        fs::write(directory.join("crates/example/src/lib.rs"), source).expect("source");
    }

    #[test]
    fn valid_references_pass() {
        let directory = tempdir().expect("temporary directory");
        setup(
            directory.path(),
            "// WHY: preserve the invariant.\n// Requirement: TEST-001\n// Regression: protects_the_invariant\n// ADR-0001\n// RFC-0001\n// docs/specs/example.md\nfn protects_the_invariant() {}\n",
        );

        let summary = check_repository(directory.path()).expect("valid references");
        assert_eq!(summary.source_files, 1);
        assert_eq!(summary.rationale_blocks, 1);
        assert_eq!(summary.references, 5);
    }

    #[test]
    fn missing_regression_test_is_reported() {
        let directory = tempdir().expect("temporary directory");
        setup(
            directory.path(),
            "// Regression: missing_regression_test\nfn unrelated() {}\n",
        );

        let error = check_repository(directory.path()).expect_err("missing test");
        assert!(
            error
                .to_string()
                .contains("unknown regression test reference")
        );
    }

    #[test]
    fn missing_requirement_and_documentation_are_reported() {
        let directory = tempdir().expect("temporary directory");
        setup(
            directory.path(),
            "// Requirement: TEST-999\n// docs/missing.md\nfn example() {}\n",
        );

        let error = check_repository(directory.path()).expect_err("missing references");
        let message = error.to_string();
        assert!(message.contains("unknown Requirement ID reference `TEST-999`"));
        assert!(message.contains("unknown documentation reference `docs/missing.md`"));
    }

    #[test]
    fn comments_without_structural_references_are_not_parsed_as_rationale() {
        let directory = tempdir().expect("temporary directory");
        setup(
            directory.path(),
            "// This comment explains a normal implementation detail.\nfn example() {}\n",
        );

        let summary = check_repository(directory.path()).expect("unannotated comment");
        assert_eq!(summary.rationale_blocks, 0);
        assert_eq!(summary.references, 0);
    }
}
