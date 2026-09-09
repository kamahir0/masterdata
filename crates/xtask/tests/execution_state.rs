use std::fs;
use std::path::{Path, PathBuf};

fn repository_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("xtask must live two directories below repository root")
        .to_path_buf()
}

fn field<'a>(content: &'a str, name: &str) -> &'a str {
    let prefix = format!("{name}: ");
    let values = content
        .lines()
        .filter_map(|line| line.strip_prefix(&prefix))
        .collect::<Vec<_>>();
    assert_eq!(values.len(), 1, "expected exactly one `{name}` field");
    values[0].trim()
}

fn section<'a>(content: &'a str, heading: &str) -> &'a str {
    let marker = format!("## {heading}");
    let (_, rest) = content
        .split_once(&marker)
        .unwrap_or_else(|| panic!("missing `{marker}` section"));
    let body = rest
        .split_once("\n## ")
        .map_or(rest, |(body, _)| body)
        .trim();
    assert!(!body.is_empty(), "`{marker}` section must not be empty");
    body
}

fn is_commit_sha(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn assert_file_exists(root: &Path, relative: &str) {
    assert!(
        root.join(relative).is_file(),
        "required development workflow file is missing: {relative}"
    );
}

#[test]
fn development_state_is_well_formed_and_discoverable() {
    let root = repository_root();
    let state_path = root.join("docs/execution-state.md");
    let state = fs::read_to_string(&state_path).expect("read docs/execution-state.md");

    assert_eq!(state.lines().next(), Some("# Development State"));
    let stage = field(&state, "Stage");
    let candidate = field(&state, "Candidate");
    let blocking = section(&state, "Blocking findings");
    let human_decision = section(&state, "Human decision needed");

    match stage {
        "designing" | "implementation-ready" => {
            assert_eq!(candidate, "none");
            assert_eq!(blocking, "None.");
            assert_eq!(human_decision, "None.");
        }
        "verification-ready" | "objective-complete" => {
            assert!(
                is_commit_sha(candidate),
                "{stage} requires an exact candidate SHA"
            );
            assert_eq!(blocking, "None.");
            assert_eq!(human_decision, "None.");
        }
        "correction-ready" => {
            assert!(
                is_commit_sha(candidate),
                "correction-ready requires the reviewed candidate SHA"
            );
            assert_ne!(blocking, "None.");
            assert_eq!(human_decision, "None.");
        }
        "decision-required" => {
            assert!(candidate == "none" || is_commit_sha(candidate));
            assert_eq!(blocking, "None.");
            assert_ne!(human_decision, "None.");
        }
        other => panic!("unknown development stage: {other}"),
    }

    assert_file_exists(&root, "docs/execution-workflow.md");
    let agents = fs::read_to_string(root.join("AGENTS.md")).expect("read AGENTS.md");
    assert!(agents.contains("docs/execution-state.md"));
    assert!(agents.contains("docs/execution-workflow.md"));
}

#[test]
fn development_workflow_keeps_readiness_topology_and_freshness_gates() {
    let root = repository_root();
    let workflow = fs::read_to_string(root.join("docs/execution-workflow.md"))
        .expect("read docs/execution-workflow.md");

    for required in [
        "## Core principle: state describes work, not agent identity",
        "同じHuman-triggered agentがdesign / implementation / verification / correctionを連続して行ってよい",
        "delegationはexecution strategy",
        "## Implementation readiness gate",
        "Humanが実装開始時期を手作業で見抜くことを前提にしない",
        "### `implementation-ready`",
        "### `verification-ready`",
        "### `correction-ready`",
        "## Pre-action freshness gate",
        "## Post-action report verification",
        "current remote HEAD",
        "fresh repositoryと矛盾する旧review結果",
    ] {
        assert!(
            workflow.contains(required),
            "development workflow lost required policy: {required}"
        );
    }

    for forbidden in [
        "role-aware launcher",
        "role-unbound",
        "fixed `main-reviewer`",
        "fixed `implementation-agent`",
    ] {
        assert!(
            !workflow.contains(forbidden),
            "development workflow still encodes fixed agent topology: {forbidden}"
        );
    }
}
