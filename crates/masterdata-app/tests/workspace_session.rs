use masterdata_app::*;
use masterdata_core::read_trace::measure_read;
use masterdata_core::*;
use std::{fs, path::Path};

fn project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    initialize_project(
        dir.path(),
        &InitOptions {
            project_id: "navigation.test".into(),
            name: "Navigation".into(),
            version: "0.1.0".into(),
        },
    )
    .unwrap();
    fs::write(dir.path().join("sources/schema.yaml"), "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\nprimaryKey:\n  fields: [id]\nrecords:\n  - id: 1\n").unwrap();
    fs::write(
        dir.path().join("sources/a.yaml"),
        "kind: data\ntable: item\nrecords:\n  - id: 2\n",
    )
    .unwrap();
    fs::write(
        dir.path().join("sources/b.yaml"),
        "kind: data\ntable: item\nrecords:\n  - id: 3\n",
    )
    .unwrap();
    dir
}
fn open(root: &Path) -> WorkspaceAuthoringSession {
    WorkspaceAuthoringSession::open(Some(root), root).unwrap()
}

#[test]
fn many_views_share_one_parse_and_do_not_validate_on_selection() {
    let dir = project();
    let (session, metrics) = measure_read(|| open(dir.path()));
    assert_eq!(metrics.phases["yamlParse"].calls, 3);
    assert_eq!(metrics.phases["discovery"].calls, 1);
    for path in [
        "sources/schema.yaml",
        "sources/a.yaml",
        "sources/b.yaml",
        "sources/a.yaml",
    ] {
        let (view, trace) = measure_read(|| session.select_source(path).unwrap());
        assert_eq!(view.path, path);
        assert!(view.validation_pending);
        assert_eq!(view.context.unwrap().record_sources.len(), 3);
        assert!(view.data.unwrap().columns[0].editable);
        for phase in ["yamlParse", "enumeration", "discovery", "validation"] {
            assert!(!trace.phases.contains_key(phase), "unexpected {phase}");
        }
    }
    let (report, trace) = measure_read(|| session.validate());
    assert!(report.validation.valid);
    assert_eq!(trace.phases["validation"].calls, 1);
    let (_, trace) = measure_read(|| session.validate());
    assert!(!trace.phases.contains_key("validation"));
    assert!(
        !session
            .select_source("sources/a.yaml")
            .unwrap()
            .validation_pending
    );
}

#[test]
fn schema_selection_redirects_in_the_same_generation_without_another_request() {
    let dir = project();
    let schema = fs::read_to_string(dir.path().join("sources/schema.yaml")).unwrap();
    fs::write(
        dir.path().join("sources/schema.yaml"),
        schema.split("records:").next().unwrap(),
    )
    .unwrap();
    let session = open(dir.path());
    let view = session.select_source("sources/schema.yaml").unwrap();
    assert_eq!(view.requested_path, "sources/schema.yaml");
    assert_eq!(view.path, "sources/a.yaml");
    assert_eq!(view.context.unwrap().schema_path, "sources/schema.yaml");
    assert_eq!(view.data.unwrap().rows[0].cells[0].text, "2");
}

#[test]
fn exact_freshness_reparses_only_changed_bytes_even_when_metadata_matches() {
    let dir = project();
    let session = open(dir.path());
    let before = session.select_source("sources/a.yaml").unwrap();
    let path = dir.path().join("sources/a.yaml");
    let old_stamp = fs::metadata(&path).unwrap().modified().unwrap();
    let source = fs::read_to_string(&path).unwrap().replace("id: 2", "id: 9");
    fs::write(&path, &source).unwrap();
    fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .unwrap()
        .set_times(fs::FileTimes::new().set_modified(old_stamp))
        .unwrap();
    let (after, trace) = measure_read(|| session.select_source("sources/a.yaml").unwrap());
    assert!(after.generation > before.generation);
    assert_eq!(trace.phases["yamlParse"].calls, 1);
    assert_eq!(after.data.unwrap().rows[0].cells[0].text, "9");
    assert!(after.validation_pending);
}

