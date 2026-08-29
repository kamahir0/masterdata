use serde::{Deserialize, Serialize};

use crate::{ErrorKind, MasterdataError, Result};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectConfig {
    pub project: ProjectMetadata,
    #[serde(default)]
    pub sources: SourceConfig,
    #[serde(default)]
    pub build: BuildConfig,
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
    #[serde(default = "default_output_directory")]
    pub output: String,
    #[serde(default = "default_cache_directory")]
    pub cache: String,
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
            output: default_output_directory(),
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
                    "PROJECT-CONFIG-001",
                    ErrorKind::Config,
                    format!("{field} must not be empty"),
                ));
            }
        }
        if self.sources.roots.is_empty() {
            return Err(MasterdataError::new(
                "PROJECT-CONFIG-002",
                ErrorKind::Config,
                "sources.roots must contain at least one source root",
            ));
        }
        if self.sources.roots.iter().any(|root| root.trim().is_empty()) {
            return Err(MasterdataError::new(
                "PROJECT-CONFIG-003",
                ErrorKind::Config,
                "sources.roots must not contain an empty path",
            ));
        }
        if self.build.output.trim().is_empty() || self.build.cache.trim().is_empty() {
            return Err(MasterdataError::new(
                "PROJECT-CONFIG-004",
                ErrorKind::Config,
                "build.output and build.cache must not be empty",
            ));
        }
        Ok(())
    }
}

fn default_source_roots() -> Vec<String> {
    vec!["sources".to_owned()]
}

fn default_output_directory() -> String {
    ".masterdata/generated".to_owned()
}

fn default_cache_directory() -> String {
    ".masterdata/cache".to_owned()
}
