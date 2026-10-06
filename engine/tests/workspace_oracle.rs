use masterdata_engine::{instrument, native::Outcome, source::Value, workspace::Workspace};
use serde_json::Value as Json;
use std::{
    fs,
    path::{Path, PathBuf},
};
fn copy(src: &Path, dst: &Path) {
    fs::create_dir_all(dst).unwrap();
    for e in fs::read_dir(src).unwrap() {
        let e = e.unwrap();
        if e.file_type().unwrap().is_dir() {
            copy(&e.path(), &dst.join(e.file_name()));
        } else {
            fs::copy(e.path(), dst.join(e.file_name())).unwrap();
        }
    }
}
fn oracle() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fixtures/rewrite-oracle/v1")
}
#[test]
fn first_source_reuses_project_syntax_only_after_fresh_actual_byte_comparison() {
    let temp = tempfile::tempdir().unwrap();
    copy(&oracle().join("save-both/input"), temp.path());
    let mut w = Workspace::open(temp.path()).unwrap();
    let (_, measurement) = instrument::measure(|| w.select("sources/data.yaml", 0, 32).unwrap());
    assert_eq!(measurement.work.local_parse, 0);
    assert_eq!(measurement.work.project_yaml_parse, 0);
    let mut fresh = Workspace::open(temp.path()).unwrap();
    let file = temp.path().join("sources/data.yaml");
    let external = fs::read_to_string(&file)
        .unwrap()
        .replace("'old'", "'actual before first selection'");
    fs::write(file, &external).unwrap();
    let (_, measurement) =
        instrument::measure(|| fresh.select("sources/data.yaml", 0, 32).unwrap());
    assert_eq!(
        fresh
            .current_doc("sources/data.yaml")
            .unwrap()
            .bytes
            .as_ref(),
        external
    );
    assert_eq!(measurement.work.local_parse, 1);
}
fn compare(expected: &Path, actual: &Path) {
    for e in fs::read_dir(expected).unwrap() {
        let e = e.unwrap();
        let other = actual.join(e.file_name());
        if e.file_type().unwrap().is_dir() {
            compare(&e.path(), &other);
        } else {
            assert_eq!(
                fs::read(e.path()).unwrap(),
                fs::read(&other).unwrap(),
                "exact disk bytes {}",
                other.display()
            );
        }
    }
}
#[test]
fn a_clean_external_schema_change_cannot_reuse_the_old_revision_for_header_authoring() {
    for reorder in [false, true] {
        let temp = tempfile::tempdir().unwrap();
        copy(&oracle().join("save-both/input"), temp.path());
        let mut w = Workspace::open(temp.path()).unwrap();
        let p = w.select("sources/data.yaml", 0, 32).unwrap();
        w.edit_text(
            "sources/data.yaml",
            p.revision,
            &p.rows[0].id,
            "note",
            "keep this draft",
        )
        .unwrap();
        let observed = w.select("sources/schema.yaml", 0, 32).unwrap();
        let data = w.current_doc("sources/data.yaml").unwrap().bytes.clone();
        let file = temp.path().join("sources/schema.yaml");
        let external = format!(
            "{}\n# External authoring\n",
            fs::read_to_string(&file).unwrap()
        );
        fs::write(&file, &external).unwrap();
        let (result, measurement) = instrument::measure(|| {
            if reorder {
                let mut order = observed
                    .columns
                    .iter()
                    .map(|c| c.field.name.clone())
                    .collect::<Vec<_>>();
                order.reverse();
                w.reorder_columns_at(
                    "sources/schema.yaml",
                    observed.schema_revision,
                    observed.generation,
                    &order,
                )
            } else {
                w.schema_modifier_at(
                    "sources/schema.yaml",
                    observed.schema_revision,
                    observed.generation,
                    "note",
                    masterdata_engine::workspace::FieldShapeEdit {
                        nullable: false,
                        array: false,
                        type_name: None,
                    },
                )
            }
        });
        assert_eq!(result.unwrap_err().code, "E-DRAFT-STALE");
        assert_eq!(
            w.drafts["sources/schema.yaml"].revision,
            observed.schema_revision
        );
        assert_eq!(fs::read_to_string(file).unwrap(), external);
        assert_eq!(
            w.current_doc("sources/schema.yaml").unwrap().bytes.as_ref(),
            external
        );
        assert!(!w.drafts["sources/schema.yaml"].dirty());
        assert!(!w.drafts["sources/schema.yaml"].can_undo());
        assert_eq!(w.current_doc("sources/data.yaml").unwrap().bytes, data);
        assert!(w.drafts["sources/data.yaml"].dirty());
        assert!(w.drafts["sources/data.yaml"].can_undo());
        assert_eq!(measurement.work.project_discovery, 0);
        assert_eq!(measurement.work.project_enumeration, 0);
        assert_eq!(measurement.work.project_yaml_parse, 0);
        assert_eq!(measurement.work.project_validation, 0);
    }
}
#[test]
fn independent_table_save_scope_and_fresh_conflicts() {
    let root = oracle();
    let manifest: Json =
        serde_json::from_slice(&fs::read(root.join("manifest.json")).unwrap()).unwrap();
    for id in manifest["saveScenarios"].as_array().unwrap() {
        let id = id.as_str().unwrap();
        let case = root.join(id);
        let temp = tempfile::tempdir().unwrap();
        copy(&case.join("input"), temp.path());
        let scenario: Json =
            serde_json::from_slice(&fs::read(case.join("scenario.json")).unwrap()).unwrap();
        let op = &scenario["operation"];
        let schema = op["schemaSource"].as_str().unwrap();
        let selected = op["selectedRecordSource"].as_str();
        let mut w = Workspace::open(temp.path()).unwrap();
        let view = w.select(selected.unwrap_or(schema), 0, 64).unwrap();
        let table = view.table.name.clone();
        if !op["schemaDraft"].is_null() {
            w.schema_modifier(
                schema,
                w.drafts[schema].revision,
                op["schemaDraft"]["field"].as_str().unwrap(),
                op["schemaDraft"]["nullable"].as_bool().unwrap(),
                false,
                None,
            )
            .unwrap();
        }
        if !op["recordDraft"].is_null() {
            let path = selected.unwrap();
            let d = &w.drafts[path];
            let row =
                d.row_ids[op["recordDraft"]["occurrence"].as_u64().unwrap() as usize - 1].clone();
            w.edit(
                path,
                d.revision,
                &row,
                &[op["recordDraft"]["field"].as_str().unwrap().into()],
                &Value::Text(op["recordDraft"]["value"]["text"].as_str().unwrap().into()),
            )
            .unwrap();
        }
        if let Some(change) = op["externalChangeAfterBaseCapture"].as_object() {
            fs::write(
                temp.path().join(change["source"].as_str().unwrap()),
                change["bytes"].as_str().unwrap(),
            )
            .unwrap();
        }
        let results = w.save_table(&table, selected).unwrap();
        let mut committed = results
            .iter()
            .filter(|r| r.outcome == Outcome::Success)
            .map(|r| r.source.clone())
            .collect::<Vec<_>>();
        committed.sort();
        let expected = scenario["expected"]["committedPhysicalSources"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s.as_str().unwrap().to_owned())
            .collect::<Vec<_>>();
        assert_eq!(committed, expected, "{id}");
        if scenario["expected"]["outcome"] == "conflict" {
            assert!(results.iter().any(|r| r.outcome == Outcome::Conflict));
            assert!(!w.dirty_paths().is_empty());
        }
        compare(&case.join("expected"), temp.path());
        println!("PASS {id}");
    }
}
#[test]
fn source_history_survives_navigation_and_success_clears_only_saved_file() {
    let temp = tempfile::tempdir().unwrap();
    copy(&oracle().join("save-both/input"), temp.path());
    let mut w = Workspace::open(temp.path()).unwrap();
    let a = w.select("sources/data.yaml", 0, 32).unwrap();
    let row = a.rows[0].id.clone();
    w.edit_text("sources/data.yaml", a.revision, &row, "note", "draft")
        .unwrap();
    let b = w.select("sources/inactive.yaml", 0, 32).unwrap();
    w.edit_text(
        "sources/inactive.yaml",
        b.revision,
        &b.rows[0].id,
        "note",
        "other draft",
    )
    .unwrap();
    let before = fs::read(temp.path().join("sources/data.yaml")).unwrap();
    let (a, measure) = instrument::measure(|| w.select("sources/data.yaml", 0, 32).unwrap());
    assert!(a.rows[0].cells.iter().any(|c| c.display == "draft"));
    assert_eq!(measure.work.project_discovery, 0);
    assert_eq!(measure.work.project_enumeration, 0);
    assert_eq!(measure.work.project_yaml_parse, 0);
    assert_eq!(measure.work.project_validation, 0);
    w.undo("sources/data.yaml", false).unwrap();
    assert!(!w.drafts["sources/data.yaml"].dirty());
    assert!(w.drafts["sources/data.yaml"].can_redo());
    w.undo("sources/data.yaml", true).unwrap();
    assert_eq!(
        fs::read(temp.path().join("sources/data.yaml")).unwrap(),
        before,
        "Undo does not write disk"
    );
    w.save_table("item", Some("sources/data.yaml")).unwrap();
    assert!(!w.drafts["sources/data.yaml"].can_undo());
    assert!(w.drafts["sources/inactive.yaml"].dirty());
    assert!(w.drafts["sources/inactive.yaml"].can_undo());
}
#[test]
fn clean_external_refresh_and_invalid_or_deleted_source_never_returns_stale_editor() {
    let temp = tempfile::tempdir().unwrap();
    copy(&oracle().join("save-record/input"), temp.path());
    let mut w = Workspace::open(temp.path()).unwrap();
    w.select("sources/data.yaml", 0, 32).unwrap();
    let path = temp.path().join("sources/data.yaml");
    fs::write(
        &path,
        "kind: data\ntable: item\nrecords:\n  - id: 9\n    note: external\n",
    )
    .unwrap();
    assert_eq!(
        w.select("sources/data.yaml", 0, 32).unwrap().rows[0].cells[0].display,
        "9"
    );
    fs::write(&path, "records: [").unwrap();
    assert!(w.select("sources/data.yaml", 0, 32).is_err());
    fs::remove_file(path).unwrap();
    assert!(w.select("sources/data.yaml", 0, 32).is_err());
}
#[test]
fn schema_reinterpretation_keeps_record_bytes_and_rejects_old_diagnostics() {
    let temp = tempfile::tempdir().unwrap();
    copy(&oracle().join("save-record/input"), temp.path());
    let mut w = Workspace::open(temp.path()).unwrap();
    w.select("sources/data.yaml", 0, 32).unwrap();
    let bytes = w.current_doc("sources/data.yaml").unwrap().bytes.clone();
    let old = w.generation;
    w.schema_modifier("sources/schema.yaml", 0, "note", false, false, Some("int"))
        .unwrap();
    let v = w.select("sources/data.yaml", 0, 32).unwrap();
    assert_eq!(w.current_doc("sources/data.yaml").unwrap().bytes, bytes);
    assert!(!v.rows[0].cells[1].valid);
    assert!(!w.drafts["sources/data.yaml"].dirty());
    assert!(!w.accept_diagnostics(old, vec![]));
    w.undo("sources/schema.yaml", false).unwrap();
    assert!(w.select("sources/data.yaml", 0, 32).unwrap().rows[0].cells[1].valid);
}

