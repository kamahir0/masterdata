use masterdata_desktop::actor::{Intent, Session};
use serde_json::Value;
use std::path::PathBuf;
#[test]
fn rapid_navigation_accepts_only_latest_target_and_warm_selection_reuses_project() {
    let session = Session::default();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/rewrite-oracle/v1/save-both/input");
    session
        .request_blocking(Intent::Open {
            path: root.to_string_lossy().into(),
            discard: false,
        })
        .unwrap();
    let paths = [
        "sources/schema.yaml",
        "sources/data.yaml",
        "sources/inactive.yaml",
        "sources/data.yaml",
    ];
    let mut pending = vec![];
    for token in 1..=50 {
        pending.push(session.submit(Intent::Select {
            path: paths[(token - 1) as usize % 4].into(),
            start: 0,
            count: 32,
            token,
        }));
    }
    let mut latest = None;
    for reply in pending {
        if let Ok(result) = reply.blocking_recv().unwrap() {
            let value: Value = serde_json::from_str(&result).unwrap();
            latest = Some(value);
        }
    }
    let latest = latest.unwrap();
    assert_eq!(latest["data"]["clicked"], paths[49 % 4]);
    assert_eq!(latest["host"]["token"], 50);
    for counter in [
        "projectDiscovery",
        "projectEnumeration",
        "projectYamlParse",
        "projectValidation",
    ] {
        assert_eq!(latest["host"]["work"][counter], 0, "{counter}");
    }
    assert!(latest["data"]["rows"].as_array().unwrap().len() <= 32);
    assert_eq!(
        session
            .request_blocking(Intent::Select {
                path: paths[0].into(),
                start: 0,
                count: 32,
                token: 49
            })
            .unwrap_err()
            .code,
        "E-SELECTION-OBSOLETE"
    );
    session.stop();
}
