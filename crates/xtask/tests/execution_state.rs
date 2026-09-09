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
        "required execution workflow file is missing: {relative}"
    );
}

#[test]
fn execution_state_is_well_formed_and_discoverable() {
    let root = repository_root();
    let state_path = root.join("docs/execution-state.md");
    let state = fs::read_to_string(&state_path).expect("read docs/execution-state.md");

    assert_eq!(state.lines().next(), Some("# Execution State"));
    let phase = field(&state, "Phase");
    let candidate = field(&state, "Candidate");
    let blocking = section(&state, "Blocking findings");
    let human_decision = section(&state, "Human decision needed");

    match phase {
        "implementation-required" => {
            assert_eq!(candidate, "none");
            assert_eq!(blocking, "None.");
            assert_eq!(human_decision, "None.");
        }
        "review-required" | "objective-complete" => {
            assert!(
                is_commit_sha(candidate),
                "{phase} requires an exact candidate SHA"
            );
            assert_eq!(blocking, "None.");
            assert_eq!(human_decision, "None.");
        }
        "corrective-required" => {
            assert!(
                is_commit_sha(candidate),
                "corrective-required requires the reviewed candidate SHA"
            );
            assert_ne!(blocking, "None.");
            assert_eq!(human_decision, "None.");
        }
        "human-decision-required" => {
            assert!(candidate == "none" || is_commit_sha(candidate));
            assert_eq!(blocking, "None.");
            assert_ne!(human_decision, "None.");
        }
        other => panic!("unknown execution phase: {other}"),
    }

    assert_file_exists(&root, "docs/execution-workflow.md");
    let agents = fs::read_to_string(root.join("AGENTS.md")).expect("read AGENTS.md");
    assert!(agents.contains("docs/execution-state.md"));
    assert!(agents.contains("docs/execution-workflow.md"));
}

#[test]
fn execution_workflow_keeps_role_and_report_safety_gates() {
    let root = repository_root();
    let workflow = fs::read_to_string(root.join("docs/execution-workflow.md"))
        .expect("read docs/execution-workflow.md");

    for required in [
        "## Session role binding",
        "`role-unbound`",
        "同一session内でroleを自動的に切り替えてはならない（MUST NOT）",
        "## Pre-action actor / freshness gate",
        "bound roleとcurrent phaseの次actorが一致しない場合",
        "## Post-action report verification",
        "current remote HEAD",
        "fresh repositoryと矛盾する旧review結果",
    ] {
        assert!(
            workflow.contains(required),
            "execution workflow lost required safety policy: {required}"
        );
    }
}