#[test]
fn overwrite_rechecks_reviewed_file_identity_and_keeps_the_draft_on_conflict() {
    let temp = tempfile::tempdir().unwrap();
    copy(&oracle().join("save-record/input"), temp.path());
    let mut w = Workspace::open(temp.path()).unwrap();
    let v = w.select("sources/data.yaml", 0, 32).unwrap();
    w.edit_text(
        "sources/data.yaml",
        v.revision,
        &v.rows[0].id,
        "note",
        "draft",
    )
    .unwrap();
    let path = temp.path().join("sources/data.yaml");
    let external = "kind: data\ntable: item\nrecords:\n  - id: 7\n    note: external\n";
    fs::write(&path, external).unwrap();
    assert_eq!(
        w.save_table("item", Some("sources/data.yaml")).unwrap()[0].outcome,
        Outcome::Conflict
    );
    let reviewed = w.compare("sources/data.yaml").unwrap().identity;
    // Identical bytes in a replaced file still have a different physical identity.
    let replacement = tempfile::NamedTempFile::new_in(path.parent().unwrap()).unwrap();
    fs::write(replacement.path(), external).unwrap();
    replacement.persist(&path).unwrap();
    assert_eq!(
        w.overwrite("sources/data.yaml", &reviewed).unwrap().outcome,
        Outcome::Conflict
    );
    assert!(w.drafts["sources/data.yaml"].dirty());
    assert!(w.drafts["sources/data.yaml"].can_undo());
    assert_eq!(fs::read_to_string(&path).unwrap(), external);
    let comparison = w.compare("sources/data.yaml").unwrap();
    assert_eq!(
        w.overwrite("sources/data.yaml", &comparison.identity)
            .unwrap()
            .outcome,
        Outcome::Success
    );
    assert_eq!(fs::read_to_string(path).unwrap(), comparison.after);
}

