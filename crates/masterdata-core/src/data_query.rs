use std::{collections::BTreeSet, path::Path};

use serde::Serialize;

use crate::{
    AuthoringQuery, BuildProfileInfo, DataFileSnapshot, Diagnostic, ErrorKind, MasterdataError,
    ProjectDocuments, QueryRow, ResolvedAuthoringField, Result, SourceRecordMutation,
    apply_authoring_query, data_file_snapshot, dry_run_source_record_mutation,
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

pub fn query_data_file(
    project_root: &Path,
    profiles: &[BuildProfileInfo],
    documents: &ProjectDocuments,
    parse_diagnostics: Vec<Diagnostic>,
    target: &Path,
    mutation: &SourceRecordMutation,
    query: &AuthoringQuery,
) -> Result<DataFileQueryResult> {
    // Pending deletions remain at their original occurrence indexes until Save.
    let visible_deletions = mutation.deletions.iter().copied().collect::<BTreeSet<_>>();
    let mut query_mutation = mutation.clone();
    query_mutation.deletions.clear();
    let projected = if query_mutation.edits.is_empty()
        && query_mutation.additions.is_empty()
        && query_mutation.tag_edits.is_empty()
    {
        documents.clone()
    } else {
        dry_run_source_record_mutation(documents, target, &query_mutation)?.transformed_documents
    };
    let snapshot = data_file_snapshot(
        project_root,
        profiles,
        &projected,
        parse_diagnostics,
        target,
    )?;
    let shapes = query_shapes(&snapshot, query)?;
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
    let ordered_record_indices = apply_authoring_query(
        &shapes
            .iter()
            .map(|(shape, _)| shape.clone())
            .collect::<Vec<_>>(),
        &rows,
        query,
    )?
    .into_iter()
    .map(|position| rows[position].source_order)
    .collect::<Vec<_>>();
    Ok(DataFileQueryResult {
        total_count: rows.len(),
        displayed_count: ordered_record_indices.len(),
        snapshot,
        ordered_record_indices,
        query: query.clone(),
    })
}

fn query_shapes(
    snapshot: &DataFileSnapshot,
    query: &AuthoringQuery,
) -> Result<Vec<(ResolvedAuthoringField, usize)>> {
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
                return Err(MasterdataError::new(
                    "E-AUTHORING-QUERY-UNAVAILABLE",
                    ErrorKind::Validation,
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
