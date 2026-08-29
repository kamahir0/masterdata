use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use masterdata_core::{ErrorKind, MasterdataError, Result};

const SPEC_STATUSES: &[&str] = &["Draft", "Proposed", "Approved", "Implemented", "Deprecated"];

#[derive(Debug, Default)]
pub struct SpecCheckSummary {
    pub spec_files: usize,
    pub gui_spec_files: usize,
    pub requirement_ids: usize,
    pub adr_numbers: usize,
    pub relative_links: usize,
}

pub fn check_repository(root: &Path) -> Result<SpecCheckSummary> {
    let spec_files = markdown_files(&root.join("docs/specs"))?;
    let gui_files = markdown_files(&root.join("docs/gui"))?;
    let adr_files = markdown_files(&root.join("docs/adr"))?;
    let mut issues = Vec::new();
    let mut summary = SpecCheckSummary {
        spec_files: spec_files
            .iter()
            .filter(|path| is_canonical_document(path))
            .count(),
        gui_spec_files: gui_files
            .iter()
            .filter(|path| is_canonical_document(path))
            .count(),
        ..SpecCheckSummary::default()
    };

    for path in spec_files
        .iter()
        .chain(gui_files.iter())
        .filter(|path| is_canonical_document(path))
    {
        let contents = read_text(path)?;
        check_spec_header(root, path, &contents, &mut issues);
    }

    let mut requirement_owners = BTreeMap::new();
    for path in spec_files
        .iter()
        .chain(gui_files.iter())
        .filter(|path| is_canonical_document(path))
    {
        let contents = read_text(path)?;
        for id in requirement_ids(&contents) {
            summary.requirement_ids += 1;
            if let Some(previous) = requirement_owners.insert(id.clone(), path.clone()) {
                issues.push(format!(
                    "duplicate specification ID `{id}` in {} and {}",
                    display_path(root, &previous),
                    display_path(root, path)
                ));
            }
        }
    }

    let mut adr_owners = BTreeMap::new();
    for path in adr_files
        .iter()
        .filter(|path| path.file_name().and_then(|name| name.to_str()) != Some("_template.md"))
    {
        if let Some(number) = adr_number(path) {
            summary.adr_numbers += 1;
            if let Some(previous) = adr_owners.insert(number, path.clone()) {
                issues.push(format!(
                    "duplicate ADR number `{number:04}` in {} and {}",
                    display_path(root, &previous),
                    display_path(root, path)
                ));
            }
        }
    }

    let mut link_files = markdown_files(&root.join("docs"))?;
    for path in [root.join("README.md"), root.join("AGENTS.md")] {
        if path.is_file() {
            link_files.push(path);
        }
    }
    link_files.sort();
    link_files.dedup();
    for path in link_files {
        let contents = read_text(&path)?;
        summary.relative_links += check_relative_links(root, &path, &contents, &mut issues);
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
            "XTASK-SPECS-001",
            ErrorKind::Validation,
            format!("spec integrity check failed:\n{details}"),
        ))
    }
}

fn markdown_files(root: &Path) -> Result<Vec<PathBuf>> {
    if !root.exists() {
        return Ok(Vec::new());
    }
    let metadata = fs::metadata(root).map_err(|error| io_error(root, error))?;
    if metadata.is_file() {
        return Ok(
            if root.extension().and_then(|value| value.to_str()) == Some("md") {
                vec![root.to_path_buf()]
            } else {
                Vec::new()
            },
        );
    }

    let entries = fs::read_dir(root).map_err(|error| io_error(root, error))?;
    let mut files = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| io_error(root, error))?;
        let path = entry.path();
        if entry
            .file_type()
            .map_err(|error| io_error(&path, error))?
            .is_dir()
        {
            files.extend(markdown_files(&path)?);
        } else if path.extension().and_then(|value| value.to_str()) == Some("md") {
            files.push(path);
        }
    }
    files.sort();
    Ok(files)
}

fn read_text(path: &Path) -> Result<String> {
    fs::read_to_string(path).map_err(|error| io_error(path, error))
}

fn io_error(path: &Path, error: impl std::fmt::Display) -> MasterdataError {
    MasterdataError::new(
        "XTASK-SPECS-002",
        ErrorKind::Io,
        format!("could not read specification workflow input: {error}"),
    )
    .with_source(path.to_path_buf())
}

fn is_canonical_document(path: &Path) -> bool {
    !matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some("README.md") | Some("_template.md")
    )
}

fn check_spec_header(root: &Path, path: &Path, contents: &str, issues: &mut Vec<String>) {
    let mut first_content_line = None;
    let mut status = None;
    for line in contents.lines().take(12) {
        let trimmed = line.trim();
        if first_content_line.is_none() && !trimmed.is_empty() {
            first_content_line = Some(trimmed.to_string());
        }
        if let Some(value) = trimmed.strip_prefix("Status:") {
            status = Some(value.trim());
        }
    }

    if !first_content_line
        .as_deref()
        .is_some_and(|line| line.starts_with("# "))
    {
        issues.push(format!(
            "malformed specification header in {}: first content line must be a level-one title",
            display_path(root, path)
        ));
    }
    match status {
        Some(value) if SPEC_STATUSES.contains(&value) => {}
        Some(value) => issues.push(format!(
            "invalid specification status `{value}` in {}: expected one of Draft, Proposed, Approved, Implemented, Deprecated",
            display_path(root, path)
        )),
        None => issues.push(format!(
            "malformed specification header in {}: missing `Status:` within the header",
            display_path(root, path)
        )),
    }
}

