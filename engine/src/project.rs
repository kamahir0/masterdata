//! Project inventory and immutable semantic snapshots. No authoring state lives here.
use crate::{
    Error, Result,
    semantic::{self, KeyPart, Reference, Table, Type, Typed, Types},
    source::{Document, Raw, content_identity},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Metadata {
    pub id: String,
    pub name: String,
    pub version: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sources {
    pub roots: Vec<String>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    #[serde(default)]
    pub include_tags: Vec<String>,
    #[serde(default)]
    pub exclude_tags: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BuildConfig {
    pub artifact_dir: String,
    pub cache: String,
    #[serde(default)]
    pub profiles: BTreeMap<String, Profile>,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublishConfig {
    #[serde(default)]
    pub targets: Vec<PublishTarget>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PublishTarget {
    pub kind: String,
    pub path: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    pub project: Metadata,
    pub sources: Sources,
    pub build: BuildConfig,
    #[serde(default)]
    pub publish: PublishConfig,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostic {
    pub code: String,
    pub kind: String,
    pub message: String,
    pub source: String,
    pub line: usize,
    pub column: usize,
    pub table: Option<String>,
    pub occurrence: Option<usize>,
    pub field_path: Vec<String>,
    pub generation: u64,
}
impl Diagnostic {
    pub fn error(source: &str, e: &Error, generation: u64) -> Self {
        Self {
            code: e.code.into(),
            kind: "error".into(),
            message: e.message.clone(),
            source: source.into(),
            line: 1,
            column: 1,
            table: None,
            occurrence: None,
            field_path: vec![],
            generation,
        }
    }
}
#[derive(Clone, Debug)]
pub struct Source {
    pub path: String,
    pub physical: PathBuf,
    pub bytes: Arc<str>,
    pub document: Option<Arc<Document>>,
    pub error: Option<String>,
    pub identity: String,
    pub kind: Option<String>,
    pub binding: Option<String>,
}
#[derive(Clone, Debug)]
pub struct Project {
    pub root: PathBuf,
    pub config: Config,
    pub config_bytes: Arc<str>,
    pub config_identity: String,
    pub roots: Vec<PathBuf>,
    pub sources: BTreeMap<String, Arc<Source>>,
    pub folders: BTreeSet<String>,
    pub tables: BTreeMap<String, Arc<Table>>,
    pub types: Arc<Types>,
    pub type_sources: BTreeMap<String, String>,
    pub type_declarations: BTreeMap<String, BTreeSet<String>>,
    pub generation: u64,
    pub declaration_problems: Vec<Diagnostic>,
}
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Row {
    pub source: String,
    pub occurrence: usize,
    pub values: Vec<(String, Typed)>,
    #[serde(skip)]
    pub key: Vec<KeyPart>,
    #[serde(skip)]
    pub tags: Vec<String>,
}

pub type Dataset = BTreeMap<String, Vec<Row>>;
pub type Validation = (Vec<Diagnostic>, Dataset);
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedReference {
    pub target_table: String,
    pub target_fields: Vec<String>,
    pub multi: bool,
    pub optional: bool,
}

pub fn discover(explicit: Option<&Path>, cwd: &Path) -> Result<PathBuf> {
    crate::instrument::count(crate::instrument::Kind::Discovery);
    if let Some(p) = explicit {
        let p = if p.is_absolute() {
            p.to_path_buf()
        } else {
            cwd.join(p)
        };
        let root = if p.is_file() {
            if p.file_name().and_then(|p| p.to_str()) != Some("masterdata.toml") {
                return Err(Error::new(
                    "E-PROJECT-NOT-FOUND",
                    "config filename must be masterdata.toml",
                ));
            }
            p.parent().unwrap().to_path_buf()
        } else {
            p
        };
        return if root.join("masterdata.toml").is_file() {
            Ok(root)
        } else {
            Err(Error::new(
                "E-PROJECT-NOT-FOUND",
                root.display().to_string(),
            ))
        };
    }
    for p in cwd.ancestors() {
        if p.join("masterdata.toml").is_file() {
            return Ok(p.to_path_buf());
        }
    }
    Err(Error::new(
        "E-PROJECT-NOT-FOUND",
        "masterdata.toml was not found",
    ))
}
pub fn config(bytes: &str) -> Result<Config> {
    let value: toml::Value =
        toml::from_str(bytes).map_err(|e| Error::new("E-CONFIG", e.to_string()))?;
    if value.get("build").and_then(|v| v.get("output")).is_some() {
        return Err(Error::new(
            "E-CONFIG-LEGACY-BUILD-OUTPUT",
            "build.output requires explicit migration to build.artifact_dir or a csharp publish target",
        ));
    }
    if value
        .get("build")
        .and_then(|v| v.get("binary_output"))
        .is_some()
    {
        return Err(Error::new(
            "E-CONFIG-LEGACY-BINARY-OUTPUT",
            "build.binary_output requires explicit migration to canonical masterdata.bytes or a binary publish target",
        ));
    }
    let c: Config = value
        .try_into()
        .map_err(|e| Error::new("E-CONFIG", e.to_string()))?;
    if [&c.project.id, &c.project.name, &c.project.version]
        .iter()
        .any(|s| s.trim().is_empty())
        || c.sources.roots.is_empty()
        || c.sources.roots.iter().any(|s| s.trim().is_empty())
        || c.build.artifact_dir.trim().is_empty()
        || c.build.cache.trim().is_empty()
    {
        return Err(Error::new(
            "E-CONFIG",
            "metadata and configured paths must not be empty",
        ));
    }
    relative_safe(&c.build.artifact_dir)?;
    for (name, p) in &c.build.profiles {
        if !semantic::kebab(name)
            || !valid_tags(&p.include_tags)
            || !valid_tags(&p.exclude_tags)
            || p.include_tags.iter().any(|s| p.exclude_tags.contains(s))
        {
            return Err(Error::new(
                "E-CONFIG-PROFILE",
                format!("invalid profile {name}"),
            ));
        }
    }
    for t in &c.publish.targets {
        if !matches!(t.kind.as_str(), "csharp" | "binary") || t.path.trim().is_empty() {
            return Err(Error::new("E-CONFIG-PUBLISH", "invalid target kind/path"));
        }
    }
    Ok(c)
}
pub fn relative_safe(s: &str) -> Result<PathBuf> {
    let p = PathBuf::from(s);
    if p.is_absolute()
        || p.as_os_str().is_empty()
        || p.components().any(|c| {
            !matches!(
                c,
                std::path::Component::Normal(_) | std::path::Component::CurDir
            )
        })
    {
        return Err(Error::new(
            "E-PATH-SCOPE",
            format!("unsafe relative path {s}"),
        ));
    }
    Ok(p)
}
pub fn valid_tags(tags: &[String]) -> bool {
    tags.iter().all(|s| semantic::kebab(s))
        && tags.iter().collect::<BTreeSet<_>>().len() == tags.len()
}
pub fn record_tags(n: &crate::source::Node) -> Result<Vec<String>> {
    let Some(n) = n.get("$tags") else {
        return Ok(vec![]);
    };
    let tags = n
        .items()?
        .iter()
        .map(|i| i.value.text().map(str::to_owned))
        .collect::<Result<Vec<_>>>()?;
    if !valid_tags(&tags) {
        return Err(Error::new("E-RECORD-TAGS", "invalid / duplicate tags"));
    }
    Ok(tags)
}
pub fn selected(tags: &[String], profile: &Profile) -> bool {
    (profile.include_tags.is_empty() || tags.iter().any(|s| profile.include_tags.contains(s)))
        && !tags.iter().any(|s| profile.exclude_tags.contains(s))
}

impl Project {
    pub fn open(path: &Path) -> Result<Self> {
        let root = discover(Some(path), &std::env::current_dir().map_err(io_error)?)?
            .canonicalize()
            .map_err(io_error)?;
        let config_bytes: Arc<str> = fs::read_to_string(root.join("masterdata.toml"))
            .map_err(io_error)?
            .into();
        let config = config(&config_bytes)?;
        let roots = config
            .sources
            .roots
            .iter()
            .map(|p| {
                let path = root.join(p);
                path.canonicalize().map_err(io_error)
            })
            .collect::<Result<Vec<_>>>()?;
        let mut paths = BTreeSet::new();
        let mut folders = BTreeSet::new();
        for source_root in &roots {
            enumerate(source_root, &mut paths, &mut folders)?;
        }
        let mut sources = BTreeMap::new();
        for path in paths {
            crate::instrument::count(crate::instrument::Kind::ProjectParse);
            let logical = path
                .strip_prefix(&root)
                .map_err(|_| {
                    Error::new(
                        "E-PATH-SCOPE",
                        "source must have project-relative provenance",
                    )
                })?
                .to_string_lossy()
                .replace('\\', "/");
            sources.insert(
                logical.clone(),
                Arc::new(read_source(&root, &roots, &logical)?),
            );
        }
        let folders = folders
            .into_iter()
            .filter_map(|path| {
                path.strip_prefix(&root)
                    .ok()
                    .map(|p| p.to_string_lossy().replace('\\', "/"))
            })
            .collect();
        let mut p = Self {
            root,
            config,
            config_identity: content_identity(config_bytes.as_bytes()),
            config_bytes,
            roots,
            sources,
            folders,
            tables: BTreeMap::new(),
            types: Arc::new(BTreeMap::new()),
            type_sources: BTreeMap::new(),
            type_declarations: BTreeMap::new(),
            generation: 1,
            declaration_problems: vec![],
        };
        p.rebuild_declarations();
        Ok(p)
    }

    pub fn rebuild_declarations(&mut self) {
        self.tables.clear();
        self.type_sources.clear();
        self.type_declarations.clear();
        self.declaration_problems.clear();
        let mut types = BTreeMap::new();
        for (path, s) in &self.sources {
            if let Some(doc) = &s.document {
                if s.kind.as_deref() == Some("type")
                    && let Some(name) = doc.root.get("name").and_then(|node| node.text().ok())
                {
                    self.type_declarations
                        .entry(name.into())
                        .or_default()
                        .insert(path.clone());
                }
                for (span, message) in &doc.subset_issues {
                    let mut d = Diagnostic::error(
                        path,
                        &Error::new("E-YAML-SUBSET", message),
                        self.generation,
                    );
                    locate(&mut d, doc, span.start);
                    self.declaration_problems.push(d);
                }
                let result = match s.kind.as_deref() {
                    Some("schema") => semantic::parse_table(doc, path).and_then(|t| {
                        if self.tables.insert(t.name.clone(), Arc::new(t)).is_some() {
                            Err(Error::new("E-TABLE-DUPLICATE", "multiple schemas"))
                        } else {
                            Ok(())
                        }
                    }),
                    Some("type") => semantic::parse_type(doc).and_then(|(name, t)| {
                        self.type_sources.insert(name.clone(), path.clone());
                        if types.insert(name, t).is_some() {
                            Err(Error::new("E-TYPE-DUPLICATE", "duplicate type"))
                        } else {
                            Ok(())
                        }
                    }),
                    Some("data") => {
                        semantic::reject_unknown(&doc.root, &["kind", "table", "records"]).and_then(
                            |_| {
                                let table = semantic::text(&doc.root, "table")?;
                                if !semantic::kebab(&table) {
                                    return Err(Error::new("E-TABLE-NAME", table));
                                }
                                doc.records()?;
                                Ok(())
                            },
                        )
                    }
                    _ => Err(Error::new(
                        "E-DOCUMENT-KIND",
                        "kind must be schema, data or type",
                    )),
                };
                if let Err(e) = result {
                    self.declaration_problems
                        .push(Diagnostic::error(path, &e, self.generation));
                }
            } else {
                self.declaration_problems.push(Diagnostic::error(
                    path,
                    &Error::new("E-YAML-PARSE", s.error.clone().unwrap_or_default()),
                    self.generation,
                ));
            }
        }
        self.types = Arc::new(types);
        self.validate_declarations();
    }

    fn validate_declarations(&mut self) {
        let mut names: BTreeSet<_> = self.types.keys().cloned().collect();
        for (name, t) in self.types.iter() {
            if matches!(t, Type::ValueObject { .. })
                && matches!(
                    name.as_str(),
                    "Value" | "Equals" | "GetHashCode" | "ToString" | "CompareTo"
                )
            {
                self.declaration_problems.push(Diagnostic::error(
                    &self.type_sources[name],
                    &Error::new("E-CSHARP-COLLISION", name),
                    self.generation,
                ));
            }
            if let Type::Custom { fields } = t {
                for f in fields {
                    if let Err(e) = semantic::shape(f, &self.types) {
                        self.declaration_problems.push(Diagnostic::error(
                            &self.type_sources[name],
                            &e,
                            self.generation,
                        ));
                    }
                    let p = semantic::public_name(&f.name);
                    if p == *name
                        || matches!(p.as_str(), "Equals" | "GetHashCode" | "ToString")
                        || csharp_keyword(&f.name)
                    {
                        self.declaration_problems.push(Diagnostic::error(
                            &self.type_sources[name],
                            &Error::new("E-CSHARP-COLLISION", &f.name),
                            self.generation,
                        ));
                    }
                }
            }
        }
        for t in self.tables.values() {
            if !names.insert(t.csharp_name.clone()) {
                self.declaration_problems.push(Diagnostic::error(
                    &t.source,
                    &Error::new("E-CSHARP-COLLISION", &t.csharp_name),
                    self.generation,
                ));
            }
            for f in &t.fields {
                if let Err(e) = semantic::shape(f, &self.types) {
                    self.declaration_problems.push(Diagnostic::error(
                        &t.source,
                        &e,
                        self.generation,
                    ));
                }
                if semantic::public_name(&f.name) == t.csharp_name || csharp_keyword(&f.name) {
                    self.declaration_problems.push(Diagnostic::error(
                        &t.source,
                        &Error::new("E-CSHARP-COLLISION", &f.name),
                        self.generation,
                    ));
                }
            }
            for k in std::iter::once(&t.primary).chain(&t.secondary) {
                for name in &k.fields {
                    if let Some(f) = t.fields.iter().find(|f| &f.name == name)
                        && !semantic::key_capable(f, &self.types)
                    {
                        self.declaration_problems.push(Diagnostic::error(
                            &t.source,
                            &Error::new("E-KEY-CAPABILITY", name),
                            self.generation,
                        ));
                    }
                }
            }
            let mut helper_names: BTreeSet<_> = t
                .fields
                .iter()
                .map(|f| semantic::public_name(&f.name))
                .collect();
            for r in &t.references {
                let Some(target) = self.tables.get(&r.target_table) else {
                    self.declaration_problems.push(Diagnostic::error(
                        &t.source,
                        &Error::new("E-REFERENCE-TARGET", &r.target_table),
                        self.generation,
                    ));
                    continue;
                };
                let valid = self.resolve_reference(t, r).is_ok()
                    && helper_names.insert(r.csharp_name.clone());
                let _ = target;
                if !valid {
                    self.declaration_problems.push(Diagnostic::error(
                        &t.source,
                        &Error::new("E-REFERENCE-SHAPE", &r.name),
                        self.generation,
                    ));
                }
            }
        }
        for (path, s) in &self.sources {
            if s.kind.as_deref() == Some("data")
                && s.binding
                    .as_ref()
                    .is_none_or(|t| !self.tables.contains_key(t))
            {
                self.declaration_problems.push(Diagnostic::error(
                    path,
                    &Error::new("E-TABLE-MISSING", "data schema not found"),
                    self.generation,
                ));
            }
        }
    }

    pub fn resolve_reference(
        &self,
        table: &Table,
        reference: &Reference,
    ) -> Result<ResolvedReference> {
        let target = self
            .tables
            .get(&reference.target_table)
            .ok_or_else(|| Error::new("E-REFERENCE-TARGET", &reference.target_table))?;
        let source_fields = reference
            .fields
            .iter()
            .filter_map(|name| table.fields.iter().find(|field| &field.name == name))
            .collect::<Vec<_>>();
        let target_fields = reference
            .target_fields
            .iter()
            .filter_map(|name| target.fields.iter().find(|field| &field.name == name))
            .collect::<Vec<_>>();
        let key = std::iter::once(&target.primary)
            .chain(&target.secondary)
            .find(|key| key.fields == reference.target_fields);
        let valid = !source_fields.is_empty()
            && self
                .sources
                .values()
                .filter(|source| {
                    source.kind.as_deref() == Some("schema")
                        && source.binding.as_deref() == Some(&reference.target_table)
                })
                .count()
                == 1
            && source_fields.len() == reference.fields.len()
            && source_fields.len() == target_fields.len()
            && target_fields.len() == reference.target_fields.len()
            && reference.fields.iter().collect::<BTreeSet<_>>().len() == reference.fields.len()
            && reference
                .target_fields
                .iter()
                .collect::<BTreeSet<_>>()
                .len()
                == reference.target_fields.len()
            && key.is_some()
            && source_fields.iter().all(|field| {
                !field.array && field.nullable == source_fields[0].nullable && {
                    let mut required = (*field).clone();
                    required.nullable = false;
                    semantic::key_capable(&required, &self.types)
                }
            })
            && target_fields
                .iter()
                .all(|field| semantic::key_capable(field, &self.types))
            && source_fields
                .iter()
                .zip(&target_fields)
                .all(|(source, target)| source.type_name == target.type_name)
            && !table
                .fields
                .iter()
                .any(|field| semantic::public_name(&field.name) == reference.csharp_name)
            && reference.csharp_name != table.csharp_name
            && table
                .references
                .iter()
                .filter(|r| r.csharp_name == reference.csharp_name)
                .count()
                == 1;
        if !valid {
            return Err(Error::new("E-REFERENCE-SHAPE", &reference.name));
        }
        Ok(ResolvedReference {
            target_table: target.name.clone(),
            target_fields: reference.target_fields.clone(),
            multi: key.unwrap().non_unique,
            optional: source_fields[0].nullable,
        })
    }

    pub fn record_sources(&self, table: &str) -> Vec<String> {
        self.sources
            .iter()
            .filter(|(_, s)| {
                s.binding.as_deref() == Some(table)
                    && s.document
                        .as_ref()
                        .is_some_and(|d| d.root.get("records").is_some())
            })
            .map(|(p, _)| p.clone())
            .collect()
    }

    pub fn dependencies(&self, table: &Table) -> Vec<String> {
        let mut paths = BTreeSet::from([table.source.clone()]);
        let mut seen = BTreeSet::new();
        for f in &table.fields {
            Self::collect_type_sources(&f.type_name, self, &mut paths, &mut seen);
        }
        paths.into_iter().collect()
    }
    pub fn type_dependencies(&self, name: &str) -> Vec<String> {
        let mut paths = BTreeSet::new();
        Self::collect_type_sources(name, self, &mut paths, &mut BTreeSet::new());
        paths.into_iter().collect()
    }
    fn collect_type_sources(
        name: &str,
        p: &Project,
        set: &mut BTreeSet<String>,
        seen: &mut BTreeSet<String>,
    ) {
        if !seen.insert(name.into()) {
            return;
        }
        if let Some(path) = p.type_sources.get(name) {
            set.insert(path.clone());
        }
        if let Some(Type::Custom { fields }) = p.types.get(name) {
            for f in fields {
                Self::collect_type_sources(&f.type_name, p, set, seen);
            }
        }
    }

    pub fn profile(&self, name: Option<&str>) -> Result<Profile> {
        name.map(|name| {
            self.config
                .build
                .profiles
                .get(name)
                .cloned()
                .ok_or_else(|| Error::new("E-PROFILE-NOT-FOUND", name))
        })
        .unwrap_or_else(|| Ok(Profile::default()))
    }

    pub fn validate(&self, profile: Option<&str>) -> Result<Validation> {
        crate::instrument::count(crate::instrument::Kind::Validation);
        let profile = self.profile(profile)?;
        let mut diagnostics = self.declaration_problems.clone();
        let mut rows: BTreeMap<String, Vec<Row>> =
            self.tables.keys().map(|n| (n.clone(), vec![])).collect();
        for (path, s) in &self.sources {
            let Some(doc) = &s.document else {
                continue;
            };
            let Some(table) = s.binding.as_ref().and_then(|n| self.tables.get(n)) else {
                continue;
            };
            let Some(record_node) = doc.root.get("records") else {
                continue;
            };
            let records = match record_node.items() {
                Ok(r) => r,
                Err(e) => {
                    diagnostics.push(Diagnostic::error(path, &e, self.generation));
                    continue;
                }
            };
            for (i, item) in records.iter().enumerate() {
                let mut values = Vec::new();
                let mut valid = true;
                let mut push = |message: String, code: &str, field_path: Vec<String>| {
                    valid = false;
                    let mut d =
                        Diagnostic::error(path, &Error::new("E-VALUE", message), self.generation);
                    d.code = code.into();
                    d.table = Some(table.name.clone());
                    d.occurrence = Some(i + 1);
                    d.field_path = field_path;
                    locate(&mut d, doc, item.value.span.start);
                    diagnostics.push(d);
                };
                if let Raw::Mapping(m) = &item.value.raw {
                    for member in m {
                        if member.name != "$tags"
                            && !table.fields.iter().any(|f| f.name == member.name)
                        {
                            push(
                                format!("unknown field {}", member.name),
                                "E-VALUE-UNKNOWN",
                                vec![member.name.clone()],
                            );
                        }
                    }
                } else {
                    push("record must be a mapping".into(), "E-RECORD-SHAPE", vec![]);
                }
                for f in &table.fields {
                    let result = item
                        .value
                        .get(&f.name)
                        .ok_or_else(|| semantic::ValueProblem {
                            code: "E-VALUE-MISSING".into(),
                            message: "field entry required".into(),
                            path: vec![],
                        })
                        .and_then(|n| semantic::interpret(f, n, &self.types));
                    match result {
                        Ok(v) => values.push((f.name.clone(), v)),
                        Err(e) => {
                            let mut path = vec![f.name.clone()];
                            path.extend(e.path);
                            push(e.message, &e.code, path);
                        }
                    }
                }
                let tags = match record_tags(&item.value) {
                    Ok(tags) => tags,
                    Err(e) => {
                        push(e.message, e.code, vec!["$tags".into()]);
                        vec![]
                    }
                };
                if valid && selected(&tags, &profile) {
                    let key = match record_key(&values, table, &table.primary.fields, &self.types) {
                        Ok(key) => key,
                        Err(e) => {
                            diagnostics.push(Diagnostic::error(path, &e, self.generation));
                            continue;
                        }
                    };
                    rows.get_mut(&table.name).unwrap().push(Row {
                        source: path.clone(),
                        occurrence: i + 1,
                        values,
                        key,
                        tags,
                    });
                }
            }
        }
        for (name, records) in &mut rows {
            let table = &self.tables[name];
            records.sort_by(|a, b| a.key.cmp(&b.key));
            for key in std::iter::once(&table.primary)
                .chain(table.secondary.iter().filter(|k| !k.non_unique))
            {
                let mut seen = BTreeSet::new();
                for row in records.iter() {
                    let k = match record_key(&row.values, table, &key.fields, &self.types) {
                        Ok(k) => k,
                        Err(e) => {
                            diagnostics.push(row_diagnostic(
                                row,
                                name,
                                self.generation,
                                e.code,
                                e.message,
                            ));
                            continue;
                        }
                    };
                    if !seen.insert(k) {
                        diagnostics.push(row_diagnostic(
                            row,
                            name,
                            self.generation,
                            "E-KEY-DUPLICATE",
                            format!("duplicate {}", semantic::query_name(&key.fields)),
                        ));
                    }
                }
            }
        }
        for (name, records) in &rows {
            let table = &self.tables[name];
            for r in &table.references {
                if r.fields.is_empty()
                    || r.fields
                        .iter()
                        .any(|n| !table.fields.iter().any(|f| &f.name == n))
                {
                    continue;
                }
                let Some(target) = self.tables.get(&r.target_table) else {
                    continue;
                };
                if !std::iter::once(&target.primary)
                    .chain(&target.secondary)
                    .any(|k| k.fields == r.target_fields)
                {
                    continue;
                }
                let mut target_keys = BTreeSet::new();
                for row in &rows[&r.target_table] {
                    if let Ok(key) = record_key(&row.values, target, &r.target_fields, &self.types)
                    {
                        target_keys.insert(key);
                    }
                }
                for row in records {
                    let values = r
                        .fields
                        .iter()
                        .map(|f| &row.values.iter().find(|(n, _)| n == f).unwrap().1)
                        .collect::<Vec<_>>();
                    let nulls = values.iter().filter(|v| matches!(v, Typed::Null)).count();
                    if nulls == values.len() {
                        continue;
                    }
                    if nulls > 0 {
                        diagnostics.push(row_diagnostic(
                            row,
                            name,
                            self.generation,
                            "E-REFERENCE-PARTIAL-NULL",
                            r.name.clone(),
                        ));
                        continue;
                    }
                    let mut required = table.as_ref().clone();
                    for f in &mut required.fields {
                        f.nullable = false;
                    }
                    let key = match record_key(&row.values, &required, &r.fields, &self.types) {
                        Ok(key) => key,
                        Err(e) => {
                            diagnostics.push(row_diagnostic(
                                row,
                                name,
                                self.generation,
                                e.code,
                                e.message,
                            ));
                            continue;
                        }
                    };
                    if !target_keys.contains(&key) {
                        diagnostics.push(row_diagnostic(
                            row,
                            name,
                            self.generation,
                            "E-REFERENCE-MISSING",
                            r.name.clone(),
                        ));
                    }
                }
            }
        }
        Ok((diagnostics, rows))
    }
}

pub fn record_key(
    values: &[(String, Typed)],
    table: &Table,
    fields: &[String],
    types: &Types,
) -> Result<Vec<KeyPart>> {
    fields
        .iter()
        .map(|name| {
            let f = table
                .fields
                .iter()
                .find(|f| &f.name == name)
                .ok_or_else(|| Error::new("E-KEY-SHAPE", "unknown component"))?;
            let v = &values
                .iter()
                .find(|(n, _)| n == name)
                .ok_or_else(|| Error::new("E-KEY-VALUE", "missing component"))?
                .1;
            semantic::key_part(v, f, types)
        })
        .collect()
}
fn row_diagnostic(
    row: &Row,
    table: &str,
    generation: u64,
    code: &str,
    message: String,
) -> Diagnostic {
    Diagnostic {
        code: code.into(),
        kind: "error".into(),
        message,
        source: row.source.clone(),
        line: 0,
        column: 0,
        table: Some(table.into()),
        occurrence: Some(row.occurrence),
        field_path: vec![],
        generation,
    }
}
pub fn locate(d: &mut Diagnostic, doc: &Document, at: usize) {
    d.line = doc.point(at).row + 1;
    d.column = doc.column(at) + 1;
}
pub fn io_error(e: std::io::Error) -> Error {
    Error::new("E-IO", e.to_string())
}

pub fn read_source(root: &Path, roots: &[PathBuf], logical: &str) -> Result<Source> {
    let p = root.join(relative_safe(logical)?);
    let physical = p.canonicalize().map_err(io_error)?;
    if !roots.iter().any(|r| physical.starts_with(r)) {
        return Err(Error::new(
            "E-PATH-SCOPE",
            "source escaped configured roots",
        ));
    }
    let bytes: Arc<str> = fs::read_to_string(&p).map_err(io_error)?.into();
    let parsed = Document::parse(bytes.clone());
    let (document, error) = match parsed {
        Ok(d) => (Some(Arc::new(d)), None),
        Err(e) => (None, Some(e.to_string())),
    };
    let kind = document
        .as_ref()
        .and_then(|d| d.root.get("kind"))
        .and_then(|n| n.text().ok())
        .map(str::to_owned);
    let binding = document
        .as_ref()
        .and_then(|d| d.root.get("table"))
        .and_then(|n| n.text().ok())
        .map(str::to_owned);
    Ok(Source {
        path: logical.into(),
        physical,
        identity: content_identity(bytes.as_bytes()),
        bytes,
        document,
        error,
        kind,
        binding,
    })
}
pub(crate) fn enumerate(
    root: &Path,
    paths: &mut BTreeSet<PathBuf>,
    folders: &mut BTreeSet<PathBuf>,
) -> Result<()> {
    crate::instrument::count(crate::instrument::Kind::Enumeration);
    folders.insert(root.to_path_buf());
    for entry in fs::read_dir(root).map_err(io_error)? {
        let entry = entry.map_err(io_error)?;
        let kind = entry.file_type().map_err(io_error)?;
        if kind.is_symlink() {
            continue;
        }
        if kind.is_dir() {
            enumerate(&entry.path(), paths, folders)?;
        } else if kind.is_file()
            && entry
                .path()
                .extension()
                .and_then(|s| s.to_str())
                .is_some_and(|s| matches!(s, "yaml" | "yml"))
        {
            paths.insert(entry.path());
        }
    }
    Ok(())
}
pub fn csharp_keyword(s: &str) -> bool {
    matches!(
        s,
        "abstract"
            | "as"
            | "base"
            | "bool"
            | "break"
            | "byte"
            | "case"
            | "catch"
            | "char"
            | "checked"
            | "class"
            | "const"
            | "continue"
            | "decimal"
            | "default"
            | "delegate"
            | "do"
            | "double"
            | "else"
            | "enum"
            | "event"
            | "explicit"
            | "extern"
            | "false"
            | "finally"
            | "fixed"
            | "float"
            | "for"
            | "foreach"
            | "goto"
            | "if"
            | "implicit"
            | "in"
            | "int"
            | "interface"
            | "internal"
            | "is"
            | "lock"
            | "long"
            | "namespace"
            | "new"
            | "null"
            | "object"
            | "operator"
            | "out"
            | "override"
            | "params"
            | "private"
            | "protected"
            | "public"
            | "readonly"
            | "ref"
            | "return"
            | "sbyte"
            | "sealed"
            | "short"
            | "sizeof"
            | "stackalloc"
            | "static"
            | "string"
            | "struct"
            | "switch"
            | "this"
            | "throw"
            | "true"
            | "try"
            | "typeof"
            | "uint"
            | "ulong"
            | "unchecked"
            | "unsafe"
            | "ushort"
            | "using"
            | "virtual"
            | "void"
            | "volatile"
            | "while"
    )
}
