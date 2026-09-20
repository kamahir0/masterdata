//! Native filesystem binding for shared authoring query semantics.

use std::path::Path;

use masterdata_core::{AuthoringQuery, Result, SourceRecordMutation};
use serde::Deserialize;

use crate::NativeApplicationService;
use crate::authoring::{AuthoringRecordMutation, load_authoring_documents, resolve_source_file};

pub use masterdata_core::DataFileQueryResult;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DataFileQueryRequest {
    pub relative_path: String,
    pub base_source: String,
    #[serde(default)]
    pub mutation: AuthoringRecordMutation,
    #[serde(default)]
    pub query: AuthoringQuery,
}

impl NativeApplicationService {
    pub fn query_data_file(
        &self,
        explicit_project: Option<&Path>,
        current_dir: &Path,
        request: &DataFileQueryRequest,
    ) -> Result<DataFileQueryResult> {
        let project = masterdata_core::Project::discover(explicit_project, current_dir)?;
        let target = resolve_source_file(&project, &request.relative_path)?;
        let (documents, parse_diagnostics) =
            load_authoring_documents(&project, Some((&target, &request.base_source)))?;
        masterdata_core::query_data_file(
            project.root(),
            &project.info().profiles,
            &documents,
            parse_diagnostics,
            &target,
            &SourceRecordMutation::from(&request.mutation),
            &request.query,
        )
    }
}
#[cfg(test)]
mod tests {
    use super::{DataFileQueryRequest, NativeApplicationService};
    use masterdata_core::{AuthoringQuery, QuerySort, SortDirection};
    use std::fs;
    use tempfile::TempDir;

    fn project() -> (TempDir, String) {
        let temp = tempfile::tempdir().expect("temporary project");
        fs::create_dir_all(temp.path().join("sources/data")).expect("data root");
        fs::create_dir_all(temp.path().join("sources/schemas")).expect("schema root");
        fs::write(
            temp.path().join("masterdata.toml"),
            "[project]\nid = \"query.test\"\nname = \"Query\"\nversion = \"0.1.0\"\n\n[sources]\nroots = [\"sources\"]\n\n[build]\nartifact_dir = \".masterdata/output\"\ncache = \".masterdata/cache\"\n",
        )
        .expect("config");
        fs::write(
            temp.path().join("sources/schemas/item.yaml"),
            "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: label\n    type: string\nprimaryKey:\n  fields: [id]\nsecondaryKeys: []\n",
        )
        .expect("schema");
        let source = "kind: data\ntable: item\nrecords:\n  - id: 1\n    label: bravo\n  - id: 2\n    label: alpha\n  - id: 3\n    label: charlie\n".to_owned();
        fs::write(temp.path().join("sources/data/item.yaml"), &source).expect("data");
        (temp, source)
    }

    #[test]
    fn query_preserves_sort_order_and_pending_delete_occurrence_indexes() {
        let (temp, source) = project();
        let service = NativeApplicationService::new();
        let sorted = service
            .query_data_file(
                Some(temp.path()),
                temp.path(),
                &DataFileQueryRequest {
                    relative_path: "sources/data/item.yaml".into(),
                    base_source: source.clone(),
                    mutation: Default::default(),
                    query: AuthoringQuery {
                        search: String::new(),
                        filters: Vec::new(),
                        sort: Some(QuerySort {
                            field: "label".into(),
                            direction: SortDirection::Descending,
                        }),
                    },
                },
            )
            .expect("sorted query");
        assert_eq!(sorted.ordered_record_indices, vec![2, 0, 1]);

        let mutation = crate::authoring::AuthoringRecordMutation {
            deleted_record_indices: vec![0],
            ..Default::default()
        };
        let visible = service
            .query_data_file(
                Some(temp.path()),
                temp.path(),
                &DataFileQueryRequest {
                    relative_path: "sources/data/item.yaml".into(),
                    base_source: source,
                    mutation,
                    query: AuthoringQuery::default(),
                },
            )
            .expect("query with pending delete");
        assert_eq!(visible.ordered_record_indices, vec![1, 2]);
        assert_eq!(visible.total_count, 2);
    }
}
