//! CONFIG-EDIT: exact TOML bytes and a separate physical-file draft. The parser
//! locates edits; its document formatter never writes the canonical source.
use crate::{
    Error, Result,
    native::{self, Fault, Outcome, Snapshot, WriteResult},
    project, semantic,
};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, ops::Range, path::PathBuf, sync::Arc};
use toml_edit::{Array, Document, Item, Table};

pub const PATH: &str = "masterdata.toml";
const PAGE: usize = 64;

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "operation", rename_all = "camelCase")]
pub enum ListEdit {
    Add { text: String },
    Remove { index: usize },
    Replace { index: usize, text: String },
}
#[derive(Clone, Debug, Deserialize)]
#[serde(
    tag = "operation",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum Operation {
    AddProfile {
        name: String,
    },
    Tags {
        profile: String,
        exclude: bool,
        edit: ListEdit,
    },
    AddTarget {
        kind: String,
        path: String,
    },
    TargetPath {
        index: usize,
        text: String,
    },
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Location {
    pub line: usize,
    pub column: usize,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Problem {
    pub message: String,
    pub location: Location,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub name: String,
    pub editable: bool,
    pub reason: Option<String>,
    pub location: Location,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Target {
    pub index: usize,
    pub kind: Option<String>,
    pub path: Option<String>,
    pub editable: bool,
    pub reason: Option<String>,
    pub location: Location,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub index: usize,
    pub text: String,
    pub valid: bool,
    pub reason: Option<String>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct List {
    pub entries: Vec<Entry>,
    pub total: usize,
    pub start: usize,
    pub editable: bool,
    pub reason: Option<String>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Detail {
    pub name: String,
    pub include: List,
    pub exclude: List,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    pub revision: u64,
    pub dirty: bool,
    pub outcome: Option<Outcome>,
    pub editable: bool,
    pub reason: Option<String>,
    pub valid: bool,
    pub profiles: Vec<Profile>,
    pub profile_count: usize,
    pub profile_start: usize,
    pub targets: Vec<Target>,
    pub target_count: usize,
    pub target_start: usize,
    pub detail: Option<Detail>,
    pub problems: Vec<Problem>,
    pub can_add_profile: bool,
    pub can_add_target: bool,
}

#[derive(Clone, Debug)]
pub struct Editor {
    root: PathBuf,
    pub base: Snapshot,
    pub bytes: Arc<str>,
    document: Option<Document<Arc<str>>>,
    parse_error: Option<String>,
    unavailable: Option<String>,
    // Only new, absent properties own an inserted boundary newline. Other
    // comments/trivia are never inferred to belong to an entry being removed.
    inserted: BTreeMap<(String, bool), bool>,
    pub revision: u64,
    pub outcome: Option<Outcome>,
}

fn parse(bytes: Arc<str>) -> Result<Document<Arc<str>>> {
    // Check both the source-locator grammar and the product's TOML reader. A
    // newer syntax accepted by just one parser must not become a saved draft.
    let _: toml::Value =
        toml::from_str(&bytes).map_err(|e| Error::new("E-CONFIG-SYNTAX", e.to_string()))?;
    Document::parse(bytes).map_err(|e| Error::new("E-CONFIG-SYNTAX", e.to_string()))
}
fn unsupported(message: impl Into<String>) -> Error {
    Error::new("E-CONFIG-EDIT-UNSUPPORTED", message)
}
fn value(bytes: &str) -> Result<toml::Value> {
    toml::from_str(bytes).map_err(|e| Error::new("E-CONFIG-SYNTAX", e.to_string()))
}
fn at<'a>(table: &'a Table, path: &[&str]) -> Option<&'a Item> {
    let (first, rest) = path.split_first()?;
    let item = table.get(first)?;
    if rest.is_empty() {
        Some(item)
    } else {
        at(item.as_table()?, rest)
    }
}
fn explicit(table: &Table) -> Result<Range<usize>> {
    if table.is_implicit() || table.is_dotted() {
        return Err(unsupported(
            "explicit TOML table required; dotted/inline representation is read-only",
        ));
    }
    table
        .span()
        .ok_or_else(|| unsupported("table has no source span"))
}
fn profile<'a>(doc: &'a Document<Arc<str>>, name: &str) -> Result<&'a Table> {
    let t = at(doc.as_table(), &["build", "profiles", name])
        .and_then(Item::as_table)
        .ok_or_else(|| unsupported(format!("profile {name}: explicit table required")))?;
    explicit(t)?;
    Ok(t)
}
fn array<'a>(table: &'a Table, key: &str) -> Result<Option<&'a Array>> {
    match table.get(key) {
        None => Ok(None),
        Some(item) => {
            let a = item
                .as_array()
                .ok_or_else(|| unsupported(format!("{key}: string array required")))?;
            if a.iter().any(|v| v.as_str().is_none() || v.span().is_none()) {
                return Err(unsupported(format!(
                    "{key}: non-string/ambiguous entry is read-only"
                )));
            }
            Ok(Some(a))
        }
    }
}
fn comma(bytes: &str, span: Range<usize>) -> Result<Option<usize>> {
    let mut comment = false;
    let mut found = None;
    for (offset, b) in bytes.as_bytes()[span.clone()].iter().copied().enumerate() {
        if comment {
            if b == b'\n' {
                comment = false;
            }
            continue;
        }
        match b {
            b'#' => comment = true,
            b',' if found.is_none() => found = Some(span.start + offset),
            b' ' | b'\t' | b'\r' | b'\n' => (),
            _ => return Err(unsupported("array separator cannot be safely located")),
        }
    }
    Ok(found)
}
fn quote(text: &str, old: Option<&str>) -> Result<String> {
    if let Some(old) = old {
        for delimiter in ["'''", "\"\"\"", "'", "\""] {
            if old.starts_with(delimiter) && old.ends_with(delimiter) {
                let candidate = format!("{delimiter}{text}{delimiter}");
                if value(&format!("v = {candidate}"))
                    .ok()
                    .and_then(|v| v.get("v").and_then(toml::Value::as_str).map(str::to_owned))
                    .as_deref()
                    == Some(text)
                {
                    return Ok(candidate);
                }
                break;
            }
        }
    }
    let atom =
        serde_json::to_string(text).map_err(|e| Error::new("E-CONFIG-INPUT", e.to_string()))?;
    if value(&format!("v = {atom}"))?
        .get("v")
        .and_then(toml::Value::as_str)
        != Some(text)
    {
        return Err(Error::new(
            "E-CONFIG-INPUT",
            "input cannot be represented losslessly",
        ));
    }
    Ok(atom)
}
fn patch(bytes: &str, mut edits: Vec<(Range<usize>, String)>) -> Result<String> {
    edits.sort_by_key(|(r, _)| (r.start, r.end));
    for pair in edits.windows(2) {
        if pair[0].0.end > pair[1].0.start {
            return Err(unsupported("overlapping config edit spans"));
        }
    }
    let mut output = bytes.to_owned();
    for (r, text) in edits.into_iter().rev() {
        if r.end > output.len()
            || !output.is_char_boundary(r.start)
            || !output.is_char_boundary(r.end)
        {
            return Err(unsupported("invalid config edit span"));
        }
        output.replace_range(r, &text);
    }
    Ok(output)
}
fn newline(bytes: &str) -> &'static str {
    if bytes.contains("\r\n") { "\r\n" } else { "\n" }
}
fn location(bytes: &str, offset: usize) -> Location {
    let before = &bytes[..offset.min(bytes.len())];
    Location {
        line: before.bytes().filter(|&b| b == b'\n').count() + 1,
        column: before.rsplit('\n').next().unwrap_or("").chars().count() + 1,
    }
}
fn append(bytes: &str, text: &str) -> String {
    let nl = newline(bytes);
    format!(
        "{bytes}{}{text}",
        if bytes.is_empty() || bytes.ends_with(&format!("{nl}{nl}")) {
            String::new()
        } else if bytes.ends_with('\n') {
            nl.to_owned()
        } else {
            format!("{nl}{nl}")
        }
    )
}
fn list_values(table: &Table, key: &str) -> Result<Vec<String>> {
    Ok(array(table, key)?.map_or_else(Vec::new, |a| {
        a.iter().map(|v| v.as_str().unwrap().to_owned()).collect()
    }))
}

