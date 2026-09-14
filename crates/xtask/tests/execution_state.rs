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

fn section<'a>(content: &'a str, heading: &str) -> Option<&'a str> {
    let marker = format!("## {heading}");
    let (_, rest) = content.split_once(&marker)?;
    Some(
        rest.split_once("\n## ")
            .map_or(rest, |(body, _)| body)
            .trim(),
    )
}

fn required_section<'a>(content: &'a str, heading: &str) -> &'a str {
    let body =
        section(content, heading).unwrap_or_else(|| panic!("missing `## {heading}` section"));
    assert!(!body.is_empty(), "`## {heading}` section must not be empty");
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
fn development_state_keeps_only_stage_mechanical_invariants() {
    let root = repository_root();
    let state = fs::read_to_string(root.join("docs/execution-state.md"))
        .expect("read docs/execution-state.md");

    assert_eq!(state.lines().next(), Some("# Development State"));
    let stage = field(&state, "Stage");
    let candidate = field(&state, "Candidate");
    let blocking = required_section(&state, "Blocking findings");

    match stage {
        "designing" | "implementation-ready" => {
            assert_eq!(candidate, "none");
            assert_eq!(blocking, "None.");
            if let Some(human_decision) = section(&state, "Human decision needed") {
                assert_eq!(human_decision, "None.");
            }
        }
        "decision-required" => {
            assert!(candidate == "none" || is_commit_sha(candidate));
            assert_eq!(blocking, "None.");
            assert_ne!(required_section(&state, "Human decision needed"), "None.");
        }
        "verification-ready" | "objective-complete" => {
            assert!(
                is_commit_sha(candidate),
                "{stage} requires an exact candidate SHA"
            );
            assert_eq!(blocking, "None.");
            if let Some(human_decision) = section(&state, "Human decision needed") {
                assert_eq!(human_decision, "None.");
            }
        }
        "correction-ready" => {
            assert!(
                is_commit_sha(candidate),
                "correction-ready requires the reviewed candidate SHA"
            );
            assert_ne!(blocking, "None.");
            if let Some(human_decision) = section(&state, "Human decision needed") {
                assert_eq!(human_decision, "None.");
            }
        }
        other => panic!("unknown development stage: {other}"),
    }

    for forbidden in [
        "Next actor:",
        "Recommended lane:",
        "Implementation starts on short continuation:",
        "Agent:",
        "Session role:",
    ] {
        assert!(
            !state.contains(forbidden),
            "Development State must not persist actor/presentation routing: {forbidden}"
        );
    }
}

#[test]
fn development_workflow_owners_are_discoverable() {
    let root = repository_root();

    for relative in [
        "AGENTS.md",
        "docs/current-objective.md",
        "docs/execution-state.md",
        "docs/execution-workflow.md",
        "skills/implement-spec/SKILL.md",
        "skills/review-code/SKILL.md",
        "docs/contributing/specification-workflow.md",
        "docs/contributing/implementation-rationale.md",
    ] {
        assert_file_exists(&root, relative);
    }

    let agents = fs::read_to_string(root.join("AGENTS.md")).expect("read AGENTS.md");
    for owner in [
        "docs/current-objective.md",
        "docs/execution-state.md",
        "docs/execution-workflow.md",
        "skills/implement-spec/SKILL.md",
        "skills/review-code/SKILL.md",
    ] {
        assert!(
            agents.contains(owner),
            "AGENTS.md must route agents to canonical owner: {owner}"
        );
    }
}

#[test]
fn development_workflow_keeps_structural_contracts() {
    let root = repository_root();
    let workflow = fs::read_to_string(root.join("docs/execution-workflow.md"))
        .expect("read docs/execution-workflow.md");

    for heading in [
        "## Model autonomy within hard boundaries",
        "## Pre-action freshness gate",
        "## Development State",
        "## Implementation readiness gate",
        "## Activity class and continuation boundary",
        "## Decision presentation gate for `decision-required`",
        "## Candidate / state transition",
        "## Human-facing execution summary",
        "## Post-action report verification",
        "## Public repository trust boundary",
        "## Integrity check",
    ] {
        assert!(
            workflow.contains(heading),
            "development workflow lost required structural owner section: {heading}"
        );
    }

    for stage in [
        "`designing`",
        "`decision-required`",
        "`implementation-ready`",
        "`verification-ready`",
        "`correction-ready`",
        "`objective-complete`",
    ] {
        assert!(
            workflow.contains(stage),
            "development workflow lost stage contract: {stage}"
        );
    }

    for label in [
        "### 次に進むと",
        "**現在地**",
        "**次にやること**",
        "**判断が必要**",
        "**「進める」の意味**",
        "**本格実装**",
        "**停止地点**",
    ] {
        assert!(
            workflow.contains(label),
            "development workflow lost stable Human-facing label: {label}"
        );
    }
}