#[test]
fn comparison_preserves_editing_base_and_fresh_external_scope_without_mutation() {
    let temp = tempfile::tempdir().unwrap();
    copy(&oracle().join("save-both/input"), temp.path());
    let source = "sources/data.yaml";
    let schema = "sources/schema.yaml";
    let path = temp.path().join(source);
    let base = fs::read_to_string(&path).unwrap();
    let schema_base = fs::read_to_string(temp.path().join(schema)).unwrap();
    let mut w = Workspace::open(temp.path()).unwrap();
    let p = w.select(source, 0, 32).unwrap();
    w.edit_text(source, p.revision, &p.rows[0].id, "note", "draft")
        .unwrap();
    let candidate = base.replace("'old'", "'draft'");
    assert_ne!(candidate, base);
    let ordinary = w.compare(source).unwrap();
    assert!(!ordinary.conflict);
    assert_eq!(ordinary.base, base);
    assert_eq!(ordinary.before, base);
    assert_eq!(ordinary.after, candidate);

    // No watcher or selection refresh has observed this external change. A
    // comparison must still capture the real bytes and isolate its own scope.
    let external = "kind: [unfinished\n# external source\n";
    fs::write(&path, external).unwrap();
    let changed = w.compare(source).unwrap();
    assert!(changed.conflict);
    assert_eq!(changed.base, base);
    assert_eq!(changed.before, external);
    assert_eq!(changed.after, candidate);
    let other = w.compare(schema).unwrap();
    assert!(
        !other.conflict,
        "another physical source inherited Conflict"
    );
    assert_eq!(other.base, schema_base);
    assert_eq!(other.before, schema_base);
    assert_eq!(other.after, schema_base);
    assert_eq!(fs::read_to_string(path).unwrap(), external);
    assert_eq!(
        fs::read_to_string(temp.path().join(schema)).unwrap(),
        schema_base
    );
    assert_eq!(w.dirty_paths(), vec![source]);
    assert!(w.drafts[source].can_undo());
}