fn requirement_ids(contents: &str) -> BTreeSet<String> {
    contents
        .lines()
        .flat_map(|line| {
            line.split(|character: char| {
                character.is_whitespace()
                    || matches!(
                        character,
                        '`' | ':'
                            | ','
                            | '.'
                            | ';'
                            | '('
                            | ')'
                            | '['
                            | ']'
                            | '{'
                            | '}'
                            | '"'
                            | '\''
                    )
            })
        })
        .filter(|token| is_requirement_id(token))
        .map(str::to_owned)
        .collect()
}

fn is_requirement_id(token: &str) -> bool {
    let parts = token.split('-').collect::<Vec<_>>();
    if parts.len() < 2 {
        return false;
    }
    let Some(number) = parts.last() else {
        return false;
    };
    if number.len() != 3 || !number.bytes().all(|byte| byte.is_ascii_digit()) {
        return false;
    }
    let segments = &parts[..parts.len() - 1];
    !segments.is_empty()
        && segments.iter().all(|segment| {
            !segment.is_empty()
                && segment
                    .bytes()
                    .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit())
        })
        && segments
            .iter()
            .any(|segment| segment.bytes().any(|byte| byte.is_ascii_uppercase()))
}

fn adr_number(path: &Path) -> Option<u16> {
    let stem = path.file_stem()?.to_str()?;
    let number = stem.get(..4)?;
    if number.len() == 4 && number.bytes().all(|byte| byte.is_ascii_digit()) {
        number.parse().ok()
    } else {
        None
    }
}

fn check_relative_links(
    root: &Path,
    path: &Path,
    contents: &str,
    issues: &mut Vec<String>,
) -> usize {
    let mut remaining = contents;
    let mut checked = 0;
    while let Some(start) = remaining.find("](") {
        let after_open = &remaining[start + 2..];
        let Some(end) = after_open.find(')') else {
            break;
        };
        let raw_destination = after_open[..end].trim();
        let destination = raw_destination
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .trim_matches('<')
            .trim_matches('>');
        let path_part = destination.split('#').next().unwrap_or_default();
        if !path_part.is_empty()
            && !destination.starts_with('#')
            && !destination.starts_with("http://")
            && !destination.starts_with("https://")
            && !destination.starts_with("mailto:")
            && !destination.starts_with("//")
        {
            checked += 1;
            let target = path.parent().unwrap_or(root).join(path_part);
            if !target.exists() {
                issues.push(format!(
                    "broken relative link in {}: `{destination}`",
                    display_path(root, path)
                ));
            }
        }
        remaining = &after_open[end + 1..];
    }
    checked
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

    #[test]
    fn valid_repository_passes() {
        let directory = tempdir().expect("temporary directory");
        let specs = directory.path().join("docs/specs");
        fs::create_dir_all(&specs).expect("spec directory");
        fs::write(
            specs.join("example.md"),
            "# Example\n\nStatus: Proposed\n\n### TEST-001\n\nThe rule MAY be used.\n",
        )
        .expect("spec file");

        let summary = check_repository(directory.path()).expect("valid repository");
        assert_eq!(summary.spec_files, 1);
        assert_eq!(summary.requirement_ids, 1);
    }

    #[test]
    fn duplicate_requirement_ids_are_reported() {
        let directory = tempdir().expect("temporary directory");
        let specs = directory.path().join("docs/specs");
        fs::create_dir_all(&specs).expect("spec directory");
        for name in ["first.md", "second.md"] {
            fs::write(
                specs.join(name),
                "# Example\n\nStatus: Draft\n\n### TEST-001\n",
            )
            .expect("spec file");
        }

        let error = check_repository(directory.path()).expect_err("duplicate ID");
        assert!(
            error
                .to_string()
                .contains("duplicate specification ID `TEST-001`")
        );
    }

    #[test]
    fn broken_relative_links_are_reported() {
        let directory = tempdir().expect("temporary directory");
        let specs = directory.path().join("docs/specs");
        fs::create_dir_all(&specs).expect("spec directory");
        fs::write(
            specs.join("example.md"),
            "# Example\n\nStatus: Draft\n\n[missing](missing.md)\n",
        )
        .expect("spec file");

        let error = check_repository(directory.path()).expect_err("broken link");
        assert!(error.to_string().contains("broken relative link"));
    }

    #[test]
    fn invalid_status_and_duplicate_adr_numbers_are_reported() {
        let directory = tempdir().expect("temporary directory");
        let specs = directory.path().join("docs/specs");
        let adrs = directory.path().join("docs/adr");
        fs::create_dir_all(&specs).expect("spec directory");
        fs::create_dir_all(&adrs).expect("ADR directory");
        fs::write(specs.join("example.md"), "# Example\n\nStatus: Accepted\n").expect("spec file");
        for name in ["0001-first.md", "0001-second.md"] {
            fs::write(adrs.join(name), "# ADR\n\nStatus: Accepted\n").expect("ADR file");
        }

        let error = check_repository(directory.path()).expect_err("invalid metadata");
        let message = error.to_string();
        assert!(message.contains("invalid specification status `Accepted`"));
        assert!(message.contains("duplicate ADR number `0001`"));
    }
}