#[test]
fn invalid_and_deleted_sources_never_return_the_old_editable_view() {
    let dir = project();
    let session = open(dir.path());
    session.select_source("sources/a.yaml").unwrap();
    fs::write(dir.path().join("sources/a.yaml"), "kind: [\n").unwrap();
    assert!(session.select_source("sources/a.yaml").is_err());
    let status = session.refresh_inventory().unwrap();
    assert_eq!(
        status
            .workspace
            .files
            .iter()
            .find(|file| file.path == "sources/a.yaml")
            .unwrap()
            .kind,
        "invalid"
    );
    // An unrelated parse failure does not erase Core's local authoring capability.
    assert!(
        session
            .select_source("sources/b.yaml")
            .unwrap()
            .data
            .unwrap()
            .columns[0]
            .editable
    );
    fs::remove_file(dir.path().join("sources/b.yaml")).unwrap();
    assert!(session.select_source("sources/b.yaml").is_err());
}

#[test]
fn schema_and_type_dependencies_are_fresh_checked_before_edit_permission() {
    let dir = project();
    let session = open(dir.path());
    let schema = dir.path().join("sources/schema.yaml");
    fs::write(
        &schema,
        fs::read_to_string(&schema)
            .unwrap()
            .replace("type: int", "type: Missing"),
    )
    .unwrap();
    let view = session.select_source("sources/a.yaml").unwrap();
    assert!(!view.data.unwrap().columns[0].editable);
    fs::remove_file(schema).unwrap();
    assert!(session.select_source("sources/a.yaml").is_err());
}

#[test]
fn inventory_addition_is_reconciled_without_reparsing_unchanged_sources() {
    let dir = project();
    let session = open(dir.path());
    fs::write(
        dir.path().join("sources/new.yaml"),
        "kind: data\ntable: item\nrecords: []\n",
    )
    .unwrap();
    let (status, trace) = measure_read(|| session.refresh_inventory().unwrap());
    assert_eq!(trace.phases["yamlParse"].calls, 1);
    assert_eq!(status.workspace.files.len(), 4);
    assert_eq!(
        session
            .select_source("sources/new.yaml")
            .unwrap()
            .context
            .unwrap()
            .record_sources
            .len(),
        4
    );
    let (_, trace) = measure_read(|| session.refresh_inventory().unwrap());
    assert!(!trace.phases.contains_key("yamlParse"));
}

#[test]
fn cached_read_identity_cannot_authorize_a_save_after_external_mutation() {
    let dir = project();
    let session = open(dir.path());
    let snapshot = session
        .select_source("sources/a.yaml")
        .unwrap()
        .data
        .unwrap();
    let path = dir.path().join(&snapshot.path);
    let external = snapshot.base_source.replace("id: 2", "id: 7");
    fs::write(&path, &external).unwrap();
    let report = NativeApplicationService::new()
        .save_data_file_mutation(
            Some(dir.path()),
            dir.path(),
            &snapshot.path,
            &snapshot.base_source,
            &snapshot.base_content_identity,
            &AuthoringRecordMutation {
                edits: vec![AuthoringEdit {
                    record_index: 0,
                    field: "id".into(),
                    value: AuthoringValue::Number { value: "8".into() },
                }],
                ..Default::default()
            },
            None,
        )
        .unwrap();
    assert_eq!(report.status, SourceSaveStatus::Conflict);
    assert_eq!(fs::read_to_string(path).unwrap(), external);
}

#[test]
fn changed_binding_requires_explicit_reload_without_reusing_old_authority() {
    let dir = project();
    let session = open(dir.path());
    let path = dir.path().join("masterdata.toml");
    fs::write(
        &path,
        fs::read_to_string(&path).unwrap() + "\n# externally changed\n",
    )
    .unwrap();
    assert_eq!(
        session
            .select_source("sources/a.yaml")
            .unwrap_err()
            .diagnostic()
            .code,
        "E-WORKSPACE-BINDING-CHANGED"
    );
    assert_eq!(
        session
            .source_content("sources/a.yaml")
            .unwrap_err()
            .diagnostic()
            .code,
        "E-WORKSPACE-BINDING-CHANGED"
    );
}