#[test]
fn read_only_write_failure_keeps_exact_disk_bytes_and_local_history() {
    let temp = tempfile::tempdir().unwrap();
    copy(&oracle().join("save-record/input"), temp.path());
    let path = temp.path().join("sources/data.yaml");
    let before = fs::read(&path).unwrap();
    let mut w = Workspace::open(temp.path()).unwrap();
    let v = w.select("sources/data.yaml", 0, 32).unwrap();
    w.edit_text(
        "sources/data.yaml",
        v.revision,
        &v.rows[0].id,
        "note",
        "draft",
    )
    .unwrap();
    let original_permissions = fs::metadata(&path).unwrap().permissions();
    let mut permissions = original_permissions.clone();
    permissions.set_readonly(true);
    fs::set_permissions(&path, permissions).unwrap();
    let results = w.save_table("item", Some("sources/data.yaml")).unwrap();
    fs::set_permissions(&path, original_permissions).unwrap();
    assert_eq!(results[0].outcome, Outcome::Failure);
    assert_eq!(fs::read(path).unwrap(), before);
    assert!(w.drafts["sources/data.yaml"].dirty());
    assert!(w.drafts["sources/data.yaml"].can_undo());
}

#[test]
fn unavailable_external_sources_invalidate_diagnostics_without_discarding_authoring() {
    for mutation in ["invalid", "delete", "binding", "schema"] {
        let temp = tempfile::tempdir().unwrap();
        copy(&oracle().join("save-both/input"), temp.path());
        let mut w = Workspace::open(temp.path()).unwrap();
        let p = w.select("sources/data.yaml", 0, 32).unwrap();
        let base = w.drafts["sources/data.yaml"].document.bytes.clone();
        w.edit_text(
            "sources/data.yaml",
            p.revision,
            &p.rows[0].id,
            "note",
            "unfinished",
        )
        .unwrap();
        let draft = w.drafts["sources/data.yaml"].document.bytes.clone();
        let old = w.validation_snapshot();
        let old_problems = old.validate(None).unwrap().0;
        w.accept_diagnostics(w.generation, old_problems.clone());
        let generation = w.generation;
        let path = if mutation == "schema" {
            "sources/schema.yaml"
        } else {
            "sources/data.yaml"
        };
        let file = temp.path().join(path);
        let bytes = fs::read(&file).unwrap();
        match mutation {
            "delete" => fs::remove_file(&file).unwrap(),
            "binding" => fs::write(&file, "kind: data\ntable: elsewhere\nrecords: []\n").unwrap(),
            _ => fs::write(&file, "kind: [\n").unwrap(),
        }
        assert!(w.select("sources/data.yaml", 0, 32).is_err());
        assert!(w.generation > generation);
        assert!(!w.accept_diagnostics(generation, old_problems));
        let failed_generation = w.generation;
        assert!(w.select("sources/data.yaml", 0, 32).is_err());
        assert_eq!(
            w.generation, failed_generation,
            "unchanged failure cannot restart diagnostics indefinitely"
        );
        let input = w.validation_snapshot();
        let problems = input.validate(None).unwrap().0;
        assert!(w.accept_diagnostics(w.generation, problems));
        assert!(
            w.diagnostics
                .iter()
                .any(|d| d.source == path && d.generation == w.generation)
        );
        assert_eq!(w.drafts["sources/data.yaml"].document.bytes, draft);
        assert!(w.drafts["sources/data.yaml"].can_undo());
        assert_eq!(w.dirty_paths(), ["sources/data.yaml"]);
        if path.ends_with("data.yaml") {
            assert!(
                w.validation_snapshot().sources[path].document.is_none(),
                "old overlay cannot hide unavailable current source"
            );
            assert_eq!(w.drafts[path].outcome, Some(Outcome::Conflict));
        }
        fs::write(&file, bytes).unwrap();
        w.refresh_source(path).unwrap();
        let current = w.select("sources/data.yaml", 0, 32).unwrap();
        assert_eq!(current.rows[0].id, p.rows[0].id);
        assert_eq!(w.drafts["sources/data.yaml"].document.bytes, draft);
        w.undo("sources/data.yaml", false).unwrap();
        assert_eq!(w.drafts["sources/data.yaml"].document.bytes, base);
    }
}

