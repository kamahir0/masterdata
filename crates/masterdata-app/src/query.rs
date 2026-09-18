//! Application binding for shared authoring query semantics.

use std::collections::BTreeSet;
use std::path::Path;

use masterdata_core::{
    AuthoringQuery, QueryRow, Result, SourceRecordMutation, apply_authoring_query,
    dry_run_source_record_mutation,
};
use serde::{Deserialize, Serialize};

use crate::NativeApplicationService;
use crate::authoring::{
    AuthoringRecordMutation, DataFileSnapshot, data_file_snapshot, load_authoring_documents,
    resolve_source_file,
};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DataFileQueryResult {
    pub snapshot: DataFileSnapshot,
    pub ordered_record_indices: Vec<usize>,
    pub total_count: usize,
    pub displayed_count: usize,
    pub query: AuthoringQuery,
}

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
        // Keep source occurrence indexes stable while evaluating a query. A
        // pending delete is a separate UI state; physically applying it here
        // would compact the YAML sequence and make the result reconnect to a
        // different occurrence. Other local mutations can be materialized in
        // the captured documents because they do not change existing indexes.
        let visible_deletions = request
            .mutation
            .deleted_record_indices
            .iter()
            .copied()
            .collect::<BTreeSet<_>>();
        let mut query_mutation = request.mutation.clone();
        query_mutation.deleted_record_indices.clear();
        let documents = if query_mutation.edits.is_empty()
            && query_mutation.added_records.is_empty()
            && query_mutation.tag_edits.is_empty()
        {
            documents
        } else {
            dry_run_source_record_mutation(
                &documents,
                &target,
                &SourceRecordMutation::from(&query_mutation),
            )?
            .transformed_documents
        };
        let snapshot = data_file_snapshot(&project, &documents, parse_diagnostics, &target)?;
        let shapes = query_shapes(&snapshot, &request.query)?;
        let rows = snapshot
            .rows
            .iter()
            .filter(|row| !visible_deletions.contains(&row.record_index))
            .map(|row| QueryRow {
                source_order: row.record_index,
                values: shapes
                    .iter()
                    .map(|(_, index)| row.cells[*index].value.clone())
                    .collect(),
            })
            .collect::<Vec<_>>();
        let ordered_positions = apply_authoring_query(
            &shapes
                .iter()
                .map(|(shape, _)| shape.clone())
                .collect::<Vec<_>>(),
            &rows,
            &request.query,
        )?;
        let ordered_record_indices = ordered_positions
            .into_iter()
            .map(|position| rows[position].source_order)
            .collect::<Vec<_>>();
        Ok(DataFileQueryResult {
            total_count: rows.len(),
            displayed_count: ordered_record_indices.len(),
            snapshot,
            ordered_record_indices,
            query: request.query.clone(),
        })
    }
}

fn query_shapes(
    snapshot: &DataFileSnapshot,
    query: &AuthoringQuery,
) -> Result<Vec<(masterdata_core::ResolvedAuthoringField, usize)>> {
    let mut requested = BTreeSet::new();
    requested.extend(query.filters.iter().map(|filter| filter.field.clone()));
    if let Some(sort) = &query.sort {
        requested.insert(sort.field.clone());
    }
    let mut result = Vec::new();
    for (index, column) in snapshot.columns.iter().enumerate() {
        if !requested.is_empty() && !requested.contains(&column.name) && query.search.is_empty() {
            continue;
        }
        let Some(shape) = column.shape.clone() else {
            if requested.contains(&column.name) {
                return Err(masterdata_core::MasterdataError::new(
                    "E-AUTHORING-QUERY-UNAVAILABLE",
                    masterdata_core::ErrorKind::Validation,
                    format!("query field `{}` has no resolved type shape", column.name),
                )
                .with_related_requirement("AUTHORING-QUERY-002"));
            }
            continue;
        };
        result.push((shape, index));
    }
    Ok(result)
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
