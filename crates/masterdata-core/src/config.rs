use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{ErrorKind, MasterdataError, Result};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ProjectConfig {
    pub project: ProjectMetadata,
    #[serde(default)]
    pub sources: SourceConfig,
    #[serde(default)]
    pub build: BuildConfig,
    #[serde(default, skip_serializing_if = "PublishConfig::is_empty")]
    pub publish: PublishConfig,
}

impl<'de> Deserialize<'de> for ProjectConfig {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RawProjectConfig {
            project: ProjectMetadata,
            #[serde(default)]
            sources: SourceConfig,
            #[serde(default)]
            build: RawBuildConfig,
            #[serde(default)]
            publish: PublishConfig,
        }

        #[derive(Deserialize)]
        struct RawBuildConfig {
            #[serde(default = "default_artifact_directory")]
            artifact_dir: String,
            #[serde(default = "default_cache_directory")]
            cache: String,
            #[serde(default)]
            output: Option<toml::Value>,
            #[serde(default)]
            binary_output: Option<toml::Value>,
            #[serde(default)]
            profiles: BTreeMap<String, BuildProfile>,
        }

        impl Default for RawBuildConfig {
            fn default() -> Self {
                Self {
                    artifact_dir: default_artifact_directory(),
                    cache: default_cache_directory(),
                    output: None,
                    binary_output: None,
                    profiles: BTreeMap::new(),
                }
            }
        }

        let raw = RawProjectConfig::deserialize(deserializer)?;
        if raw.build.output.is_some() {
            return Err(serde::de::Error::custom(
                "legacy build.output is not supported; use build.artifact_dir or a csharp publish target",
            ));
        }
        if raw.build.binary_output.is_some() {
            return Err(serde::de::Error::custom(
                "legacy build.binary_output is not supported; use the canonical binary or a binary publish target",
            ));
        }
        Ok(Self {
            project: raw.project,
            sources: raw.sources,
            build: BuildConfig {
                artifact_dir: raw.build.artifact_dir,
                cache: raw.build.cache,
                profiles: raw.build.profiles,
            },
            publish: raw.publish,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectMetadata {
    pub id: String,
    pub name: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SourceConfig {
    #[serde(default = "default_source_roots")]
    pub roots: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BuildConfig {
    #[serde(default = "default_artifact_directory")]
    pub artifact_dir: String,
    #[serde(default = "default_cache_directory")]
    pub cache: String,
    #[serde(default)]
    pub profiles: BTreeMap<String, BuildProfile>,
}

/// A named, project-scoped Build Selection.  The collection order is source
/// formatting only; selection resolves both members as sets.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(deny_unknown_fields)]
pub struct BuildProfile {
    #[serde(default)]
    pub include_tags: Vec<String>,
    #[serde(default)]
    pub exclude_tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct PublishConfig {
    #[serde(default)]
    pub targets: Vec<PublishTarget>,
}

impl PublishConfig {
    pub fn is_empty(&self) -> bool {
        self.targets.is_empty()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PublishTarget {
    pub kind: PublishTargetKind,
    pub path: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PublishTargetKind {
    CSharp,
    Binary,
}

impl Default for SourceConfig {
    fn default() -> Self {
        Self {
            roots: default_source_roots(),
        }
    }
}

impl Default for BuildConfig {
    fn default() -> Self {
        Self {
            artifact_dir: default_artifact_directory(),
            cache: default_cache_directory(),
            profiles: BTreeMap::new(),
        }
    }
}

impl ProjectConfig {
    pub fn validate(&self) -> Result<()> {
        let fields = [
            ("project.id", self.project.id.trim()),
            ("project.name", self.project.name.trim()),
            ("project.version", self.project.version.trim()),
        ];
        for (field, value) in fields {
            if value.is_empty() {
                return Err(MasterdataError::new(
                    "E-PROJECT-CONFIG-EMPTY",
                    ErrorKind::Config,
                    format!("{field} must not be empty"),
                ));
            }
        }
        if self.sources.roots.is_empty() {
            return Err(MasterdataError::new(
                "E-PROJECT-CONFIG-NO-SOURCE-ROOT",
                ErrorKind::Config,
                "sources.roots must contain at least one source root",
            ));
        }
        if self.sources.roots.iter().any(|root| root.trim().is_empty()) {
            return Err(MasterdataError::new(
                "E-PROJECT-CONFIG-EMPTY-SOURCE-ROOT",
                ErrorKind::Config,
                "sources.roots must not contain an empty path",
            ));
        }
        if self.build.artifact_dir.trim().is_empty() || self.build.cache.trim().is_empty() {
            return Err(MasterdataError::new(
                "E-PROJECT-CONFIG-EMPTY-BUILD-PATH",
                ErrorKind::Config,
                "build.artifact_dir and build.cache must not be empty",
            ));
        }
        if let Some(target) = self
            .publish
            .targets
            .iter()
            .find(|target| target.path.trim().is_empty())
        {
            return Err(MasterdataError::new(
                "E-PROJECT-CONFIG-EMPTY-PUBLISH-PATH",
                ErrorKind::Config,
                format!(
                    "publish target path for {:?} must not be empty",
                    target.kind
                ),
            ));
        }
        for (name, profile) in &self.build.profiles {
            if !is_profile_or_tag_name(name) {
                return Err(MasterdataError::new(
                    "E-BUILD-PROFILE-INVALID-NAME",
                    ErrorKind::Config,
                    format!("Build Profile name `{name}` is not lowercase kebab-case"),
                ));
            }
            validate_profile_tags(name, "include_tags", &profile.include_tags)?;
            validate_profile_tags(name, "exclude_tags", &profile.exclude_tags)?;
            if profile
                .include_tags
                .iter()
                .any(|tag| profile.exclude_tags.iter().any(|other| other == tag))
            {
                return Err(MasterdataError::new(
                    "E-BUILD-PROFILE-TAG-OVERLAP",
                    ErrorKind::Config,
                    format!("Build Profile `{name}` includes and excludes the same tag"),
                ));
            }
        }
        Ok(())
    }
}

fn validate_profile_tags(profile: &str, member: &str, tags: &[String]) -> Result<()> {
    let mut seen = std::collections::BTreeSet::new();
    for tag in tags {
        if !is_profile_or_tag_name(tag) {
            return Err(MasterdataError::new(
                "E-BUILD-PROFILE-INVALID-TAG",
                ErrorKind::Config,
                format!("Build Profile `{profile}` {member} contains invalid tag `{tag}`"),
            ));
        }
        if !seen.insert(tag) {
            return Err(MasterdataError::new(
                "E-BUILD-PROFILE-DUPLICATE-TAG",
                ErrorKind::Config,
                format!("Build Profile `{profile}` {member} contains duplicate tag `{tag}`"),
            ));
        }
    }
    Ok(())
}

fn is_profile_or_tag_name(value: &str) -> bool {
    let mut segments = value.split('-');
    let Some(first) = segments.next() else {
        return false;
    };
    !first.is_empty()
        && first.as_bytes()[0].is_ascii_lowercase()
        && first
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        && segments.all(|segment| {
            !segment.is_empty()
                && segment
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
        })
}

fn default_source_roots() -> Vec<String> {
    vec!["sources".to_owned()]
}

fn default_artifact_directory() -> String {
    ".masterdata/output".to_owned()
}

fn default_cache_directory() -> String {
    ".masterdata/cache".to_owned()
}
