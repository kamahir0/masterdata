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
        }

        impl Default for RawBuildConfig {
            fn default() -> Self {
                Self {
                    artifact_dir: default_artifact_directory(),
                    cache: default_cache_directory(),
                    output: None,
                    binary_output: None,
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
        Ok(())
    }
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
