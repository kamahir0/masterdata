use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

use masterdata_core::{ErrorKind, MasterdataError, Result};
use same_file::is_same_file;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const ARTIFACT_SET_RECEIPT_FILENAME: &str = ".masterdata-artifact-set.json";
pub const ARTIFACT_RECEIPT_FILENAME: &str = ARTIFACT_SET_RECEIPT_FILENAME;
pub const ARTIFACT_SET_RECEIPT_VERSION: u32 = 1;
pub const ARTIFACT_HASH_ALGORITHM: &str = "sha256";
pub const CANONICAL_BINARY_PATH: &str = "masterdata.bytes";

const CSHARP_DIRECTORY: &str = "csharp";

/// The v1 semantic receipt contract for a complete canonical artifact set.
///
/// Unknown JSON properties are intentionally ignored by serde. The approved
/// contract fixes the required semantic fields, but does not make unrelated
/// future metadata a reason for a v1 reader to reject an otherwise valid set.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactSetReceipt {
    pub version: u32,
    pub project_id: String,
    pub hash_algorithm: String,
    pub csharp: Vec<ArtifactReceiptEntry>,
    pub binary: ArtifactReceiptEntry,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactReceiptEntry {
    pub path: String,
    pub hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedArtifactSet {
    pub artifact_root: PathBuf,
    pub receipt_path: PathBuf,
    pub receipt: ArtifactSetReceipt,
    pub csharp: Vec<ValidatedArtifact>,
    pub binary: ValidatedArtifact,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidatedArtifact {
    pub path: PathBuf,
    pub relative_path: String,
    pub hash: String,
}

#[derive(Debug, Clone)]
struct DiscoveredCSharpFile {
    path: PathBuf,
    relative_path: String,
}

/// Create the v1 receipt from the exact bytes already assembled under a
/// staged canonical root. This function does not write the receipt.
pub fn create_artifact_set_receipt(
    artifact_root: &Path,
    project_id: &str,
) -> Result<ArtifactSetReceipt> {
    validate_staged_root_layout(artifact_root)?;
    let csharp_root = artifact_root.join(CSHARP_DIRECTORY);
    let csharp_files = discover_csharp_files(&csharp_root)?;
    let csharp = csharp_files
        .iter()
        .map(|file| {
            Ok(ArtifactReceiptEntry {
                path: file.relative_path.clone(),
                hash: hash_regular_file(&file.path)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let binary_path = artifact_root.join(CANONICAL_BINARY_PATH);
    let binary = ArtifactReceiptEntry {
        path: CANONICAL_BINARY_PATH.to_owned(),
        hash: hash_regular_file(&binary_path)?,
    };

    Ok(ArtifactSetReceipt {
        version: ARTIFACT_SET_RECEIPT_VERSION,
        project_id: project_id.to_owned(),
        hash_algorithm: ARTIFACT_HASH_ALGORITHM.to_owned(),
        csharp,
        binary,
    })
}

/// Write a receipt beside a staged complete artifact set.
pub fn write_artifact_set_receipt(
    artifact_root: &Path,
    project_id: &str,
) -> Result<ArtifactSetReceipt> {
    let receipt = create_artifact_set_receipt(artifact_root, project_id)?;
    let receipt_path = artifact_root.join(ARTIFACT_SET_RECEIPT_FILENAME);
    let content = serde_json::to_vec_pretty(&receipt).map_err(|error| {
        receipt_generation_error(
            &receipt_path,
            format!("could not serialize artifact-set receipt: {error}"),
        )
    })?;
    fs::write(&receipt_path, content).map_err(|error| {
        receipt_generation_error(
            &receipt_path,
            format!("could not write artifact-set receipt: {error}"),
        )
    })?;
    Ok(receipt)
}

/// Validate an existing canonical artifact set without loading source YAML or
/// invoking any build tool.
pub fn validate_artifact_set(
    artifact_root: &Path,
    project_id: &str,
) -> Result<ValidatedArtifactSet> {
    validate_root_layout(artifact_root)?;

    let receipt_path = artifact_root.join(ARTIFACT_SET_RECEIPT_FILENAME);
    let receipt = read_receipt(&receipt_path)?;
    validate_receipt_shape(&receipt, project_id, &receipt_path)?;

    let csharp_root = artifact_root.join(CSHARP_DIRECTORY);
    let actual_csharp = discover_csharp_files(&csharp_root)?;
    let validated_csharp = validate_csharp_entries(&receipt, &csharp_root, &actual_csharp)?;

    let binary_path = artifact_root.join(CANONICAL_BINARY_PATH);
    validate_binary_entry(&receipt.binary, &binary_path)?;
    let binary = ValidatedArtifact {
        path: binary_path,
        relative_path: CANONICAL_BINARY_PATH.to_owned(),
        hash: receipt.binary.hash.clone(),
    };

    Ok(ValidatedArtifactSet {
        artifact_root: artifact_root.to_path_buf(),
        receipt_path,
        receipt,
        csharp: validated_csharp,
        binary,
    })
}

fn read_receipt(path: &Path) -> Result<ArtifactSetReceipt> {
    let content = fs::read(path).map_err(|error| {
        artifact_error(
            "E-ARTIFACT-SET-RECEIPT-READ",
            ErrorKind::Io,
            path,
            format!("could not read artifact-set receipt: {error}"),
            true,
        )
    })?;
    serde_json::from_slice(&content).map_err(|error| {
        artifact_error(
            "E-ARTIFACT-SET-RECEIPT-PARSE",
            ErrorKind::Parse,
            path,
            format!("artifact-set receipt is not valid JSON or v1 shape: {error}"),
            false,
        )
    })
}

fn validate_receipt_shape(
    receipt: &ArtifactSetReceipt,
    project_id: &str,
    receipt_path: &Path,
) -> Result<()> {
    if receipt.version != ARTIFACT_SET_RECEIPT_VERSION {
        return Err(artifact_error(
            "E-ARTIFACT-SET-RECEIPT-VERSION",
            ErrorKind::Validation,
            receipt_path,
            format!(
                "unsupported artifact-set receipt version {}; supported version is {}",
                receipt.version, ARTIFACT_SET_RECEIPT_VERSION
            ),
            false,
        ));
    }
    if receipt.hash_algorithm != ARTIFACT_HASH_ALGORITHM {
        return Err(artifact_error(
            "E-ARTIFACT-SET-RECEIPT-HASH-ALGORITHM",
            ErrorKind::Validation,
            receipt_path,
            format!(
                "unsupported artifact hash algorithm {}; supported algorithm is {}",
                receipt.hash_algorithm, ARTIFACT_HASH_ALGORITHM
            ),
            false,
        ));
    }
    if receipt.project_id != project_id {
        return Err(artifact_error(
            "E-ARTIFACT-SET-PROJECT-MISMATCH",
            ErrorKind::Validation,
            receipt_path,
            format!(
                "artifact-set receipt project_id {} does not match current project.id {}",
                receipt.project_id, project_id
            ),
            false,
        ));
    }
    if receipt.binary.path != CANONICAL_BINARY_PATH {
        return Err(artifact_error(
            "E-ARTIFACT-SET-BINARY-PATH",
            ErrorKind::Validation,
            receipt_path,
            format!(
                "artifact-set receipt binary path must be exactly {}",
                CANONICAL_BINARY_PATH
            ),
            false,
        ));
    }
    if !is_sha256(&receipt.binary.hash) {
        return Err(artifact_error(
            "E-ARTIFACT-SET-BINARY-HASH",
            ErrorKind::Validation,
            receipt_path,
            "artifact-set receipt binary hash must be lowercase 64-hex SHA-256",
            false,
        ));
    }

    let mut previous_path: Option<&str> = None;
    let mut seen = BTreeSet::new();
    for entry in &receipt.csharp {
        validate_receipt_relative_path(&entry.path, receipt_path)?;
        if !seen.insert(entry.path.as_str()) {
            return Err(artifact_error(
                "E-ARTIFACT-SET-CSHARP-PATH-DUPLICATE",
                ErrorKind::Validation,
                receipt_path,
                format!("duplicate C# receipt path {}", entry.path),
                false,
            ));
        }
        if let Some(previous_path) = previous_path
            && previous_path >= entry.path.as_str()
        {
            return Err(artifact_error(
                "E-ARTIFACT-SET-CSHARP-PATH-ORDER",
                ErrorKind::Validation,
                receipt_path,
                "C# receipt paths must be in deterministic ascending order",
                false,
            ));
        }
        if !is_sha256(&entry.hash) {
            return Err(artifact_error(
                "E-ARTIFACT-SET-CSHARP-HASH",
                ErrorKind::Validation,
                receipt_path,
                format!(
                    "C# receipt hash for {} must be lowercase 64-hex SHA-256",
                    entry.path
                ),
                false,
            ));
        }
        previous_path = Some(&entry.path);
    }
    Ok(())
}

fn validate_csharp_entries(
    receipt: &ArtifactSetReceipt,
    csharp_root: &Path,
    actual_files: &[DiscoveredCSharpFile],
) -> Result<Vec<ValidatedArtifact>> {
    if receipt.csharp.len() != actual_files.len() {
        return Err(artifact_error(
            "E-ARTIFACT-SET-CSHARP-FILE-SET",
            ErrorKind::Validation,
            csharp_root,
            format!(
                "artifact-set receipt lists {} C# file(s), but canonical csharp/ contains {} regular file(s)",
                receipt.csharp.len(),
                actual_files.len()
            ),
            false,
        ));
    }

    let mut receipt_paths = Vec::with_capacity(receipt.csharp.len());
    for entry in &receipt.csharp {
        let path = safe_receipt_path(csharp_root, &entry.path)?;
        ensure_regular_file(&path, "C# artifact")?;
        receipt_paths.push(path);
    }
    reject_filesystem_aliases(
        &receipt_paths,
        csharp_root,
        "E-ARTIFACT-SET-CSHARP-PATH-ALIAS",
        "C# receipt paths alias the same filesystem file",
    )?;

    let mut matched = vec![false; actual_files.len()];
    let mut validated = Vec::with_capacity(receipt.csharp.len());
    for (entry, receipt_path) in receipt.csharp.iter().zip(receipt_paths) {
        let mut actual_index = None;
        for (index, actual) in actual_files.iter().enumerate() {
            if matched[index] {
                continue;
            }
            if is_same_file(&actual.path, &receipt_path).map_err(|error| {
                artifact_error(
                    "E-ARTIFACT-SET-FILESYSTEM-IDENTITY",
                    ErrorKind::Io,
                    &receipt_path,
                    format!("could not compare receipt and canonical C# files: {error}"),
                    false,
                )
            })? {
                actual_index = Some(index);
                break;
            }
        }
        let actual_index = actual_index.ok_or_else(|| {
            artifact_error(
                "E-ARTIFACT-SET-CSHARP-FILE-SET",
                ErrorKind::Validation,
                &receipt_path,
                format!(
                    "receipt C# artifact {} is missing from the canonical csharp/ file set",
                    entry.path
                ),
                false,
            )
        })?;
        matched[actual_index] = true;
        let actual = &actual_files[actual_index];
        let actual_hash = hash_regular_file(&actual.path)?;
        if actual_hash != entry.hash {
            return Err(artifact_error(
                "E-ARTIFACT-SET-CSHARP-HASH-MISMATCH",
                ErrorKind::Validation,
                &actual.path,
                format!(
                    "C# artifact {} does not match the SHA-256 recorded in the receipt",
                    entry.path
                ),
                false,
            ));
        }
        validated.push(ValidatedArtifact {
            path: actual.path.clone(),
            relative_path: entry.path.clone(),
            hash: entry.hash.clone(),
        });
    }
    Ok(validated)
}

fn validate_binary_entry(entry: &ArtifactReceiptEntry, path: &Path) -> Result<()> {
    ensure_regular_file(path, "canonical binary")?;
    let actual_hash = hash_regular_file(path)?;
    if actual_hash != entry.hash {
        return Err(artifact_error(
            "E-ARTIFACT-SET-BINARY-HASH-MISMATCH",
            ErrorKind::Validation,
            path,
            "canonical binary does not match the SHA-256 recorded in the receipt",
            false,
        ));
    }
    Ok(())
}

fn validate_root_layout(artifact_root: &Path) -> Result<()> {
    ensure_real_directory(artifact_root, "canonical artifact root")?;
    let expected = [
        CSHARP_DIRECTORY,
        CANONICAL_BINARY_PATH,
        ARTIFACT_SET_RECEIPT_FILENAME,
    ];
    let mut seen = BTreeSet::new();
    let entries = fs::read_dir(artifact_root).map_err(|error| {
        artifact_error(
            "E-ARTIFACT-SET-ROOT-READ",
            ErrorKind::Io,
            artifact_root,
            format!("could not enumerate canonical artifact root: {error}"),
            false,
        )
    })?;
    for entry in entries {
        let entry = entry.map_err(|error| {
            artifact_error(
                "E-ARTIFACT-SET-ROOT-READ",
                ErrorKind::Io,
                artifact_root,
                format!("could not enumerate canonical artifact root: {error}"),
                false,
            )
        })?;
        let name = entry.file_name().into_string().map_err(|_| {
            artifact_error(
                "E-ARTIFACT-SET-UNEXPECTED-ENTRY",
                ErrorKind::Validation,
                &entry.path(),
                "canonical artifact root contains an entry that cannot be represented safely",
                false,
            )
        })?;
        if !expected.contains(&name.as_str()) {
            return Err(artifact_error(
                "E-ARTIFACT-SET-UNEXPECTED-ENTRY",
                ErrorKind::Validation,
                &entry.path(),
                format!("unexpected entry {} in canonical artifact root", name),
                false,
            ));
        }
        seen.insert(name);
    }
    for name in expected {
        if !seen.contains(name) {
            let path = artifact_root.join(name);
            return Err(artifact_error(
                if name == ARTIFACT_SET_RECEIPT_FILENAME {
                    "E-ARTIFACT-SET-RECEIPT-MISSING"
                } else {
                    "E-ARTIFACT-SET-ARTIFACT-MISSING"
                },
                ErrorKind::Validation,
                &path,
                format!("canonical artifact root is missing {}", name),
                true,
            ));
        }
    }
    ensure_real_directory(
        &artifact_root.join(CSHARP_DIRECTORY),
        "canonical C# directory",
    )?;
    ensure_regular_file(
        &artifact_root.join(CANONICAL_BINARY_PATH),
        "canonical binary",
    )?;
    ensure_regular_file(
        &artifact_root.join(ARTIFACT_SET_RECEIPT_FILENAME),
        "artifact-set receipt",
    )?;
    Ok(())
}

fn validate_staged_root_layout(artifact_root: &Path) -> Result<()> {
    ensure_real_directory(artifact_root, "staged canonical artifact root")?;
    let expected = [CSHARP_DIRECTORY, CANONICAL_BINARY_PATH];
    let entries = fs::read_dir(artifact_root).map_err(|error| {
        receipt_generation_error(
            artifact_root,
            format!("could not enumerate staged canonical artifact root: {error}"),
        )
    })?;
    for entry in entries {
        let entry = entry.map_err(|error| {
            receipt_generation_error(
                artifact_root,
                format!("could not enumerate staged canonical artifact root: {error}"),
            )
        })?;
        let name = entry.file_name().into_string().map_err(|_| {
            receipt_generation_error(
                &entry.path(),
                "staged canonical artifact root contains an unsafe entry name",
            )
        })?;
        if !expected.contains(&name.as_str()) {
            return Err(receipt_generation_error(
                &entry.path(),
                format!(
                    "unexpected entry {} in staged canonical artifact root",
                    name
                ),
            ));
        }
    }
    for name in expected {
        let path = artifact_root.join(name);
        if !path_exists(&path) {
            return Err(receipt_generation_error(
                &path,
                format!("staged canonical artifact root is missing {}", name),
            ));
        }
    }
    ensure_real_directory(
        &artifact_root.join(CSHARP_DIRECTORY),
        "staged canonical C# directory",
    )
    .map_err(|error| receipt_generation_error_from(error, artifact_root))?;
    ensure_regular_file(
        &artifact_root.join(CANONICAL_BINARY_PATH),
        "staged canonical binary",
    )
    .map_err(|error| receipt_generation_error_from(error, artifact_root))?;
    Ok(())
}

fn discover_csharp_files(csharp_root: &Path) -> Result<Vec<DiscoveredCSharpFile>> {
    let mut files = Vec::new();
    discover_csharp_files_recursive(csharp_root, csharp_root, &mut files)?;
    files.sort_by(|left, right| left.relative_path.cmp(&right.relative_path));
    reject_filesystem_aliases(
        &files
            .iter()
            .map(|file| file.path.clone())
            .collect::<Vec<_>>(),
        csharp_root,
        "E-ARTIFACT-SET-CSHARP-FILE-ALIAS",
        "canonical C# files alias the same filesystem file",
    )?;
    Ok(files)
}

fn discover_csharp_files_recursive(
    root: &Path,
    directory: &Path,
    files: &mut Vec<DiscoveredCSharpFile>,
) -> Result<()> {
    let entries = fs::read_dir(directory).map_err(|error| {
        artifact_error(
            "E-ARTIFACT-SET-CSHARP-READ",
            ErrorKind::Io,
            directory,
            format!("could not enumerate canonical C# directory: {error}"),
            false,
        )
    })?;
    for entry in entries {
        let entry = entry.map_err(|error| {
            artifact_error(
                "E-ARTIFACT-SET-CSHARP-READ",
                ErrorKind::Io,
                directory,
                format!("could not enumerate canonical C# directory: {error}"),
                false,
            )
        })?;
        let path = entry.path();
        let metadata = fs::symlink_metadata(&path).map_err(|error| {
            artifact_error(
                "E-ARTIFACT-SET-CSHARP-ENTRY",
                ErrorKind::Io,
                &path,
                format!("could not inspect canonical C# entry: {error}"),
                false,
            )
        })?;
        if metadata.file_type().is_symlink() {
            return Err(artifact_error(
                "E-ARTIFACT-SET-CSHARP-SYMLINK",
                ErrorKind::Validation,
                &path,
                "symlink is not an accepted canonical C# artifact or container",
                false,
            ));
        }
        if metadata.file_type().is_dir() {
            discover_csharp_files_recursive(root, &path, files)?;
            continue;
        }
        if !metadata.file_type().is_file() {
            return Err(artifact_error(
                "E-ARTIFACT-SET-CSHARP-SPECIAL",
                ErrorKind::Validation,
                &path,
                "special filesystem entry is not an accepted canonical C# artifact",
                false,
            ));
        }
        let relative = path.strip_prefix(root).map_err(|_| {
            artifact_error(
                "E-ARTIFACT-SET-CSHARP-PATH-UNSAFE",
                ErrorKind::Validation,
                &path,
                "canonical C# artifact path is outside csharp/",
                false,
            )
        })?;
        let relative_path = portable_path(relative, &path)?;
        validate_receipt_relative_path(&relative_path, &path)?;
        files.push(DiscoveredCSharpFile {
            path,
            relative_path,
        });
    }
    Ok(())
}

fn portable_path(relative: &Path, source: &Path) -> Result<String> {
    let mut parts = Vec::new();
    for component in relative.components() {
        let Component::Normal(part) = component else {
            return Err(artifact_error(
                "E-ARTIFACT-SET-CSHARP-PATH-UNSAFE",
                ErrorKind::Validation,
                source,
                "canonical C# artifact path is not a portable relative path",
                false,
            ));
        };
        let part = part.to_str().ok_or_else(|| {
            artifact_error(
                "E-ARTIFACT-SET-CSHARP-PATH-UNSAFE",
                ErrorKind::Validation,
                source,
                "canonical C# artifact path cannot be represented as UTF-8 receipt JSON",
                false,
            )
        })?;
        parts.push(part.to_owned());
    }
    Ok(parts.join("/"))
}

fn validate_receipt_relative_path(path: &str, source: &Path) -> Result<()> {
    // WHY: Receipt paths are a platform-independent slash representation, so
    // host Path::is_absolute alone cannot recognize Windows prefixes while
    // running on Unix.
    // IF REMOVED: a receipt could escape csharp/ after being consumed on a
    // different host, or two spellings could address the same artifact.
    // EVIDENCE: docs/specs/build-pipeline.md; Regression: artifact_receipt_rejects_unsafe_paths.
    let unsafe_path = path.is_empty()
        || path.starts_with('/')
        || path.contains('\\')
        || path.contains('\0')
        || path
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..");
    let windows_drive_prefix =
        path.len() >= 2 && path.as_bytes()[0].is_ascii_alphabetic() && path.as_bytes()[1] == b':';
    if unsafe_path || windows_drive_prefix {
        return Err(artifact_error(
            "E-ARTIFACT-SET-CSHARP-PATH-UNSAFE",
            ErrorKind::Validation,
            source,
            format!(
                "C# receipt path {} must be a non-empty portable relative path under csharp/",
                path
            ),
            false,
        ));
    }
    Ok(())
}

fn safe_receipt_path(csharp_root: &Path, relative_path: &str) -> Result<PathBuf> {
    validate_receipt_relative_path(relative_path, csharp_root)?;
    let mut path = csharp_root.to_path_buf();
    for component in relative_path.split('/') {
        path.push(component);
    }
    Ok(path)
}

fn reject_filesystem_aliases(
    paths: &[PathBuf],
    source: &Path,
    code: &str,
    message: &str,
) -> Result<()> {
    // WHY: Alias detection follows the destination filesystem identity instead
    // of applying global lowercase normalization.
    // IF REMOVED: a case-sensitive filesystem could lose distinct valid files,
    // while a case-insensitive filesystem could accept duplicate receipt paths.
    // EVIDENCE: docs/specs/build-pipeline.md; Regression: artifact_receipt_rejects_unsafe_paths.
    for (index, left) in paths.iter().enumerate() {
        for right in paths.iter().skip(index + 1) {
            let same = is_same_file(left, right).map_err(|error| {
                artifact_error(
                    "E-ARTIFACT-SET-FILESYSTEM-IDENTITY",
                    ErrorKind::Io,
                    source,
                    format!("could not compare canonical artifact filesystem entries: {error}"),
                    false,
                )
            })?;
            if same {
                return Err(artifact_error(
                    code,
                    ErrorKind::Validation,
                    source,
                    message,
                    false,
                ));
            }
        }
    }
    Ok(())
}

fn ensure_real_directory(path: &Path, label: &str) -> Result<()> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        artifact_error(
            "E-ARTIFACT-SET-ENTRY-MISSING",
            ErrorKind::Validation,
            path,
            format!("{} is missing: {error}", label),
            true,
        )
    })?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_dir() {
        return Err(artifact_error(
            "E-ARTIFACT-SET-ENTRY-TYPE",
            ErrorKind::Validation,
            path,
            format!("{} must be a real directory", label),
            false,
        ));
    }
    Ok(())
}

fn ensure_regular_file(path: &Path, label: &str) -> Result<()> {
    let metadata = fs::symlink_metadata(path).map_err(|error| {
        artifact_error(
            "E-ARTIFACT-SET-ENTRY-MISSING",
            ErrorKind::Validation,
            path,
            format!("{} is missing: {error}", label),
            true,
        )
    })?;
    if metadata.file_type().is_symlink() || !metadata.file_type().is_file() {
        return Err(artifact_error(
            "E-ARTIFACT-SET-ENTRY-TYPE",
            ErrorKind::Validation,
            path,
            format!("{} must be a regular file and not a symlink", label),
            false,
        ));
    }
    Ok(())
}

fn hash_regular_file(path: &Path) -> Result<String> {
    ensure_regular_file(path, "artifact")?;
    let bytes = fs::read(path).map_err(|error| {
        artifact_error(
            "E-ARTIFACT-SET-HASH-READ",
            ErrorKind::Io,
            path,
            format!("could not read artifact bytes for SHA-256: {error}"),
            false,
        )
    })?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn path_exists(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok()
}

fn receipt_generation_error(path: &Path, message: impl Into<String>) -> MasterdataError {
    MasterdataError::new("E-BUILD-ARTIFACT-RECEIPT", ErrorKind::Io, message)
        .with_source(path.to_path_buf())
        .with_related_requirement("ARTIFACT-SET-001")
        .with_related_requirement("ARTIFACT-SET-008")
}

fn receipt_generation_error_from(
    error: MasterdataError,
    fallback_source: &Path,
) -> MasterdataError {
    if error.diagnostic().source.is_some() {
        error
    } else {
        error.with_source(fallback_source.to_path_buf())
    }
}

fn artifact_error(
    code: &str,
    kind: ErrorKind,
    source: &Path,
    message: impl Into<String>,
    build_guidance: bool,
) -> MasterdataError {
    let mut error = MasterdataError::new(code, kind, message)
        .with_source(source.to_path_buf())
        .with_related_requirement("ARTIFACT-SET-004")
        .with_related_requirement("ARTIFACT-SET-006");
    if code.contains("RECEIPT") || code.contains("HASH") {
        error = error.with_related_requirement("ARTIFACT-SET-002");
    }
    if code.contains("CSHARP") || code.contains("BINARY") {
        error = error.with_related_requirement("ARTIFACT-SET-003");
    }
    if build_guidance {
        error = error.with_suggestion(
            "run masterdata build to generate a new coherent canonical artifact set",
        );
    }
    error
}

#[cfg(test)]
mod tests {
    use std::fs;

    use serde_json::Value;
    use tempfile::tempdir;

    use super::{
        ARTIFACT_SET_RECEIPT_FILENAME, CANONICAL_BINARY_PATH, create_artifact_set_receipt,
        validate_artifact_set, write_artifact_set_receipt,
    };

    fn artifact_root() -> tempfile::TempDir {
        let directory = tempdir().expect("temporary directory");
        fs::create_dir_all(directory.path().join("output/csharp/nested")).expect("C# directory");
        fs::write(directory.path().join("output/csharp/Item.g.cs"), b"item").expect("C# file");
        fs::write(
            directory.path().join("output/csharp/nested/Enemy.g.cs"),
            b"enemy",
        )
        .expect("nested C# file");
        fs::write(
            directory.path().join("output").join(CANONICAL_BINARY_PATH),
            b"binary",
        )
        .expect("binary");
        directory
    }

    #[test]
    fn artifact_receipt_is_deterministic() {
        let directory = artifact_root();
        let root = directory.path().join("output");
        let first = create_artifact_set_receipt(&root, "game.masterdata").expect("receipt");
        let second = create_artifact_set_receipt(&root, "game.masterdata").expect("receipt");
        assert_eq!(first, second);
    }

    #[test]
    fn artifact_receipt_has_v1_shape_and_sha256_hashes() {
        let directory = artifact_root();
        let root = directory.path().join("output");
        write_artifact_set_receipt(&root, "game.masterdata").expect("receipt");

        let json: Value = serde_json::from_slice(
            &fs::read(root.join(ARTIFACT_SET_RECEIPT_FILENAME)).expect("receipt bytes"),
        )
        .expect("receipt JSON");
        assert_eq!(json["version"], 1);
        assert_eq!(json["project_id"], "game.masterdata");
        assert_eq!(json["hash_algorithm"], "sha256");
        assert_eq!(json["binary"]["path"], CANONICAL_BINARY_PATH);
        assert_eq!(json["csharp"][0]["path"], "Item.g.cs");
        assert_eq!(json["csharp"][1]["path"], "nested/Enemy.g.cs");
        for hash in [
            json["csharp"][0]["hash"].as_str().expect("C# hash"),
            json["csharp"][1]["hash"].as_str().expect("C# hash"),
            json["binary"]["hash"].as_str().expect("binary hash"),
        ] {
            assert_eq!(hash.len(), 64);
            assert!(hash.chars().all(|character| character.is_ascii_hexdigit()));
            assert_eq!(hash, hash.to_ascii_lowercase());
        }
    }

    #[test]
    fn artifact_receipt_covers_complete_csharp_and_binary_set() {
        let directory = artifact_root();
        let root = directory.path().join("output");
        write_artifact_set_receipt(&root, "game.masterdata").expect("receipt");

        let validated = validate_artifact_set(&root, "game.masterdata").expect("valid set");
        assert_eq!(validated.csharp.len(), 2);
        assert_eq!(validated.binary.relative_path, CANONICAL_BINARY_PATH);
    }

    #[test]
    fn artifact_receipt_rejects_unsafe_paths() {
        let directory = artifact_root();
        let root = directory.path().join("output");
        write_artifact_set_receipt(&root, "game.masterdata").expect("receipt");
        let receipt_path = root.join(ARTIFACT_SET_RECEIPT_FILENAME);
        let mut json: Value = serde_json::from_slice(&fs::read(&receipt_path).expect("receipt"))
            .expect("receipt JSON");
        json["csharp"][0]["path"] = Value::String("../Item.g.cs".to_owned());
        fs::write(&receipt_path, serde_json::to_vec(&json).expect("JSON")).expect("rewrite");

        let error = validate_artifact_set(&root, "game.masterdata").expect_err("unsafe path");
        assert_eq!(error.diagnostic().code, "E-ARTIFACT-SET-CSHARP-PATH-UNSAFE");
    }

    #[test]
    fn artifact_receipt_rejects_windows_and_absolute_path_spellings() {
        for unsafe_path in [
            "/Item.g.cs",
            r"C:\Item.g.cs",
            "C:Item.g.cs",
            r"\\server\share\Item.g.cs",
            "nested/../../Item.g.cs",
            r"nested\Item.g.cs",
        ] {
            let directory = artifact_root();
            let root = directory.path().join("output");
            write_artifact_set_receipt(&root, "game.masterdata").expect("receipt");
            let receipt_path = root.join(ARTIFACT_SET_RECEIPT_FILENAME);
            let mut json: Value =
                serde_json::from_slice(&fs::read(&receipt_path).expect("receipt")).expect("JSON");
            json["csharp"][0]["path"] = Value::String(unsafe_path.to_owned());
            fs::write(&receipt_path, serde_json::to_vec(&json).expect("JSON"))
                .expect("rewrite receipt");

            let error =
                validate_artifact_set(&root, "game.masterdata").expect_err("unsafe path spelling");
            assert_eq!(error.diagnostic().code, "E-ARTIFACT-SET-CSHARP-PATH-UNSAFE");
        }
    }

    #[test]
    fn missing_receipt_is_rejected_with_build_guidance() {
        let directory = artifact_root();
        let root = directory.path().join("output");

        let error = validate_artifact_set(&root, "game.masterdata").expect_err("missing receipt");

        assert_eq!(error.diagnostic().code, "E-ARTIFACT-SET-RECEIPT-MISSING");
        assert!(
            error
                .diagnostic()
                .suggestion
                .as_deref()
                .is_some_and(|suggestion| suggestion.contains("masterdata build"))
        );
    }

    #[test]
    fn malformed_receipt_is_rejected_without_repair() {
        let directory = artifact_root();
        let root = directory.path().join("output");
        let receipt_path = root.join(ARTIFACT_SET_RECEIPT_FILENAME);
        fs::write(&receipt_path, b"{not-json").expect("malformed receipt");

        let error = validate_artifact_set(&root, "game.masterdata").expect_err("malformed receipt");

        assert_eq!(error.diagnostic().code, "E-ARTIFACT-SET-RECEIPT-PARSE");
        assert_eq!(
            fs::read(&receipt_path).expect("receipt bytes"),
            b"{not-json"
        );
    }

    #[test]
    fn receipt_validation_rejects_unsupported_version_algorithm_and_project() {
        for (field, value, expected_code) in [
            ("version", Value::from(2), "E-ARTIFACT-SET-RECEIPT-VERSION"),
            (
                "hash_algorithm",
                Value::from("sha512"),
                "E-ARTIFACT-SET-RECEIPT-HASH-ALGORITHM",
            ),
            (
                "project_id",
                Value::from("other.project"),
                "E-ARTIFACT-SET-PROJECT-MISMATCH",
            ),
        ] {
            let directory = artifact_root();
            let root = directory.path().join("output");
            write_artifact_set_receipt(&root, "game.masterdata").expect("receipt");
            let receipt_path = root.join(ARTIFACT_SET_RECEIPT_FILENAME);
            let mut json: Value =
                serde_json::from_slice(&fs::read(&receipt_path).expect("receipt")).expect("JSON");
            json[field] = value;
            fs::write(&receipt_path, serde_json::to_vec(&json).expect("JSON"))
                .expect("rewrite receipt");

            let error =
                validate_artifact_set(&root, "game.masterdata").expect_err("unsupported receipt");
            assert_eq!(error.diagnostic().code, expected_code);
        }
    }

    #[test]
    fn receipt_validation_rejects_duplicate_paths_and_invalid_hashes() {
        let directory = artifact_root();
        let root = directory.path().join("output");
        write_artifact_set_receipt(&root, "game.masterdata").expect("receipt");
        let receipt_path = root.join(ARTIFACT_SET_RECEIPT_FILENAME);
        let mut json: Value =
            serde_json::from_slice(&fs::read(&receipt_path).expect("receipt")).expect("JSON");
        json["csharp"][1]["path"] = json["csharp"][0]["path"].clone();
        fs::write(&receipt_path, serde_json::to_vec(&json).expect("JSON")).expect("rewrite");
        let error = validate_artifact_set(&root, "game.masterdata").expect_err("duplicate path");
        assert_eq!(
            error.diagnostic().code,
            "E-ARTIFACT-SET-CSHARP-PATH-DUPLICATE"
        );

        let directory = artifact_root();
        let root = directory.path().join("output");
        write_artifact_set_receipt(&root, "game.masterdata").expect("receipt");
        let receipt_path = root.join(ARTIFACT_SET_RECEIPT_FILENAME);
        let mut json: Value =
            serde_json::from_slice(&fs::read(&receipt_path).expect("receipt")).expect("JSON");
        json["binary"]["hash"] = Value::String("ABC".to_owned());
        fs::write(&receipt_path, serde_json::to_vec(&json).expect("JSON")).expect("rewrite");
        let error = validate_artifact_set(&root, "game.masterdata").expect_err("invalid hash");
        assert_eq!(error.diagnostic().code, "E-ARTIFACT-SET-BINARY-HASH");
    }

    #[test]
    fn receipt_validation_rejects_missing_extra_and_tampered_artifacts() {
        let directory = artifact_root();
        let root = directory.path().join("output");
        write_artifact_set_receipt(&root, "game.masterdata").expect("receipt");
        fs::remove_file(root.join("csharp/Item.g.cs")).expect("remove C# file");
        let error = validate_artifact_set(&root, "game.masterdata").expect_err("missing C# file");
        assert_eq!(error.diagnostic().code, "E-ARTIFACT-SET-CSHARP-FILE-SET");

        let directory = artifact_root();
        let root = directory.path().join("output");
        write_artifact_set_receipt(&root, "game.masterdata").expect("receipt");
        fs::write(root.join("csharp/Extra.g.cs"), b"extra").expect("extra C# file");
        let error = validate_artifact_set(&root, "game.masterdata").expect_err("extra C# file");
        assert_eq!(error.diagnostic().code, "E-ARTIFACT-SET-CSHARP-FILE-SET");

        let directory = artifact_root();
        let root = directory.path().join("output");
        write_artifact_set_receipt(&root, "game.masterdata").expect("receipt");
        fs::write(root.join(CANONICAL_BINARY_PATH), b"tampered").expect("tampered binary");
        let error = validate_artifact_set(&root, "game.masterdata").expect_err("tampered binary");
        assert_eq!(
            error.diagnostic().code,
            "E-ARTIFACT-SET-BINARY-HASH-MISMATCH"
        );
    }

    #[test]
    fn receipt_validation_rejects_unexpected_top_level_entry() {
        let directory = artifact_root();
        let root = directory.path().join("output");
        write_artifact_set_receipt(&root, "game.masterdata").expect("receipt");
        fs::write(root.join("unexpected.txt"), b"unexpected").expect("unexpected entry");

        let error = validate_artifact_set(&root, "game.masterdata").expect_err("unexpected entry");

        assert_eq!(error.diagnostic().code, "E-ARTIFACT-SET-UNEXPECTED-ENTRY");
    }

    #[cfg(unix)]
    #[test]
    fn receipt_validation_rejects_artifact_symlinks() {
        use std::os::unix::fs::symlink;

        let directory = artifact_root();
        let root = directory.path().join("output");
        write_artifact_set_receipt(&root, "game.masterdata").expect("receipt");
        let target = root.join("csharp/Item.g.cs");
        let link = root.join("csharp/Link.g.cs");
        fs::remove_file(&target).expect("remove target");
        symlink(root.join("masterdata.bytes"), &link).expect("C# symlink");

        let error = validate_artifact_set(&root, "game.masterdata").expect_err("symlink");

        assert_eq!(error.diagnostic().code, "E-ARTIFACT-SET-CSHARP-SYMLINK");
    }
}
