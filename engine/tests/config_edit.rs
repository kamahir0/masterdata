use masterdata_engine::{
    config::{Editor, ListEdit, Operation},
    native::{Fault, Outcome},
    project,
};
use std::fs;
const BASE: &str = "# project\n[project]\nid = 'rewrite.config'\nname = \"Settings\"\nversion = '0.1.0'\n[sources]\nroots = ['sources']\n[build]\nartifact_dir = '.masterdata/output'\ncache = '.masterdata/cache'\n";
fn setup(extra: &str) -> (tempfile::TempDir, Editor) {
    let t = tempfile::tempdir().unwrap();
    fs::create_dir(t.path().join("sources")).unwrap();
    fs::write(t.path().join("masterdata.toml"), format!("{BASE}{extra}")).unwrap();
    let editor = Editor::open(t.path().canonicalize().unwrap()).unwrap();
    (t, editor)
}
fn edit(e: &mut Editor, op: Operation) {
    e.edit(e.revision, op).unwrap();
}
fn tags(e: &mut Editor, exclude: bool, op: ListEdit) {
    edit(
        e,
        Operation::Tags {
            profile: "production".into(),
            exclude,
            edit: op,
        },
    );
}
#[test]
fn quoted_multiline_crlf_entry_patches_keep_comments_unknown_sections_and_other_entries() {
    let (t, mut e) = setup(
        "[build.profiles.\"production\"] # header\ninclude_tags = [\n  'common', # first\n  \"debug\",\n  # standalone\n]\nexclude_tags = []\n[other]\nuntouched = '\\windows\\path' # unknown\n",
    );
    let original = fs::read_to_string(t.path().join("masterdata.toml"))
        .unwrap()
        .replace('\n', "\r\n");
    fs::write(t.path().join("masterdata.toml"), &original).unwrap();
    e.reload(e.revision, true).unwrap();
    assert!(!e.dirty());
    assert!(e.view(Some("production"), [0; 4]).editable);
    tags(
        &mut e,
        false,
        ListEdit::Replace {
            index: 1,
            text: "release".into(),
        },
    );
    assert_eq!(
        e.bytes.as_ref(),
        original.replace("\"debug\"", "\"release\"")
    );
    tags(
        &mut e,
        false,
        ListEdit::Add {
            text: "new-tag".into(),
        },
    );
    assert_eq!(
        e.bytes.as_ref(),
        original.replace("\"debug\"", "\"release\"").replace(
            "  # standalone\r\n]",
            "  # standalone\r\n  \"new-tag\",\r\n]"
        )
    );
    tags(&mut e, false, ListEdit::Remove { index: 0 });
    assert!(e.bytes.contains(" # first\r\n"));
    assert!(e.bytes.contains("# standalone\r\n"));
    assert!(
        e.bytes
            .ends_with("[other]\r\nuntouched = '\\windows\\path' # unknown\r\n")
    );
    assert_eq!(
        fs::read_to_string(t.path().join("masterdata.toml")).unwrap(),
        original
    );
    assert_eq!(
        e.save(e.revision, Fault::None).unwrap().outcome,
        Outcome::Success
    );
    assert!(!e.dirty());
    assert!(
        project::config(&e.bytes).is_err(),
        "unknown sections remain strict-domain-invalid, saveable TOML"
    );
}
#[test]
fn absent_property_returns_to_exact_absence_and_existing_property_remains_explicit_empty() {
    for suffix in [
        "[build.profiles.production]\n# keep\n",
        "[build.profiles.production]",
    ] {
        let (_t, mut e) = setup(suffix);
        let original = e.bytes.clone();
        tags(
            &mut e,
            false,
            ListEdit::Add {
                text: "debug".into(),
            },
        );
        tags(&mut e, false, ListEdit::Remove { index: 0 });
        assert_eq!(e.bytes, original);
        assert!(!e.dirty());
    }
    let (_t, mut e) = setup("[build.profiles.production]\ninclude_tags = ['debug'] # outside\n");
    tags(&mut e, false, ListEdit::Remove { index: 0 });
    assert!(e.bytes.ends_with("include_tags = [] # outside\n"));
}
#[test]
fn two_absent_lists_restore_the_unterminated_header_in_either_removal_order() {
    for remove_first in [false, true] {
        let (_t, mut e) = setup("[build.profiles.production]");
        let original = e.bytes.clone();
        tags(
            &mut e,
            false,
            ListEdit::Add {
                text: "debug".into(),
            },
        );
        tags(
            &mut e,
            true,
            ListEdit::Add {
                text: "release".into(),
            },
        );
        tags(&mut e, remove_first, ListEdit::Remove { index: 0 });
        tags(&mut e, !remove_first, ListEdit::Remove { index: 0 });
        assert_eq!(e.bytes, original);
        assert!(!e.dirty());
    }
}
#[test]
fn empty_comment_array_retains_standalone_and_inline_comments() {
    let (_t, mut e) = setup(
        "[build.profiles.production]\ninclude_tags = [ # begin\n  'debug', # entry\n  # keep\n] # outside\nexclude_tags = []\n",
    );
    tags(&mut e, false, ListEdit::Remove { index: 0 });
    for c in ["# begin\n", "# entry\n", "# keep\n", "# outside\n"] {
        assert!(e.bytes.contains(c));
    }
    assert!(e.bytes.contains("include_tags = []"));
    assert!(project::config(&e.bytes).is_ok());
}
#[test]
fn invalid_empty_duplicates_overlap_can_save_but_profile_name_creation_is_strict() {
    let (_t, mut e) = setup("[build.profiles.production]\n");
    for text in ["", " Debug ", "debug", "debug"] {
        tags(&mut e, false, ListEdit::Add { text: text.into() });
    }
    tags(
        &mut e,
        true,
        ListEdit::Add {
            text: "debug".into(),
        },
    );
    let v = e.view(Some("production"), [0; 4]);
    assert!(!v.valid);
    assert!(v.detail.unwrap().include.entries.iter().all(|e| !e.valid));
    assert_eq!(
        e.save(e.revision, Fault::None).unwrap().outcome,
        Outcome::Success
    );
    assert!(project::config(&e.bytes).is_err());
    for name in ["", "Debug", "production", "not valid"] {
        let bytes = e.bytes.clone();
        assert!(
            e.edit(e.revision, Operation::AddProfile { name: name.into() })
                .is_err()
        );
        assert_eq!(e.bytes, bytes);
    }
    edit(
        &mut e,
        Operation::AddProfile {
            name: "release".into(),
        },
    );
    assert!(e.bytes.ends_with("[build.profiles.release]\n"));
}
#[test]
fn target_array_occurrence_quotes_and_unknown_members_are_preserved() {
    let (t, mut e) = setup(
        "[[publish.targets]]\nkind = 'binary'\npath = 'same' # first\n[[publish.targets]]\nkind = 'binary'\n'path' = \"same\" # second\nunknown = 7\n",
    );
    let original = e.bytes.clone();
    edit(
        &mut e,
        Operation::TargetPath {
            index: 1,
            text: "../new/bytes".into(),
        },
    );
    assert_eq!(
        e.bytes.as_ref(),
        original.replace("\"same\" # second", "\"../new/bytes\" # second")
    );
    edit(
        &mut e,
        Operation::AddTarget {
            kind: "csharp".into(),
            path: String::new(),
        },
    );
    assert!(
        e.bytes
            .ends_with("[[publish.targets]]\nkind = \"csharp\"\npath = \"\"\n")
    );
    assert_eq!(
        e.save(e.revision, Fault::None).unwrap().outcome,
        Outcome::Success
    );
    assert_eq!(
        fs::read_to_string(t.path().join("masterdata.toml")).unwrap(),
        e.bytes.as_ref()
    );
    assert!(!t.path().join(".masterdata").exists());
}
#[test]
fn malformed_and_inline_representations_are_readonly_with_reason_and_stale_edits_reject() {
    for text in [
        "[build.profiles.production]\ninclude_tags = ['a'\n",
        "[build.profiles]\nproduction = {include_tags = ['debug']}\n",
    ] {
        let (_t, mut e) = setup(text);
        let base = e.bytes.clone();
        assert!(
            e.edit(
                e.revision,
                Operation::Tags {
                    profile: "production".into(),
                    exclude: false,
                    edit: ListEdit::Add {
                        text: "common".into()
                    }
                }
            )
            .is_err()
        );
        assert_eq!(e.bytes, base);
        let v = e.view(Some("production"), [0; 4]);
        assert!(v.reason.is_some() || v.profiles.iter().any(|p| p.reason.is_some()));
    }
    let (_t, mut e) = setup("[build.profiles.production]\ninclude_tags = [\"\\u0064ebug\"]\n");
    let bytes = e.bytes.clone();
    let revision = e.revision;
    tags(
        &mut e,
        false,
        ListEdit::Replace {
            index: 0,
            text: "debug".into(),
        },
    );
    assert_eq!(e.bytes, bytes);
    assert_eq!(e.revision, revision);
    tags(
        &mut e,
        false,
        ListEdit::Add {
            text: "common".into(),
        },
    );
    assert!(
        e.edit(
            revision,
            Operation::AddProfile {
                name: "new-profile".into()
            }
        )
        .is_err()
    );
}
#[test]
fn fresh_conflict_never_overwrites_and_reload_requires_explicit_discard() {
    let (t, mut e) = setup("[build.profiles.production]\n");
    tags(
        &mut e,
        false,
        ListEdit::Add {
            text: "debug".into(),
        },
    );
    let draft = e.bytes.clone();
    let external = format!("{BASE}[build.profiles.production]\nexclude_tags = ['release']\n");
    fs::write(t.path().join("masterdata.toml"), &external).unwrap();
    assert_eq!(
        e.save(e.revision, Fault::None).unwrap().outcome,
        Outcome::Conflict
    );
    assert_eq!(e.bytes, draft);
    assert_eq!(e.compare().unwrap().1, external);
    assert!(e.reload(e.revision, false).is_err());
    e.reload(e.revision, true).unwrap();
    assert_eq!(e.bytes.as_ref(), external);
    assert!(!e.dirty());
}
#[cfg(feature = "oracle-faults")]
#[test]
fn failure_and_unknown_preserve_draft_and_retry_requires_actual_observation() {
    let (t, mut e) = setup("[build.profiles.production]\n");
    tags(
        &mut e,
        false,
        ListEdit::Add {
            text: "debug".into(),
        },
    );
    let draft = e.bytes.clone();
    let base = e.base.bytes.clone();
    assert_eq!(
        e.save(e.revision, Fault::BeforeCommit).unwrap().outcome,
        Outcome::Failure
    );
    assert_eq!(e.bytes, draft);
    assert_eq!(
        fs::read_to_string(t.path().join("masterdata.toml")).unwrap(),
        base.as_ref()
    );
    assert_eq!(
        e.save(e.revision, Fault::AfterCommitObservation)
            .unwrap()
            .outcome,
        Outcome::OutcomeUnknown
    );
    assert_eq!(e.bytes, draft);
    assert!(e.uncertain());
    assert!(e.save(e.revision, Fault::None).is_err());
    assert_eq!(e.recheck().unwrap().outcome, Outcome::Success);
    assert!(!e.dirty());
    assert!(!e.uncertain());
}