#[test]
fn dirty_overlay_and_query_reuse_base_parse_without_becoming_source_authority() {
    let dir = project();
    let session = open(dir.path());
    let snapshot = session
        .select_source("sources/a.yaml")
        .unwrap()
        .data
        .unwrap();
    let mutation = AuthoringRecordMutation {
        edits: vec![AuthoringEdit {
            record_index: 0,
            field: "id".into(),
            value: AuthoringValue::Number { value: "8".into() },
        }],
        ..Default::default()
    };
    let (preview, trace) = measure_read(|| {
        session
            .preview_data_file(&snapshot.path, &snapshot.base_source, &mutation)
            .unwrap()
    });
    assert!(!trace.phases.contains_key("discovery"));
    assert!(!trace.phases.contains_key("enumeration"));
    let (native, native_trace) = measure_read(|| {
        NativeApplicationService::new()
            .preview_data_file_mutation(
                Some(dir.path()),
                dir.path(),
                &snapshot.path,
                &snapshot.base_source,
                &mutation,
            )
            .unwrap()
    });
    assert_eq!(
        native_trace.phases["yamlParse"].calls,
        trace.phases["yamlParse"].calls + 3
    );
    assert_eq!(preview.candidate_source, native.candidate_source);
    assert_eq!(preview.validation, native.validation);
    assert_eq!(
        session
            .select_source(&snapshot.path)
            .unwrap()
            .data
            .unwrap()
            .base_content_identity,
        snapshot.base_content_identity
    );
    assert_eq!(
        fs::read_to_string(dir.path().join(&snapshot.path)).unwrap(),
        snapshot.base_source
    );
    let request = DataFileQueryRequest {
        relative_path: snapshot.path.clone(),
        base_source: snapshot.base_source,
        mutation,
        query: AuthoringQuery::default(),
    };
    let (query, trace) = measure_read(|| session.query_data_file(&request).unwrap());
    let native = NativeApplicationService::new()
        .query_data_file(Some(dir.path()), dir.path(), &request)
        .unwrap();
    assert_eq!(
        serde_json::to_value(query).unwrap(),
        serde_json::to_value(native).unwrap()
    );
    assert!(!trace.phases.contains_key("discovery"));
    assert!(!trace.phases.contains_key("enumeration"));
}

#[test]
fn cached_schema_identity_cannot_authorize_a_structural_operation() {
    let dir = project();
    let workspace = open(dir.path());
    let context = workspace
        .select_source("sources/a.yaml")
        .unwrap()
        .context
        .unwrap();
    let path = dir.path().join(&context.schema_path);
    let external = context.schema_source + "\n# external edit\n";
    fs::write(&path, &external).unwrap();
    let mut writer = TableAuthoringSession::default();
    let result = writer.apply_intent(
        dir.path(),
        TableOperationInput::Rename {
            table: "item".into(),
            field: "id".into(),
            new_name: "identifier".into(),
        },
        &[TableSourceIdentity {
            path: context.schema_path,
            content_identity: context.schema_content_identity,
        }],
        &[],
    );
    assert_eq!(
        result.unwrap_err().diagnostic().code,
        "E-TABLE-STALE-SOURCE"
    );
    assert_eq!(fs::read_to_string(path).unwrap(), external);
}

#[test]
fn changed_source_ownership_is_returned_with_the_current_view() {
    let dir = project();
    let session = open(dir.path());
    session.select_source("sources/a.yaml").unwrap();
    let source = "kind: type\nname: Rarity\nenum:\n  underlying: int\n  members:\n    - name: Rare\n      value: 1\n";
    fs::write(dir.path().join("sources/a.yaml"), source).unwrap();
    let view = session.select_source("sources/a.yaml").unwrap();
    assert!(view.data.is_none());
    assert!(view.context.is_none());
    assert_eq!(view.type_snapshot.unwrap().name, "Rarity");
    let current = view.current_source.unwrap();
    assert_eq!(current.source, source);
    assert_eq!(current.content_identity, source_content_identity(source));
    let file = view
        .files
        .iter()
        .find(|file| file.path == "sources/a.yaml")
        .unwrap();
    assert_eq!(file.kind, "type");
    assert_eq!(file.type_name.as_deref(), Some("Rarity"));
    assert!(file.table.is_none());
}

