use masterdata_desktop::{
    actor::{Intent, Session},
    delivery::Request,
};
use serde_json::Value;
use std::{
    fs,
    path::PathBuf,
    time::{Duration, Instant},
};

fn project() -> tempfile::TempDir {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/rewrite-oracle/v1/consumer/minimal");
    let temp = tempfile::tempdir().unwrap();
    fs::copy(
        root.join("masterdata.toml"),
        temp.path().join("masterdata.toml"),
    )
    .unwrap();
    fs::create_dir(temp.path().join("sources")).unwrap();
    for entry in fs::read_dir(root.join("sources")).unwrap() {
        let entry = entry.unwrap();
        fs::copy(
            entry.path(),
            temp.path().join("sources").join(entry.file_name()),
        )
        .unwrap();
    }
    temp
}
fn call(session: &Session, intent: Intent) -> Value {
    serde_json::from_str(&session.request_blocking(intent).unwrap()).unwrap()
}
fn complete(session: &Session) -> Value {
    let deadline = Instant::now() + Duration::from_secs(180);
    loop {
        let state = call(session, Intent::DeliveryState);
        if state["data"]["running"] == false {
            return state["data"].clone();
        }
        assert!(
            Instant::now() < deadline,
            "delivery job did not settle: {state}"
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}
fn open(session: &Session, temp: &tempfile::TempDir) {
    call(
        session,
        Intent::Open {
            path: temp.path().to_string_lossy().into(),
            discard: false,
        },
    );
}
fn start(session: &Session, request: Request) -> Value {
    call(session, Intent::DeliveryStart { request })["data"].clone()
}

#[test]
fn failed_build_keeps_captured_diagnostics_through_other_delivery_jobs_and_rejects_a_new_project() {
    let temp = project();
    let file = temp.path().join("sources/data.yaml");
    fs::write(
        &file,
        fs::read_to_string(&file)
            .unwrap()
            .replace("direct: 3002", "direct: broken"),
    )
    .unwrap();
    let session = Session::default();
    open(&session, &temp);
    start(
        &session,
        Request::Build {
            profile: None,
            dry_run: false,
        },
    );
    let failed = complete(&session);
    assert_eq!(failed["status"], "failed");
    assert_eq!(failed["error"]["code"], "E-BUILD-INVALID");
    assert_eq!(
        failed["lastBuild"]["captured"]["project"]["id"],
        "fixture.minimal"
    );
    let id = failed["id"].as_u64().unwrap();
    let diagnostics = failed["error"]["diagnostics"].as_array().unwrap();
    assert!(!diagnostics.is_empty());
    let index = diagnostics
        .iter()
        .position(|d| d["source"] == "sources/data.yaml" && d["occurrence"] == 2)
        .unwrap();
    let selected = call(
        &session,
        Intent::Select {
            path: "sources/data.yaml".into(),
            start: 0,
            count: 32,
            token: 1,
        },
    );
    let original = selected["data"]["rows"][1]["id"].clone();
    start(&session, Request::PublishPreview);
    let publish = complete(&session);
    assert_eq!(publish["status"], "failed");
    assert_eq!(publish["lastBuild"], failed["lastBuild"]);
    assert_eq!(publish["result"]["preflight"], "unavailable");
    assert!(!temp.path().join(".masterdata/output").exists());
    let page = call(
        &session,
        Intent::DeliveryProblems {
            id,
            start: 0,
            count: 1,
        },
    );
    assert_eq!(page["data"]["diagnostics"].as_array().unwrap().len(), 1);
    let target = call(&session, Intent::DeliveryProblemTarget { id, index });
    assert_eq!(target["data"]["target"]["row"], original);
    start(
        &session,
        Request::Build {
            profile: Some("gone".into()),
            dry_run: false,
        },
    );
    let missing = complete(&session);
    assert_eq!(missing["status"], "failed");
    assert_eq!(missing["profile"], "gone");
    assert_eq!(missing["lastBuild"]["profile"], "gone");
    assert_eq!(
        session
            .request_blocking(Intent::DeliveryProblems {
                id,
                start: 0,
                count: 1
            })
            .unwrap_err()
            .code,
        "E-PROJECT-OBSOLETE"
    );
    call(
        &session,
        Intent::Open {
            path: temp.path().to_string_lossy().into(),
            discard: false,
        },
    );
    assert!(call(&session, Intent::DeliveryState)["data"]["lastBuild"].is_null());
    session.stop();
}

#[cfg(feature = "native-consumer")]
#[test]
fn native_build_keeps_navigation_and_drafts_live_then_publishes_only_a_fresh_confirmed_receipt() {
    use masterdata_engine::{
        delivery::SavedConfig,
        native::{artifact::ArtifactSet, dotnet},
        source::Value as SourceValue,
    };
    let temp = project();
    let session = Session::default();
    open(&session, &temp);
    let selected = call(
        &session,
        Intent::Select {
            path: "sources/data.yaml".into(),
            start: 0,
            count: 32,
            token: 1,
        },
    );
    let row = selected["data"]["rows"][0]["id"]
        .as_str()
        .unwrap()
        .to_owned();
    call(
        &session,
        Intent::EditValue {
            source: "sources/data.yaml".into(),
            revision: selected["data"]["revision"].as_u64().unwrap(),
            generation: selected["data"]["generation"].as_u64().unwrap(),
            row: row.clone(),
            path: vec!["direct".into()],
            value: SourceValue::Literal("7777".into()),
        },
    );
    let started = start(
        &session,
        Request::Build {
            profile: None,
            dry_run: false,
        },
    );
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let state = call(&session, Intent::DeliveryState);
        if state["data"]["phase"] == "building" {
            break;
        }
        assert!(
            Instant::now() < deadline,
            "Build never accepted immutable Plan"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(
        session
            .request_blocking(Intent::DeliveryStart {
                request: Request::Build {
                    profile: None,
                    dry_run: false
                }
            })
            .unwrap_err()
            .code,
        "E-DELIVERY-BUSY"
    );
    assert_eq!(
        session
            .request_blocking(Intent::Open {
                path: temp.path().to_string_lossy().into(),
                discard: true
            })
            .unwrap_err()
            .code,
        "E-DELIVERY-BUSY"
    );
    for token in 2..12 {
        let selected = call(
            &session,
            Intent::Select {
                path: "sources/data.yaml".into(),
                start: 0,
                count: 32,
                token,
            },
        );
        for counter in [
            "projectDiscovery",
            "projectEnumeration",
            "projectYamlParse",
            "projectValidation",
        ] {
            assert_eq!(selected["host"]["work"][counter], 0);
        }
        assert_eq!(selected["data"]["rows"][0]["id"], row);
        assert_eq!(selected["data"]["rows"][0]["cells"][1]["display"], "7777");
    }
    let saved = call(
        &session,
        Intent::SaveSource {
            source: "sources/data.yaml".into(),
        },
    );
    assert_eq!(saved["data"][0]["outcome"], "Success");
    let built = complete(&session);
    assert_eq!(built["status"], "succeeded");
    assert_eq!(built["id"], started["id"]);
    let actual = ArtifactSet::load(&SavedConfig::load(temp.path()).unwrap()).unwrap();
    let consumer = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/rewrite-oracle/v1/consumer/minimal/Consumer.cs");
    let csharp = actual
        .csharp
        .iter()
        .map(|(p, b)| (p.clone(), String::from_utf8(b.clone()).unwrap()))
        .collect();
    assert!(
        dotnet::verify_consumer(&csharp, &actual.binary, &consumer, "")
            .unwrap()
            .contains("PASS "),
        "Build mixed the later source Save into its captured input"
    );
    // Publish-only may inspect receipt/config, never current invalid YAML.
    fs::write(temp.path().join("sources/data.yaml"), "kind: [\n").unwrap();
    let config = temp.path().join("masterdata.toml");
    let original = fs::read_to_string(&config).unwrap();
    fs::write(
        &config,
        format!("{original}\n[[publish.targets]]\nkind = \"binary\"\npath = \"delivery/masterdata.bytes\"\n"),
    )
    .unwrap();
    start(&session, Request::PublishPreview);
    let preview = complete(&session);
    let token = preview["preview"]["token"].as_str().unwrap().to_owned();
    assert_eq!(preview["status"], "succeeded");
    assert_eq!(preview["lastBuild"], built["lastBuild"]);
    assert!(!temp.path().join("delivery").exists());
    fs::write(
        &config,
        format!(
            "{}\n# external target/config change\n",
            fs::read_to_string(&config).unwrap()
        ),
    )
    .unwrap();
    start(&session, Request::ConfirmPublish { token });
    let stale = complete(&session);
    assert_eq!(stale["status"], "failed");
    assert!(!temp.path().join("delivery").exists());
    start(&session, Request::PublishPreview);
    complete(&session);
    start(&session, Request::CancelPreview);
    let cancelled = complete(&session);
    assert_eq!(cancelled["result"]["outcome"], "NotAttempted");
    assert!(temp.path().join(".masterdata/output").exists());
    assert!(!temp.path().join("delivery").exists());
    start(&session, Request::PublishPreview);
    let preview = complete(&session);
    start(
        &session,
        Request::ConfirmPublish {
            token: preview["preview"]["token"].as_str().unwrap().into(),
        },
    );
    let result = complete(&session);
    assert_eq!(result["result"]["outcome"], "Success");
    assert_eq!(result["result"]["unityVerification"], "not_observed");
    assert_eq!(
        fs::read(temp.path().join("delivery/masterdata.bytes")).unwrap(),
        actual.binary
    );
    assert_eq!(result["lastBuild"], built["lastBuild"]);
    session.stop();
}
