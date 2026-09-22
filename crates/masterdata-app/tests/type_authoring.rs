use masterdata_app::*;
use masterdata_core::*;
use serde_json::json;
use std::fs;
fn project() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    initialize_project(
        dir.path(),
        &InitOptions {
            project_id: "test.types".into(),
            name: "Types".into(),
            version: "0.1.0".into(),
        },
    )
    .unwrap();
    for (p, s) in [
        (
            "type.yaml",
            "kind: type\nname: Rarity\nenum:\n  underlying: ulong\n  members:\n    - name: Rare\n      value: 18446744073709551615\n    - name: Common\n      value: 0\n",
        ),
        (
            "custom.yaml",
            "kind: type\nname: Reward\ncustom:\n  fields:\n    - key: 0\n      name: rarity\n      type: Rarity\n",
        ),
        (
            "schema.yaml",
            "kind: schema\ntable: item\nfields:\n  - key: 0\n    name: id\n    type: int\n  - key: 1\n    name: reward\n    type: Reward\nprimaryKey:\n  fields: [id]\n",
        ),
        (
            "data.yaml",
            "kind: data\ntable: item\nrecords:\n  - id: 1\n    reward:\n      rarity: Rare\n",
        ),
    ] {
        fs::write(dir.path().join("sources").join(p), s).unwrap();
    }
    dir
}
fn input(v: serde_json::Value) -> TypeOperationInput {
    serde_json::from_value(v).unwrap()
}
fn rename() -> TypeOperationInput {
    input(json!({"operation":"rename_enum","target":"Rarity","member":"Rare","newName":"Epic"}))
}
#[test]
fn type_plan_is_read_only_and_lossless_and_apply_refreshes_authority() {
    let dir = project();
    let mut session = TableAuthoringSession::default();
    let root = dir.path();
    let snapshot = session.open_type(root, "sources/type.yaml").unwrap();
    assert_eq!(snapshot.category, "Enum");
    assert_eq!(snapshot.members[0].value, "18446744073709551615");
    let plan = session.plan_type(root, rename()).unwrap();
    assert_eq!(plan.affected_occurrence_count, 1);
    let compatibility = plan.compatibility.report.as_ref().expect("compatibility report");
    assert!(
        compatibility
            .changes
            .iter()
            .any(|change| change.generated_api == GeneratedApiImpact::Breaking)
    );
    assert_eq!(plan.files.len(), 2);
    for f in &plan.files {
        assert_eq!(fs::read_to_string(root.join(&f.path)).unwrap(), f.before);
    }
    assert_eq!(
        session.apply(root, &plan.token, false).unwrap().state,
        "success"
    );
    assert_eq!(
        session
            .open_type(root, "sources/type.yaml")
            .unwrap()
            .members[0]
            .name,
        "Epic"
    );
    assert!(!root.join(".masterdata/output").exists());
}
#[test]
fn exact_numeric_and_initializer_transport_reaches_canonical_yaml() {
    let dir = project();
    let root = dir.path();
    let mut session = TableAuthoringSession::default();
    let plan=session.plan_type(root,input(json!({"operation":"add_enum","target":"Rarity","name":"High","value":"18446744073709551614"}))).unwrap();
    assert!(plan.files[0].after.contains("18446744073709551614"));
    let plan=session.plan_type(root,input(json!({"operation":"add_custom","target":"Reward","field":{"key":1,"name":"amount","type":"ulong"},"initializer":"18446744073709551615"}))).unwrap();
    assert!(
        plan.files
            .iter()
            .any(|f| f.after.contains("amount: 18446744073709551615"))
    );
    assert!(session.plan_type(root,input(json!({"operation":"add_enum","target":"Rarity","name":"TooHigh","value":"18446744073709551616"}))).is_err());
}
#[test]
fn type_apply_rejects_stale_config_membership_and_source() {
    for change in 0..3 {
        let dir = project();
        let root = dir.path();
        let mut session = TableAuthoringSession::default();
        let plan = session.plan_type(root, rename()).unwrap();
        let path = match change {
            0 => "masterdata.toml",
            1 => "sources/new.yaml",
            _ => "sources/type.yaml",
        };
        if change == 1 {
            fs::write(root.join(path), "kind: data\ntable: item\nrecords: []\n").unwrap();
        } else {
            let text = fs::read_to_string(root.join(path)).unwrap();
            fs::write(root.join(path), text + "# external\n").unwrap();
        }
        let data = fs::read(root.join("sources/data.yaml")).unwrap();
        let result = session.apply(root, &plan.token, false).unwrap();
        assert_eq!(result.state, "not_started");
        assert!(result.diagnostic.unwrap().message.contains("stale"));
        assert_eq!(fs::read(root.join("sources/data.yaml")).unwrap(), data);
    }
}
#[test]
fn unused_drop_requires_machine_authorization() {
    let dir = project();
    let root = dir.path();
    let mut session = TableAuthoringSession::default();
    let plan = session
        .plan_type(
            root,
            input(json!({"operation":"drop_enum","target":"Rarity","member":"Common"})),
        )
        .unwrap();
    assert!(plan.destructive);
    assert_eq!(
        session.apply(root, &plan.token, false).unwrap().state,
        "not_started"
    );
    assert_eq!(
        session.apply(root, &plan.token, true).unwrap().state,
        "success"
    );
}
#[test]
fn type_transactions_share_rollback_and_cross_surface_recovery_gate() {
    for recovery in [false, true] {
        let dir = project();
        let root = dir.path();
        let mut session = TableAuthoringSession::default();
        let plan = session.plan_type(root, rename()).unwrap();
        let mut faults = vec![MigrationCommitFailureInjection::write_file(1)];
        if recovery {
            faults.push(MigrationCommitFailureInjection::rollback_file(0));
        }
        let result = session
            .apply_with_failures(root, &plan.token, false, &faults)
            .unwrap();
        if recovery {
            assert_eq!(result.state, "recovery_required");
            assert!(session.plan_type(root, rename()).is_err());
            assert!(session.plan(root,serde_json::from_value(json!({"operation":"rename","table":"item","field":"id","newName":"itemId"})).unwrap()).is_err());
            for f in &plan.files {
                fs::write(root.join(&f.path), &f.before).unwrap();
            }
            assert!(session.recheck(root).unwrap().is_none());
        } else {
            assert_eq!(result.state, "rolled_back");
            for f in &plan.files {
                assert_eq!(fs::read_to_string(root.join(&f.path)).unwrap(), f.before);
            }
        }
    }
}