#[test]
fn batch_reads_use_the_shared_overlay_without_reopening_the_project() {
    let dir = project();
    let schema_path = dir.path().join("sources/schema.yaml");
    let schema = fs::read_to_string(&schema_path)
        .unwrap()
        .replace("primaryKey:\n  fields: [id]\n", "");
    fs::write(schema_path, schema).unwrap();
    let session = open(dir.path());
    let snapshot = session
        .select_source("sources/a.yaml")
        .unwrap()
        .data
        .unwrap();
    let request = AuthoringBatchRequest {
        targets: vec![BatchTarget {
            record_index: Some(0),
            added_record_index: None,
            field: "id".into(),
        }],
        clipboard_text: "8".into(),
        fill: false,
    };
    let mutation = AuthoringRecordMutation::default();
    let (preview, trace) = measure_read(|| {
        session
            .preview_data_file_batch(&snapshot.path, &snapshot.base_source, &mutation, &request)
            .unwrap()
    });
    let native = NativeApplicationService::new()
        .preview_data_file_batch(
            Some(dir.path()),
            dir.path(),
            &snapshot.path,
            &snapshot.base_source,
            &mutation,
            &request,
        )
        .unwrap();
    assert_eq!(
        serde_json::to_value(preview).unwrap(),
        serde_json::to_value(native).unwrap()
    );
    assert!(!trace.phases.contains_key("discovery"));
    assert!(!trace.phases.contains_key("enumeration"));
    let request = AuthoringBatchCopyRequest {
        targets: request.targets,
        current_mutation: mutation,
    };
    let (copy, trace) = measure_read(|| {
        session
            .copy_data_file_batch(&snapshot.path, &snapshot.base_source, &request)
            .unwrap()
    });
    let native = NativeApplicationService::new()
        .copy_data_file_batch(
            Some(dir.path()),
            dir.path(),
            &snapshot.path,
            &snapshot.base_source,
            &request,
        )
        .unwrap();
    assert_eq!(
        serde_json::to_value(copy).unwrap(),
        serde_json::to_value(native).unwrap()
    );
    assert!(!trace.phases.contains_key("discovery"));
    assert!(!trace.phases.contains_key("enumeration"));
    assert_eq!(
        fs::read_to_string(dir.path().join(snapshot.path)).unwrap(),
        snapshot.base_source
    );
}

#[test]
fn overview_reuses_parsed_sources_and_detects_external_record_changes() {
    let dir = project();
    let session = open(dir.path());
    let request = TableOverviewRequest {
        table: "item".into(),
        profile: None,
        query: AuthoringQuery::default(),
        selected_only: false,
    };
    let (view, trace) = measure_read(|| session.table_overview(&request).unwrap());
    let native = NativeApplicationService::new()
        .table_overview(Some(dir.path()), dir.path(), &request)
        .unwrap();
    assert_eq!(
        serde_json::to_value(view).unwrap(),
        serde_json::to_value(native).unwrap()
    );
    assert!(!trace.phases.contains_key("discovery"));
    assert!(!trace.phases.contains_key("yamlParse"));
    let path = dir.path().join("sources/b.yaml");
    fs::write(&path, "kind: data\ntable: item\nrecords:\n  - id: 9\n").unwrap();
    let (view, trace) = measure_read(|| session.table_overview(&request).unwrap());
    assert_eq!(trace.phases["yamlParse"].calls, 1);
    assert_eq!(
        view.rows
            .iter()
            .find(|row| row.source_path == "sources/b.yaml")
            .unwrap()
            .values[0],
        AuthoringValue::Number { value: "9".into() }
    );
    assert_eq!(
        view.sources
            .iter()
            .find(|source| source.path == "sources/b.yaml")
            .unwrap()
            .content_identity,
        source_content_identity(&fs::read_to_string(path).unwrap())
    );
}

#[cfg(unix)]
#[test]
fn cached_inventory_does_not_follow_a_source_replaced_by_an_escaping_symlink() {
    let dir = project();
    let session = open(dir.path());
    let path = dir.path().join("sources/a.yaml");
    let outside = tempfile::tempdir().unwrap();
    let target = outside.path().join("secret.yaml");
    fs::write(&target, "kind: data\ntable: item\nrecords:\n  - id: 999\n").unwrap();
    fs::remove_file(&path).unwrap();
    std::os::unix::fs::symlink(&target, &path).unwrap();
    assert_eq!(
        session
            .source_content("sources/a.yaml")
            .unwrap_err()
            .diagnostic()
            .code,
        "E-WORKSPACE-SOURCE-SYMLINK"
    );
    assert!(session.select_source("sources/a.yaml").is_err());
    assert!(
        session
            .select_source("sources/b.yaml")
            .unwrap()
            .data
            .unwrap()
            .columns[0]
            .editable
    );
}