impl Editor {
    pub fn from_snapshot(root: PathBuf, base: Snapshot) -> Self {
        let mut result = Self {
            root,
            bytes: base.bytes.clone(),
            base,
            document: None,
            parse_error: None,
            unavailable: None,
            inserted: BTreeMap::new(),
            revision: 0,
            outcome: None,
        };
        result.reparse();
        result
    }
    pub fn open(root: PathBuf) -> Result<Self> {
        let base = native::capture(&root, std::slice::from_ref(&root), PATH)?;
        Ok(Self::from_snapshot(root, base))
    }
    fn reparse(&mut self) {
        match parse(self.bytes.clone()) {
            Ok(doc) => {
                self.document = Some(doc);
                self.parse_error = None;
            }
            Err(e) => {
                self.document = None;
                self.parse_error = Some(e.to_string());
            }
        }
    }
    pub fn dirty(&self) -> bool {
        self.bytes != self.base.bytes
    }
    pub fn uncertain(&self) -> bool {
        self.outcome == Some(Outcome::OutcomeUnknown)
    }
    fn capture(&self) -> Result<Snapshot> {
        let actual = native::capture(&self.root, std::slice::from_ref(&self.root), PATH)?;
        if !self.base.same_binding(&actual) {
            return Err(Error::new(
                "E-CONFIG-BINDING",
                "Project root/config binding changed; reopen Project explicitly",
            ));
        }
        Ok(actual)
    }
    /// Returns true only when a clean base was advanced. Dirty drafts never
    /// rebase implicitly, including file replacement with identical content.
    pub fn observe(&mut self) -> Result<bool> {
        let actual = match self.capture() {
            Ok(a) => a,
            Err(e) => {
                self.unavailable = Some(e.to_string());
                return Err(e);
            }
        };
        self.unavailable = None;
        if self.uncertain() {
            return Ok(false);
        }
        if self.base.matches(&actual) {
            return Ok(false);
        }
        if self.dirty() {
            self.outcome = Some(Outcome::Conflict);
            return Ok(false);
        }
        self.reset(actual);
        Ok(true)
    }
    fn reset(&mut self, actual: Snapshot) {
        self.bytes = actual.bytes.clone();
        self.base = actual;
        self.inserted.clear();
        self.outcome = None;
        self.unavailable = None;
        self.revision += 1;
        self.reparse();
    }
    pub fn reload(&mut self, revision: u64, discard_authorized: bool) -> Result<()> {
        self.check_revision(revision)?;
        if (self.dirty() || self.uncertain()) && !discard_authorized {
            return Err(Error::new(
                "E-CONFIG-DISCARD-AUTHORIZATION",
                "explicit input discard confirmation required",
            ));
        }
        let actual = self.capture()?;
        self.reset(actual);
        Ok(())
    }
    pub fn compare(&self) -> Result<(String, String, String)> {
        let actual = self.capture()?;
        Ok((
            actual.content,
            actual.bytes.to_string(),
            self.bytes.to_string(),
        ))
    }
    pub fn saved_compare(&self) -> (String, String) {
        (self.base.bytes.to_string(), self.bytes.to_string())
    }
    fn check_revision(&self, revision: u64) -> Result<()> {
        if revision != self.revision {
            return Err(Error::new(
                "E-CONFIG-EDIT-STALE",
                "config buffer revision changed; input retained",
            ));
        }
        Ok(())
    }
    pub fn edit(&mut self, revision: u64, operation: Operation) -> Result<()> {
        self.check_revision(revision)?;
        if self.uncertain() {
            return Err(Error::new(
                "E-OUTCOME-UNKNOWN",
                "observe actual config before another operation",
            ));
        }
        if let Some(reason) = &self.unavailable {
            return Err(unsupported(reason));
        }
        let doc = self.document.as_ref().ok_or_else(|| {
            unsupported(
                self.parse_error
                    .as_deref()
                    .unwrap_or("TOML cannot be located safely"),
            )
        })?;
        let mut expected = value(&self.bytes)?;
        let nl = newline(&self.bytes);
        let mut inserted = self.inserted.clone();
        let candidate = match operation {
            Operation::AddProfile { name } => {
                if !semantic::kebab(&name) {
                    return Err(Error::new(
                        "E-CONFIG-PROFILE-NAME",
                        "profile name must be lowercase kebab-case; input was not changed",
                    ));
                }
                let build = at(doc.as_table(), &["build"])
                    .and_then(Item::as_table)
                    .ok_or_else(|| unsupported("build table is unavailable"))?;
                if let Some(profiles) = build.get("profiles") {
                    let profiles = profiles
                        .as_table()
                        .ok_or_else(|| unsupported("inline profiles are read-only"))?;
                    if profiles.contains_key(&name) {
                        return Err(Error::new(
                            "E-CONFIG-PROFILE-NAME",
                            "profile already exists; input was not changed",
                        ));
                    }
                }
                let build = expected
                    .get_mut("build")
                    .and_then(toml::Value::as_table_mut)
                    .ok_or_else(|| unsupported("build table is unavailable"))?;
                build
                    .entry("profiles")
                    .or_insert_with(|| toml::Value::Table(Default::default()))
                    .as_table_mut()
                    .ok_or_else(|| unsupported("profiles table is unavailable"))?
                    .insert(name.clone(), toml::Value::Table(Default::default()));
                append(&self.bytes, &format!("[build.profiles.{name}]{nl}"))
            }
            Operation::AddTarget { kind, path } => {
                if !matches!(kind.as_str(), "csharp" | "binary") {
                    return Err(Error::new(
                        "E-CONFIG-TARGET-KIND",
                        "explicit csharp or binary kind required",
                    ));
                }
                if let Some(publish) = at(doc.as_table(), &["publish"]) {
                    let publish = publish
                        .as_table()
                        .ok_or_else(|| unsupported("inline publish configuration is read-only"))?;
                    if publish
                        .get("targets")
                        .is_some_and(|i| i.as_array_of_tables().is_none())
                    {
                        return Err(unsupported(
                            "publish.targets requires an array of explicit tables",
                        ));
                    }
                }
                let root = expected.as_table_mut().unwrap();
                let publish = root
                    .entry("publish")
                    .or_insert_with(|| toml::Value::Table(Default::default()))
                    .as_table_mut()
                    .ok_or_else(|| unsupported("publish table is unavailable"))?;
                let targets = publish
                    .entry("targets")
                    .or_insert_with(|| toml::Value::Array(vec![]))
                    .as_array_mut()
                    .ok_or_else(|| unsupported("targets array is unavailable"))?;
                targets.push(toml::Value::Table(
                    [
                        (String::from("kind"), toml::Value::String(kind.clone())),
                        (String::from("path"), toml::Value::String(path.clone())),
                    ]
                    .into_iter()
                    .collect(),
                ));
                append(
                    &self.bytes,
                    &format!(
                        "[[publish.targets]]{nl}kind = {}{nl}path = {}{nl}",
                        quote(&kind, None)?,
                        quote(&path, None)?
                    ),
                )
            }
            Operation::TargetPath { index, text } => {
                let targets = at(doc.as_table(), &["publish", "targets"])
                    .and_then(Item::as_array_of_tables)
                    .ok_or_else(|| unsupported("targets require explicit array tables"))?;
                let table = targets.get(index).ok_or_else(|| {
                    Error::new(
                        "E-CONFIG-TARGET-MISSING",
                        "target occurrence is unavailable",
                    )
                })?;
                explicit(table)?;
                let item = table
                    .get("path")
                    .ok_or_else(|| unsupported("target path is absent"))?;
                if item.as_str() == Some(text.as_str()) {
                    return Ok(());
                }
                let span = item
                    .span()
                    .filter(|_| item.as_str().is_some())
                    .ok_or_else(|| unsupported("target path is not a locatable string"))?;
                expected["publish"]["targets"][index]["path"] = toml::Value::String(text.clone());
                patch(
                    &self.bytes,
                    vec![(span.clone(), quote(&text, Some(&self.bytes[span]))?)],
                )?
            }
            Operation::Tags {
                profile: name,
                exclude,
                edit,
            } => {
                let key = if exclude {
                    "exclude_tags"
                } else {
                    "include_tags"
                };
                let table = profile(doc, &name)?;
                let current = array(table, key)?;
                let mut values = list_values(table, key)?;
                if let ListEdit::Replace { index, text } = &edit
                    && values.get(*index) == Some(text)
                {
                    return Ok(());
                }
                match &edit {
                    ListEdit::Add { text } => values.push(text.clone()),
                    ListEdit::Remove { index } => {
                        if *index >= values.len() {
                            return Err(Error::new(
                                "E-CONFIG-TAG-MISSING",
                                "tag occurrence is unavailable",
                            ));
                        }
                        values.remove(*index);
                    }
                    ListEdit::Replace { index, text } => {
                        let entry = values.get_mut(*index).ok_or_else(|| {
                            Error::new("E-CONFIG-TAG-MISSING", "tag occurrence is unavailable")
                        })?;
                        *entry = text.clone();
                    }
                }
                let restore_absent =
                    values.is_empty() && inserted.contains_key(&(name.clone(), exclude));
                let target = expected["build"]["profiles"][&name]
                    .as_table_mut()
                    .ok_or_else(|| unsupported("profile table is unavailable"))?;
                if restore_absent {
                    target.remove(key);
                } else {
                    target.insert(
                        key.to_owned(),
                        toml::Value::Array(
                            values.iter().cloned().map(toml::Value::String).collect(),
                        ),
                    );
                }
                if let Some(a) = current {
                    if restore_absent {
                        let key_span = table
                            .key(key)
                            .and_then(toml_edit::Key::span)
                            .ok_or_else(|| unsupported("property key is unlocatable"))?;
                        let end = a
                            .span()
                            .ok_or_else(|| unsupported("array is unlocatable"))?
                            .end;
                        let line_start = self.bytes[..key_span.start]
                            .rfind('\n')
                            .map_or(0, |i| i + 1);
                        if !self.bytes[line_start..key_span.start].trim().is_empty() {
                            return Err(unsupported("new property boundary is ambiguous"));
                        }
                        let suffix = &self.bytes[end..];
                        let owns_prefix = inserted.remove(&(name.clone(), exclude)).unwrap();
                        let only_property = table.iter().all(|(name, _)| name == key);
                        if owns_prefix
                            && !only_property
                            && let Some((_, owner)) = inserted
                                .iter_mut()
                                .find(|((profile, _), _)| profile == &name)
                        {
                            *owner = true;
                        }
                        let (start, end) = if owns_prefix
                            && only_property
                            && line_start >= nl.len()
                            && self.bytes[line_start - nl.len()..line_start] == *nl
                            && (suffix.is_empty() || suffix.starts_with(nl))
                        {
                            (
                                line_start - nl.len(),
                                end + if suffix.starts_with(nl) { nl.len() } else { 0 },
                            )
                        } else if suffix.starts_with(nl) {
                            (line_start, end + nl.len())
                        } else if suffix.is_empty() {
                            (line_start, end)
                        } else {
                            return Err(unsupported("new property acquired unrelated trivia"));
                        };
                        patch(&self.bytes, vec![(start..end, String::new())])?
                    } else {
                        edit_array(&self.bytes, a, &edit)?
                    }
                } else {
                    let span = explicit(table)?;
                    if self.bytes.as_bytes().get(span.start) != Some(&b'[') {
                        return Err(unsupported("profile table header cannot be located"));
                    }
                    let end = self.bytes[span.start..]
                        .find('\n')
                        .map_or(self.bytes.len(), |i| span.start + i + 1);
                    let owns_prefix = end == self.bytes.len() && !self.bytes.ends_with('\n');
                    let atoms = values
                        .iter()
                        .map(|s| quote(s, None))
                        .collect::<Result<Vec<_>>>()?
                        .join(", ");
                    let text = if owns_prefix {
                        format!("{nl}{key} = [{atoms}]")
                    } else {
                        format!("{key} = [{atoms}]{nl}")
                    };
                    inserted.insert((name, exclude), owns_prefix);
                    patch(&self.bytes, vec![(end..end, text)])?
                }
            }
        };
        let new_doc = parse(Arc::from(candidate.as_str()))?;
        if value(&candidate)? != expected {
            return Err(Error::new(
                "E-CONFIG-EDIT-POSTCONDITION",
                "edit changed an unrelated value or failed its exact target",
            ));
        }
        if candidate != self.bytes.as_ref() {
            self.bytes = candidate.into();
            self.document = Some(new_doc);
            self.inserted = inserted;
            self.revision += 1;
        }
        Ok(())
    }
    pub fn save(&mut self, revision: u64, fault: Fault) -> Result<WriteResult> {
        self.check_revision(revision)?;
        if self.uncertain() {
            return Err(Error::new(
                "E-OUTCOME-UNKNOWN",
                "config Save was not retried; actual bytes must be observed",
            ));
        }
        parse(self.bytes.clone())?;
        if !self.dirty() {
            return Ok(
                match native::preflight(
                    &self.root,
                    std::slice::from_ref(&self.root),
                    PATH,
                    &self.base,
                ) {
                    Ok(_) => WriteResult::new(PATH, Outcome::Success, "config is unchanged"),
                    Err(e) => {
                        self.outcome = Some(Outcome::Conflict);
                        WriteResult::new(PATH, Outcome::Conflict, e.to_string())
                    }
                },
            );
        }
        let (result, actual) = native::commit(
            &self.root,
            std::slice::from_ref(&self.root),
            PATH,
            &self.base,
            &self.bytes,
            fault,
        );
        if result.outcome == Outcome::Success {
            self.reset(actual.unwrap());
        } else {
            self.outcome = Some(result.outcome.clone());
        }
        Ok(result)
    }
    pub fn recheck(&mut self) -> Result<WriteResult> {
        let actual = self.capture()?;
        let result = if actual.bytes == self.bytes {
            self.reset(actual);
            WriteResult::new(
                PATH,
                Outcome::Success,
                "actual config matches the candidate",
            )
        } else if actual.matches(&self.base) {
            self.outcome = Some(Outcome::Failure);
            self.unavailable = None;
            WriteResult::new(
                PATH,
                Outcome::Failure,
                "actual config still matches the base; draft retained",
            )
        } else {
            self.outcome = Some(Outcome::Conflict);
            self.unavailable = None;
            WriteResult::new(
                PATH,
                Outcome::Conflict,
                "actual config differs; Compare / Reload required",
            )
        };
        Ok(result)
    }
    pub fn view(&self, selected: Option<&str>, starts: [usize; 4]) -> View {
        let reason = self
            .unavailable
            .clone()
            .or_else(|| self.parse_error.clone())
            .or_else(|| {
                self.uncertain()
                    .then(|| "Outcome Unknown: Recheck actual config before editing".into())
            });
        let mut view = View {
            revision: self.revision,
            dirty: self.dirty(),
            outcome: self.outcome.clone(),
            editable: reason.is_none(),
            reason,
            valid: false,
            profiles: vec![],
            profile_count: 0,
            profile_start: starts[0],
            targets: vec![],
            target_count: 0,
            target_start: starts[1],
            detail: None,
            problems: vec![],
            can_add_profile: false,
            can_add_target: false,
        };
        let strict = project::config(&self.bytes);
        view.valid = strict.is_ok();
        if let Err(e) = strict {
            view.problems.push(Problem {
                message: e.to_string(),
                location: location(&self.bytes, 0),
            });
        }
        let Some(doc) = &self.document else {
            return view;
        };
        let build = at(doc.as_table(), &["build"]).and_then(Item::as_table);
        let profiles = build.and_then(|t| t.get("profiles"));
        view.can_add_profile =
            view.editable && build.is_some() && profiles.is_none_or(|i| i.as_table().is_some());
        if let Some(profiles) = profiles {
            if let Some(table) = profiles.as_table() {
                view.profile_count = table.len();
                for (name, item) in table.iter().skip(starts[0]).take(PAGE) {
                    let reason = item
                        .as_table()
                        .and_then(|t| explicit(t).err())
                        .map(|e| e.message)
                        .or_else(|| {
                            item.as_table()
                                .is_none()
                                .then(|| "inline/non-table Profile is read-only".into())
                        });
                    let offset = item.span().map_or(0, |r| r.start);
                    view.profiles.push(Profile {
                        name: name.into(),
                        editable: view.editable && reason.is_none(),
                        reason,
                        location: location(&self.bytes, offset),
                    });
                }
            } else {
                view.problems.push(Problem {
                    message: "build.profiles: inline/non-table representation is read-only".into(),
                    location: location(&self.bytes, profiles.span().map_or(0, |r| r.start)),
                });
            }
        }
        if let Some(name) = selected
            && let Ok(table) = profile(doc, name)
        {
            let include = project_list(table, "include_tags", starts[2], view.editable);
            let exclude = project_list(table, "exclude_tags", starts[3], view.editable);
            view.detail = Some(Detail {
                name: name.into(),
                include,
                exclude,
            });
            for (key, item) in table.iter() {
                if !matches!(key, "include_tags" | "exclude_tags") {
                    view.problems.push(Problem {message:format!("{name}.{key}: unknown property is preserved; repair it in the source file"),location:location(&self.bytes,item.span().map_or(0,|r|r.start))});
                }
            }
            if !semantic::kebab(name) {
                view.problems.push(Problem {
                    message: format!(
                        "{name}: invalid Profile name is preserved; rename in the source file"
                    ),
                    location: location(&self.bytes, table.span().map_or(0, |r| r.start)),
                });
            }
        }
        let publish = at(doc.as_table(), &["publish"]);
        let targets = at(doc.as_table(), &["publish", "targets"]);
        view.can_add_target = view.editable
            && publish.is_none_or(|i| i.as_table().is_some())
            && targets.is_none_or(|i| i.as_array_of_tables().is_some());
        if let Some(targets) = targets {
            if let Some(array) = targets.as_array_of_tables() {
                view.target_count = array.len();
                for (index, t) in array.iter().enumerate().skip(starts[1]).take(PAGE) {
                    let path = t.get("path");
                    let kind = t.get("kind").and_then(Item::as_str).map(str::to_owned);
                    let reason = explicit(t).err().map(|e| e.message).or_else(|| {
                        path.is_none_or(|i| i.as_str().is_none() || i.span().is_none())
                            .then(|| {
                                "path is absent/non-string; repair it in the source file".into()
                            })
                    });
                    view.targets.push(Target {
                        index,
                        kind,
                        path: path.and_then(Item::as_str).map(str::to_owned),
                        editable: view.editable && reason.is_none(),
                        reason,
                        location: location(&self.bytes, t.span().map_or(0, |r| r.start)),
                    });
                    for (key, item) in t.iter() {
                        if !matches!(key, "kind" | "path") {
                            view.problems.push(Problem {
                                message: format!(
                                    "target {}.{key}: unknown property is preserved",
                                    index + 1
                                ),
                                location: location(&self.bytes, item.span().map_or(0, |r| r.start)),
                            });
                        }
                    }
                }
            } else {
                view.problems.push(Problem {
                    message: "publish.targets: inline/non-array-table representation is read-only"
                        .into(),
                    location: location(&self.bytes, targets.span().map_or(0, |r| r.start)),
                });
            }
        }
        view.problems.truncate(100);
        view
    }
}

