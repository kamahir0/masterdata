use std::path::PathBuf;

use masterdata_core::{
    AddedRecordDraft, AddedRecordField, AuthoringQuery, AuthoringValue, BuildProfileInfo,
    Diagnostic, ProjectConfig, ProjectDocuments, RecordTagEdit, RecordValueEdit,
    SourceRecordMutation, data_file_snapshot, dry_run_source_record_mutation, parse_yaml_document,
    query_data_file, source_content_identity, table_snapshot, type_snapshot, validate_documents,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
enum Request {
    ParseConfig {
        source: String,
    },
    Analyze {
        files: Vec<InputFile>,
    },
    OpenData {
        files: Vec<InputFile>,
        target: String,
        #[serde(default)]
        profiles: Vec<BuildProfileInfo>,
    },
    OpenTable {
        files: Vec<InputFile>,
        target: String,
    },
    OpenType {
        files: Vec<InputFile>,
        target: String,
    },
    QueryData {
        files: Vec<InputFile>,
        target: String,
        mutation: WebMutation,
        query: AuthoringQuery,
        #[serde(default)]
        profiles: Vec<BuildProfileInfo>,
    },
    PreviewData {
        files: Vec<InputFile>,
        target: String,
        mutation: WebMutation,
    },
    Identity {
        source: String,
    },
}

#[derive(Deserialize)]
struct InputFile {
    path: String,
    source: String,
}

#[derive(Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct WebMutation {
    #[serde(default)]
    edits: Vec<WebEdit>,
    #[serde(default)]
    added_records: Vec<WebRecord>,
    #[serde(default)]
    deleted_record_indices: Vec<usize>,
    #[serde(default)]
    tag_edits: Vec<WebTagEdit>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WebEdit {
    record_index: usize,
    field: String,
    value: AuthoringValue,
}

#[derive(Deserialize)]
struct WebRecord {
    fields: Vec<WebField>,
    #[serde(default)]
    tags: Vec<String>,
}

#[derive(Deserialize)]
struct WebField {
    field: String,
    value: AuthoringValue,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WebTagEdit {
    record_index: usize,
    tags: Vec<String>,
}

impl From<WebMutation> for SourceRecordMutation {
    fn from(value: WebMutation) -> Self {
        Self {
            edits: value
                .edits
                .into_iter()
                .map(|item| RecordValueEdit {
                    record_index: item.record_index,
                    field: item.field,
                    value: item.value,
                })
                .collect(),
            additions: value
                .added_records
                .into_iter()
                .map(|record| AddedRecordDraft {
                    fields: record
                        .fields
                        .into_iter()
                        .map(|item| AddedRecordField {
                            field: item.field,
                            value: item.value,
                        })
                        .collect(),
                    tags: record.tags,
                })
                .collect(),
            deletions: value.deleted_record_indices,
            tag_edits: value
                .tag_edits
                .into_iter()
                .map(|item| RecordTagEdit {
                    record_index: item.record_index,
                    tags: item.tags,
                })
                .collect(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SourceFile {
    path: String,
    kind: String,
    table: Option<String>,
    type_name: Option<String>,
    diagnostic: Option<Diagnostic>,
}

fn process(input: &[u8]) -> Value {
    let request: Request = match serde_json::from_slice(input) {
        Ok(request) => request,
        Err(error) => {
            return json!({ "error": { "message": format!("invalid Web adapter request: {error}") } });
        }
    };
    match request {
        Request::ParseConfig { source } => match toml::from_str::<ProjectConfig>(&source) {
            Ok(config) => match config.validate() {
                Ok(()) => json!({ "config": config }),
                Err(error) => json!({ "error": { "diagnostic": error.diagnostic() } }),
            },
            Err(error) => {
                json!({ "error": { "message": format!("invalid masterdata.toml: {error}") } })
            }
        },
        Request::Analyze { files } => {
            let (documents, entries, parse_diagnostics) = parse_files(files);
            let mut validation = validate_documents(&documents);
            validation.diagnostics.extend(parse_diagnostics);
            validation.valid = validation.diagnostics.is_empty();
            json!({ "files": entries, "validation": validation })
        }
        Request::OpenData {
            files,
            target,
            profiles,
        } => {
            let (documents, _, diagnostics) = parse_files(files);
            match data_file_snapshot(
                PathBuf::from(".").as_path(),
                &profiles,
                &documents,
                diagnostics,
                PathBuf::from(&target).as_path(),
            ) {
                Ok(snapshot) => json!({ "snapshot": snapshot }),
                Err(error) => json!({ "error": { "diagnostic": error.diagnostic() } }),
            }
        }
        Request::OpenTable { files, target } => {
            let (documents, _, _) = parse_files(files);
            match table_snapshot(&documents, PathBuf::from(&target).as_path(), &target) {
                Ok(snapshot) => json!({ "snapshot": snapshot }),
                Err(error) => json!({ "error": { "diagnostic": error.diagnostic() } }),
            }
        }
        Request::OpenType { files, target } => {
            let (documents, _, _) = parse_files(files);
            match type_snapshot(&documents, PathBuf::from(&target).as_path(), &target) {
                Ok(snapshot) => json!({ "snapshot": snapshot }),
                Err(error) => json!({ "error": { "diagnostic": error.diagnostic() } }),
            }
        }
        Request::QueryData {
            files,
            target,
            mutation,
            query,
            profiles,
        } => {
            let (documents, _, diagnostics) = parse_files(files);
            match query_data_file(
                PathBuf::from(".").as_path(),
                &profiles,
                &documents,
                diagnostics,
                PathBuf::from(&target).as_path(),
                &mutation.into(),
                &query,
            ) {
                Ok(result) => json!({ "result": result }),
                Err(error) => json!({ "error": { "diagnostic": error.diagnostic() } }),
            }
        }
        Request::PreviewData {
            files,
            target,
            mutation,
        } => {
            let (documents, _, mut parse_diagnostics) = parse_files(files);
            match dry_run_source_record_mutation(
                &documents,
                PathBuf::from(&target).as_path(),
                &mutation.into(),
            ) {
                Ok(dry_run) => {
                    let mut validation = validate_documents(&dry_run.transformed_documents);
                    validation.diagnostics.append(&mut parse_diagnostics);
                    validation.valid = validation.diagnostics.is_empty();
                    json!({ "preview": {
                        "candidateSource": dry_run.plan.candidate_source,
                        "candidateContentIdentity": dry_run.plan.candidate_content_identity,
                        "changed": dry_run.plan.changed,
                        "validation": validation,
                    }})
                }
                Err(error) => json!({ "error": { "diagnostic": error.diagnostic() } }),
            }
        }
        Request::Identity { source } => json!({ "identity": source_content_identity(&source) }),
    }
}

fn parse_files(files: Vec<InputFile>) -> (ProjectDocuments, Vec<SourceFile>, Vec<Diagnostic>) {
    let mut documents = ProjectDocuments::default();
    let mut entries = Vec::with_capacity(files.len());
    let mut parse_diagnostics = Vec::new();
    for file in files {
        let path = PathBuf::from(&file.path);
        match parse_yaml_document(path, &file.source) {
            Ok(loaded) => {
                entries.push(SourceFile {
                    path: file.path,
                    kind: loaded.document.kind().to_owned(),
                    table: loaded.document.table_identity().map(str::to_owned),
                    type_name: loaded.document.type_name().map(str::to_owned),
                    diagnostic: None,
                });
                documents.files.push(loaded);
            }
            Err(error) => {
                let diagnostic = error.diagnostic().clone();
                parse_diagnostics.push(diagnostic.clone());
                entries.push(SourceFile {
                    path: file.path,
                    kind: "invalid".to_owned(),
                    table: None,
                    type_name: None,
                    diagnostic: Some(diagnostic),
                });
            }
        }
    }
    (documents, entries, parse_diagnostics)
}

// The pointer/length ABI keeps the browser adapter independent of a JS runtime
// generator. All domain parsing and validation still execute in shared Rust.
#[unsafe(no_mangle)]
pub extern "C" fn md_alloc(len: u32) -> *mut u8 {
    let bytes = vec![0_u8; len as usize].into_boxed_slice();
    Box::into_raw(bytes) as *mut u8
}

/// # Safety
/// `pointer` and `len` must be a live allocation returned by `md_alloc` or
/// `md_call`, and must not have been freed before this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn md_free(pointer: *mut u8, len: u32) {
    unsafe {
        drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
            pointer,
            len as usize,
        )));
    }
}

/// # Safety
/// `pointer` and `len` must describe a live `md_alloc` allocation. This call
/// takes ownership of it and returns a new packed pointer/length allocation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn md_call(pointer: *mut u8, len: u32) -> u64 {
    let input = unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(pointer, len as usize)) };
    let output = serde_json::to_vec(&process(&input)).expect("JSON response");
    let length = output.len() as u32;
    let pointer = Box::into_raw(output.into_boxed_slice()) as *mut u8;
    ((length as u64) << 32) | (pointer as u32 as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn browser_analysis_uses_shared_validation() {
        let request = json!({
            "op": "analyze",
            "files": [
                { "path": "schemas/item.yaml", "source": "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\nprimaryKey:\n  fields: [id]\n" },
                { "path": "data/item.yaml", "source": "kind: data\ntable: item\nrecords:\n  - id: 1\n" }
            ]
        });
        let result = process(&serde_json::to_vec(&request).unwrap());
        assert_eq!(result["validation"]["valid"], true);
        assert_eq!(result["files"][1]["kind"], "data");
    }
}