#[test]
fn overflowing_integer_never_coerces_to_double_and_negative_zero_remains_integer() {
    let dir = project();
    let root = dir.path();
    let mut session = TableAuthoringSession::default();
    let command = |ty: &str, value: &str| {
        input(
            json!({"operation":"add_custom","target":"Reward","field":{"key":1,"name":"amount","type":ty},"initializer":value}),
        )
    };
    assert!(
        session
            .plan_type(root, command("double", "18446744073709551616"))
            .is_err()
    );
    let plan = session.plan_type(root, command("long", "-0")).unwrap();
    assert!(plan.files.iter().any(|f| f.after.contains("amount: 0")));
}

#[test]
fn complex_initializer_preserves_nested_scalar_and_rejects_duplicate_keys() {
    let dir = project();
    let root = dir.path();
    let mut session = TableAuthoringSession::default();
    fs::write(root.join("sources/constant.yaml"),"kind: type\nname: Constant\ncustom:\n  fields:\n    - key: 0\n      name: amount\n      type: ulong\n").unwrap();
    let command = |value: &str| {
        input(
            json!({"operation":"add_custom","target":"Reward","field":{"key":1,"name":"constant","type":"Constant"},"initializer":value}),
        )
    };
    let plan = session
        .plan_type(root, command("{\"amount\":18446744073709551615}"))
        .unwrap();
    assert!(
        plan.files
            .iter()
            .any(|f| f.after.contains("amount: 18446744073709551615"))
    );
    assert!(
        session
            .plan_type(root, command("{\"amount\":18446744073709551616}"))
            .is_err()
    );
    assert!(
        session
            .plan_type(root, command("{\"amount\":1,\"amount\":2}"))
            .is_err()
    );
}
