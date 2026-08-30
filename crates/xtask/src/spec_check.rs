use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use masterdata_core::{ErrorKind, MasterdataError, Result};

const SPEC_STATUSES: &[&str] = &["Draft", "Proposed", "Approved", "Implemented", "Deprecated"];
const RFC_STATUSES: &[&str] = &["Draft", "Proposed", "Accepted", "Rejected", "Superseded"];
const SPEC_CHANGE_STATUSES: &[&str] = &["Draft", "Proposed", "Approved", "Applied", "Rejected"];

#[derive(Debug, Default)]
pub struct SpecCheckSummary {
    pub spec_files: usize,
    pub gui_spec_files: usize,
    pub requirement_ids: usize,
    pub adr_numbers: usize,
    pub rfc_numbers: usize,
    pub proposal_numbers: usize,
    pub relative_links: usize,
}

pub fn check_repository(root: &Path) -> Result<SpecCheckSummary> {
    let spec_files = markdown_files(&root.join("docs/specs"))?;
    let gui_files = markdown_files(&root.join("docs/gui"))?;
    let adr_files = markdown_files(&root.join("docs/adr"))?;
    let rfc_files = markdown_files(&root.join("docs/rfcs"))?;
    let proposal_files = markdown_files(&root.join("docs/spec-changes"))?;
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

    let mut requirement_owners: BTreeMap<String, PathBuf> = BTreeMap::new();
    for path in spec_files
        .iter()
        .chain(gui_files.iter())
        .filter(|path| is_canonical_document(path))
    {
        let contents = read_text(path)?;
        for (heading, line) in malformed_requirement_headings(&contents) {
            issues.push(format!(
                "malformed specification ID definition `{heading}` in {} at line {line}",
                display_path(root, path)
            ));
        }
        let definitions = requirement_definitions(&contents);
        let mut local_definitions = BTreeMap::new();
        for definition in definitions {
            summary.requirement_ids += 1;
            if let Some(previous_line) =
                local_definitions.insert(definition.id.clone(), definition.line)
            {
                issues.push(format!(
                    "duplicate specification ID definition `{}` within {} at lines {} and {}",
                    definition.id,
                    display_path(root, path),
                    previous_line,
                    definition.line
                ));
            }
            if let Some(previous) = requirement_owners.get(&definition.id) {
                if previous != path {
                    issues.push(format!(
                        "duplicate specification ID definition `{}` in {} and {}",
                        definition.id,
                        display_path(root, previous),
                        display_path(root, path)
                    ));
                }
            } else {
                requirement_owners.insert(definition.id.clone(), (*path).clone());
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

    let mut rfc_owners = BTreeMap::new();
    for path in rfc_files.iter().filter(|path| is_numbered_document(path)) {
        let contents = read_text(path)?;
        check_document_header(root, path, &contents, RFC_STATUSES, "RFC", &mut issues);
        check_numbered_filename(root, path, "RFC", &mut issues);
        if let Some(number) = numbered_document_number(path) {
            summary.rfc_numbers += 1;
            if let Some(previous) = rfc_owners.insert(number, path.clone()) {
                issues.push(format!(
                    "duplicate RFC number `{number:04}` in {} and {}",
                    display_path(root, &previous),
                    display_path(root, path)
                ));
            }
        }
    }

    let mut proposal_owners = BTreeMap::new();
    for path in proposal_files
        .iter()
        .filter(|path| is_numbered_document(path))
    {
        let contents = read_text(path)?;
        check_document_header(
            root,
            path,
            &contents,
            SPEC_CHANGE_STATUSES,
            "specification change",
            &mut issues,
        );
        check_numbered_filename(root, path, "specification change", &mut issues);
        if !has_section(&contents, "Affected Specifications") {
            issues.push(format!(
                "malformed specification change in {}: missing `## Affected Specifications` section",
                display_path(root, path)
            ));
        }
        if let Some(number) = numbered_document_number(path) {
            summary.proposal_numbers += 1;
            if let Some(previous) = proposal_owners.insert(number, path.clone()) {
                issues.push(format!(
                    "duplicate specification change number `{number:04}` in {} and {}",
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
            "E-XTASK-SPECS-CHECK",
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
        let file_type = entry.file_type().map_err(|error| io_error(&path, error))?;
        if file_type.is_symlink() {
            continue;
        }
        if file_type.is_dir() {
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
        "E-XTASK-SPECS-IO",
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

pub(crate) fn canonical_requirement_ids(root: &Path) -> Result<std::collections::BTreeSet<String>> {
    let spec_files = markdown_files(&root.join("docs/specs"))?;
    let gui_files = markdown_files(&root.join("docs/gui"))?;
    let mut ids = std::collections::BTreeSet::new();
    for path in spec_files
        .iter()
        .chain(gui_files.iter())
        .filter(|path| is_canonical_document(path))
    {
        let contents = read_text(path)?;
        ids.extend(
            requirement_definitions(&contents)
                .into_iter()
                .map(|definition| definition.id),
        );
    }
    Ok(ids)
}

fn check_spec_header(root: &Path, path: &Path, contents: &str, issues: &mut Vec<String>) {
    check_document_header(root, path, contents, SPEC_STATUSES, "specification", issues);
}

fn check_document_header(
    root: &Path,
    path: &Path,
    contents: &str,
    statuses: &[&str],
    document_kind: &str,
    issues: &mut Vec<String>,
) {
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
            "malformed {document_kind} header in {}: first content line must be a level-one title",
            display_path(root, path),
        ));
    }
    match status {
        Some(value) if statuses.contains(&value) => {}
        Some(value) => issues.push(format!(
            "invalid {document_kind} status `{value}` in {}",
            display_path(root, path),
        )),
        None => issues.push(format!(
            "malformed {document_kind} header in {}: missing `Status:` within the header",
            display_path(root, path),
        )),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RequirementDefinition {
    id: String,
    line: usize,
}

/// Requirement IDs are owned only by explicit level-three headings. Mentions
/// in prose such as `See PROJECT-001` are references and must not become a
/// second owner. Keeping the grammar intentionally small also makes the
/// checker useful without attempting to parse Markdown in full.
fn requirement_definitions(contents: &str) -> Vec<RequirementDefinition> {
    contents
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let heading = line.strip_prefix("### ")?.trim();
            is_requirement_id(heading).then(|| RequirementDefinition {
                id: heading.to_owned(),
                line: index + 1,
            })
        })
        .collect()
}

fn malformed_requirement_headings(contents: &str) -> Vec<(String, usize)> {
    contents
        .lines()
        .enumerate()
        .filter_map(|(index, line)| {
            let heading = line.strip_prefix("### ")?.trim();
            if !heading.is_empty()
                && looks_like_requirement_heading(heading)
                && !is_requirement_id(heading)
            {
                Some((heading.to_owned(), index + 1))
            } else {
                None
            }
        })
        .collect()
}

fn looks_like_requirement_heading(heading: &str) -> bool {
    // A normal Markdown title may contain hyphens and mixed case. Only flag a
    // compact, identifier-shaped heading whose author appears to have tried
    // to write a requirement ID but missed the repository grammar.
    if heading.is_empty() || heading.bytes().any(|byte| byte.is_ascii_whitespace()) {
        return false;
    }

    if heading.contains('_') {
        let parts = heading.split('_').collect::<Vec<_>>();
        if parts.len() >= 2
            && parts.iter().all(|part| is_ascii_identifier_segment(part))
            && parts[0].bytes().any(|byte| byte.is_ascii_uppercase())
        {
            return true;
        }
    }

    if heading.contains('-') {
        let parts = heading.split('-').collect::<Vec<_>>();
        let Some(number) = parts.last() else {
            return false;
        };
        if parts.len() >= 2
            && parts[..parts.len() - 1]
                .iter()
                .all(|part| is_ascii_identifier_segment(part))
            && !parts[..parts.len() - 1].is_empty()
            && number.len() <= 3
            && !number.is_empty()
            && number.bytes().all(|byte| byte.is_ascii_digit())
        {
            return true;
        }
    }

    let mut uppercase_prefix = 0;
    for byte in heading.bytes() {
        if byte.is_ascii_uppercase() {
            uppercase_prefix += 1;
        } else {
            break;
        }
    }
    uppercase_prefix >= 3
        && heading.len() > uppercase_prefix
        && heading[uppercase_prefix..]
            .bytes()
            .all(|byte| byte.is_ascii_digit())
}

fn is_ascii_identifier_segment(segment: &str) -> bool {
    !segment.is_empty() && segment.bytes().all(|byte| byte.is_ascii_alphanumeric())
}

pub(crate) fn is_requirement_id(token: &str) -> bool {
    if token.starts_with("E-") {
        return false;
    }
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

fn is_numbered_document(path: &Path) -> bool {
    !matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some("README.md") | Some("_template.md")
    )
}

fn numbered_document_number(path: &Path) -> Option<u16> {
    let stem = path.file_stem()?.to_str()?;
    let number = stem.get(..4)?;
    if number.len() == 4 && number.bytes().all(|byte| byte.is_ascii_digit()) {
        number.parse().ok()
    } else {
        None
    }
}

fn check_numbered_filename(
    root: &Path,
    path: &Path,
    document_kind: &str,
    issues: &mut Vec<String>,
) {
    let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
        issues.push(format!(
            "invalid {document_kind} filename in {}",
            display_path(root, path)
        ));
        return;
    };
    let stem = name.strip_suffix(".md");
    let valid = stem.is_some_and(|stem| {
        let Some((number, slug)) = stem.split_once('-') else {
            return false;
        };
        number.len() == 4
            && number.bytes().all(|byte| byte.is_ascii_digit())
            && !slug.is_empty()
            && !slug.starts_with('-')
            && !slug.ends_with('-')
            && slug
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
    });
    if !valid {
        issues.push(format!(
            "invalid {document_kind} filename `{name}` in {}: expected NNNN-lowercase-slug.md",
            display_path(root, path)
        ));
    }
}

fn has_section(contents: &str, title: &str) -> bool {
    let expected = format!("## {title}");
    contents.lines().any(|line| line.trim() == expected)
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
            "# Example\n\nStatus: Proposed\n\n### TEST-001\n\nThe rule MAY be used. See TEST-001 for the reference.\n",
        )
        .expect("spec file");

        let summary = check_repository(directory.path()).expect("valid repository");
        assert_eq!(summary.spec_files, 1);
        assert_eq!(summary.requirement_ids, 1);
    }

    #[test]
    fn natural_hyphenated_heading_is_allowed() {
        let directory = tempdir().expect("temporary directory");
        let specs = directory.path().join("docs/specs");
        fs::create_dir_all(&specs).expect("spec directory");
        fs::write(
            specs.join("example.md"),
            "# Example\n\nStatus: Draft\n\n### Round-trip editing\n\n### Cross-platform behavior\n",
        )
        .expect("spec file");

        check_repository(directory.path()).expect("natural headings are allowed");
    }

    #[test]
    fn natural_mixed_case_title_is_allowed() {
        let directory = tempdir().expect("temporary directory");
        let specs = directory.path().join("docs/specs");
        fs::create_dir_all(&specs).expect("spec directory");
        fs::write(
            specs.join("example.md"),
            "# Example\n\nStatus: Draft\n\n### Value-object compatibility\n\n### yaml Parser Notes\n",
        )
        .expect("spec file");

        check_repository(directory.path()).expect("natural title is allowed");
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
                .contains("duplicate specification ID definition `TEST-001`")
        );
    }

    #[test]
    fn duplicate_requirement_id_in_one_file_is_reported() {
        let directory = tempdir().expect("temporary directory");
        let specs = directory.path().join("docs/specs");
        fs::create_dir_all(&specs).expect("spec directory");
        fs::write(
            specs.join("example.md"),
            "# Example\n\nStatus: Draft\n\n### TEST-001\n\n### TEST-001\n",
        )
        .expect("spec file");

        let error = check_repository(directory.path()).expect_err("duplicate ID");
        let message = error.to_string();
        assert!(message.contains("within"));
        assert!(message.contains("example.md"));
    }

    #[test]
    fn malformed_requirement_heading_is_reported() {
        let directory = tempdir().expect("temporary directory");
        let specs = directory.path().join("docs/specs");
        fs::create_dir_all(&specs).expect("spec directory");
        fs::write(
            specs.join("example.md"),
            "# Example\n\nStatus: Draft\n\n### TEST_bad\n",
        )
        .expect("spec file");

        let error = check_repository(directory.path()).expect_err("malformed ID");
        assert!(error.to_string().contains("malformed specification ID"));
    }

    #[test]
    fn malformed_compact_requirement_heading_is_reported() {
        let directory = tempdir().expect("temporary directory");
        let specs = directory.path().join("docs/specs");
        fs::create_dir_all(&specs).expect("spec directory");
        fs::write(
            specs.join("example.md"),
            "# Example\n\nStatus: Draft\n\n### TEST1\n",
        )
        .expect("spec file");

        let error = check_repository(directory.path()).expect_err("malformed ID");
        assert!(error.to_string().contains("malformed specification ID"));
    }

    #[test]
    fn malformed_requirement_like_headings_are_reported() {
        let directory = tempdir().expect("temporary directory");
        let specs = directory.path().join("docs/specs");
        fs::create_dir_all(&specs).expect("spec directory");
        fs::write(
            specs.join("example.md"),
            "# Example\n\nStatus: Draft\n\n### TEST_001\n\n### TEST-01\n\n### SCHEMA-vo-001\n\n### PROJECT001\n",
        )
        .expect("spec file");

        let error = check_repository(directory.path()).expect_err("malformed IDs");
        let message = error.to_string();
        for heading in ["TEST_001", "TEST-01", "SCHEMA-vo-001", "PROJECT001"] {
            assert!(
                message.contains(heading),
                "missing malformed heading {heading}"
            );
        }
    }

    #[test]
    fn diagnostic_namespace_is_not_a_requirement_definition() {
        let directory = tempdir().expect("temporary directory");
        let specs = directory.path().join("docs/specs");
        fs::create_dir_all(&specs).expect("spec directory");
        fs::write(
            specs.join("example.md"),
            "# Example\n\nStatus: Draft\n\n### E-SCHEMA-001\n",
        )
        .expect("spec file");

        let error = check_repository(directory.path()).expect_err("diagnostic namespace");
        assert!(error.to_string().contains("malformed specification ID"));
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

    #[test]
    fn specification_change_requires_number_header_and_affected_section() {
        let directory = tempdir().expect("temporary directory");
        let changes = directory.path().join("docs/spec-changes");
        fs::create_dir_all(&changes).expect("change directory");
        fs::write(
            changes.join("0001-example.md"),
            "# Specification change: Example\n\nStatus: Proposed\n\n## Affected Specifications\n\n- `docs/specs/example.md`\n",
        )
        .expect("change file");

        let summary = check_repository(directory.path()).expect("valid change proposal");
        assert_eq!(summary.proposal_numbers, 1);
    }

    #[test]
    fn duplicate_specification_change_numbers_are_reported() {
        let directory = tempdir().expect("temporary directory");
        let changes = directory.path().join("docs/spec-changes");
        fs::create_dir_all(&changes).expect("change directory");
        for name in ["0001-first.md", "0001-second.md"] {
            fs::write(
                changes.join(name),
                "# Specification change\n\nStatus: Draft\n\n## Affected Specifications\n",
            )
            .expect("change file");
        }

        let error = check_repository(directory.path()).expect_err("duplicate proposal number");
        assert!(
            error
                .to_string()
                .contains("duplicate specification change number `0001`")
        );
    }

    #[test]
    fn rfc_filename_and_status_are_checked() {
        let directory = tempdir().expect("temporary directory");
        let rfcs = directory.path().join("docs/rfcs");
        fs::create_dir_all(&rfcs).expect("RFC directory");
        fs::write(rfcs.join("bad-name.md"), "# RFC\n\nStatus: Approved\n").expect("RFC file");

        let error = check_repository(directory.path()).expect_err("invalid RFC metadata");
        let message = error.to_string();
        assert!(message.contains("invalid RFC filename"));
        assert!(message.contains("invalid RFC status `Approved`"));
    }

    #[test]
    fn duplicate_rfc_numbers_are_reported() {
        let directory = tempdir().expect("temporary directory");
        let rfcs = directory.path().join("docs/rfcs");
        fs::create_dir_all(&rfcs).expect("RFC directory");
        for name in ["0001-first.md", "0001-second.md"] {
            fs::write(rfcs.join(name), "# RFC\n\nStatus: Proposed\n").expect("RFC file");
        }

        let error = check_repository(directory.path()).expect_err("duplicate RFC number");
        assert!(error.to_string().contains("duplicate RFC number `0001`"));
    }
}