fn project_list(table: &Table, key: &str, start: usize, enabled: bool) -> List {
    match list_values(table, key) {
        Err(e) => List {
            entries: vec![],
            total: 0,
            start,
            editable: false,
            reason: Some(e.message),
        },
        Ok(values) => {
            let other = list_values(
                table,
                if key == "include_tags" {
                    "exclude_tags"
                } else {
                    "include_tags"
                },
            )
            .unwrap_or_default();
            let entries = values
                .iter()
                .enumerate()
                .skip(start)
                .take(PAGE)
                .map(|(index, text)| {
                    let reason = if !semantic::kebab(text) {
                        Some("lowercase kebab-case tag required".into())
                    } else if values.iter().filter(|s| *s == text).count() > 1 {
                        Some("duplicate tag".into())
                    } else if other.contains(text) {
                        Some("include/exclude overlap".into())
                    } else {
                        None
                    };
                    Entry {
                        index,
                        text: text.clone(),
                        valid: reason.is_none(),
                        reason,
                    }
                })
                .collect();
            List {
                entries,
                total: values.len(),
                start,
                editable: enabled,
                reason: None,
            }
        }
    }
}

fn edit_array(bytes: &str, array: &Array, edit: &ListEdit) -> Result<String> {
    let span = array
        .span()
        .ok_or_else(|| unsupported("array has no source span"))?;
    if bytes.as_bytes().get(span.start) != Some(&b'[')
        || bytes.as_bytes().get(span.end - 1) != Some(&b']')
    {
        return Err(unsupported("array boundary cannot be located"));
    }
    let values = array
        .iter()
        .map(|v| v.span().ok_or_else(|| unsupported("entry has no span")))
        .collect::<Result<Vec<_>>>()?;
    let mut edits = Vec::new();
    match edit {
        ListEdit::Replace { index, text } => {
            let r = values
                .get(*index)
                .ok_or_else(|| Error::new("E-CONFIG-TAG-MISSING", "tag occurrence unavailable"))?
                .clone();
            edits.push((r.clone(), quote(text, Some(&bytes[r]))?));
        }
        ListEdit::Remove { index } => {
            let r = values
                .get(*index)
                .ok_or_else(|| Error::new("E-CONFIG-TAG-MISSING", "tag occurrence unavailable"))?
                .clone();
            let after_end = values.get(index + 1).map_or(span.end - 1, |r| r.start);
            let after = comma(bytes, r.end..after_end)?;
            let before_start = index
                .checked_sub(1)
                .and_then(|i| values.get(i))
                .map_or(span.start + 1, |r| r.end);
            let before = comma(bytes, before_start..r.start)?;
            edits.push((r, String::new()));
            if let Some(c) = after.or(before) {
                edits.push((c..c + 1, String::new()));
            }
            if values.len() == 1 {
                // Empty is explicit []; retain all comments within the edited
                // array as trailing/standalone trivia, never delete by proximity.
                let inside = patch(bytes, edits.clone())?;
                let removed = edits.iter().map(|(r, _)| r.len()).sum::<usize>();
                let trivia = &inside[span.start + 1..span.end - removed - 1];
                let replacement = if trivia.contains('#') {
                    format!("[]{trivia}")
                } else {
                    "[]".into()
                };
                edits = vec![(span, replacement)];
            }
        }
        ListEdit::Add { text } => {
            let close = span.end - 1;
            let trailing = if let Some(last) = values.last() {
                comma(bytes, last.end..close)?
            } else {
                comma(bytes, span.start + 1..close)?
            };
            if let Some(last) = values.last()
                && trailing.is_none()
            {
                edits.push((last.end..last.end, ",".into()));
            }
            let atom = quote(text, None)?;
            let line_start = bytes[..close].rfind('\n').map_or(0, |i| i + 1);
            if line_start > span.start && bytes[line_start..close].trim().is_empty() {
                let indent = values
                    .first()
                    .and_then(|r| {
                        let begin = bytes[..r.start].rfind('\n').map_or(0, |i| i + 1);
                        bytes[begin..r.start]
                            .trim()
                            .is_empty()
                            .then(|| bytes[begin..r.start].to_owned())
                    })
                    .unwrap_or_else(|| format!("{}  ", &bytes[line_start..close]));
                edits.push((
                    line_start..line_start,
                    format!("{indent}{atom},{}", newline(bytes)),
                ));
            } else {
                let spacing = if values.is_empty() || close == span.start + 1 {
                    ""
                } else {
                    " "
                };
                edits.push((
                    close..close,
                    format!(
                        "{spacing}{atom}{}",
                        if trailing.is_some() { "," } else { "" }
                    ),
                ));
            }
        }
    }
    patch(bytes, edits)
}