#[test]
fn unrelated_source_generations_preserve_observed_authoring_but_required_context_changes_reject_it()
{
    let temp = tempfile::tempdir().unwrap();
    copy(&oracle().join("save-both/input"), temp.path());
    let mut w = Workspace::open(temp.path()).unwrap();
    let p = w.select("sources/data.yaml", 0, 32).unwrap();
    let file = w.read.root.join("sources/inactive.yaml");
    fs::write(
        &file,
        format!(
            "{}\n# changed inactive source\n",
            fs::read_to_string(&file).unwrap()
        ),
    )
    .unwrap();
    w.refresh_paths(&[file]);
    assert!(w.generation > p.generation);
    w.edit_text_at(
        "sources/data.yaml",
        p.revision,
        p.generation,
        &p.rows[0].id,
        "note",
        "observed input survives",
    )
    .unwrap();
    let after = w.select("sources/data.yaml", 0, 32).unwrap();
    let draft = w.drafts["sources/data.yaml"].document.bytes.clone();
    let schema = w.read.root.join("sources/schema.yaml");
    fs::write(
        &schema,
        format!(
            "{}\n# changed required schema\n",
            fs::read_to_string(&schema).unwrap()
        ),
    )
    .unwrap();
    w.refresh_paths(&[schema]);
    assert_eq!(
        w.edit_text_at(
            "sources/data.yaml",
            after.revision,
            after.generation,
            &after.rows[0].id,
            "note",
            "stale"
        )
        .unwrap_err()
        .code,
        "E-DRAFT-STALE"
    );
    assert_eq!(w.drafts["sources/data.yaml"].document.bytes, draft);
    assert!(w.drafts["sources/data.yaml"].can_undo());
}
