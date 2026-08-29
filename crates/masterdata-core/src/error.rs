use std::fmt::{Display, Formatter};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

pub type Result<T> = std::result::Result<T, MasterdataError>;

/// Stable-enough categories for adapters such as the GUI to render errors.
/// More detailed error codes are carried by [`Diagnostic::code`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorKind {
    Io,
    ProjectNotFound,
    Config,
    Parse,
    Validation,
    ExternalTool,
    NotImplemented,
    Usage,
}

/// A structured diagnostic that can grow without forcing the GUI to parse a
/// human-readable string.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Diagnostic {
    pub code: String,
    pub kind: ErrorKind,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub column: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub record_identity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
    /// Normative requirement IDs related to this runtime diagnostic. These are
    /// references, not diagnostic codes and do not make the diagnostic an
    /// owner of the requirement.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub related_requirements: Vec<String>,
}

impl Diagnostic {
    pub fn new(code: impl Into<String>, kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            kind,
            message: message.into(),
            source: None,
            line: None,
            column: None,
            schema_path: None,
            record_identity: None,
            suggestion: None,
            related_requirements: Vec::new(),
        }
    }

    pub fn with_source(mut self, source: impl Into<PathBuf>) -> Self {
        self.source = Some(source.into());
        self
    }

    pub fn with_schema_path(mut self, schema_path: impl Into<String>) -> Self {
        self.schema_path = Some(schema_path.into());
        self
    }

    pub fn with_record_identity(mut self, identity: impl Into<String>) -> Self {
        self.record_identity = Some(identity.into());
        self
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    pub fn with_related_requirement(mut self, requirement: impl Into<String>) -> Self {
        self.related_requirements.push(requirement.into());
        self
    }
}

impl Display for Diagnostic {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "[{}] {}", self.code, self.message)?;
        if let Some(source) = &self.source {
            write!(formatter, " ({})", source.display())?;
        }
        if let Some(suggestion) = &self.suggestion {
            write!(formatter, "; suggestion: {suggestion}")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Error)]
#[error("{diagnostic}")]
pub struct MasterdataError {
    pub diagnostic: Box<Diagnostic>,
}

impl MasterdataError {
    pub fn new(code: impl Into<String>, kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            diagnostic: Box::new(Diagnostic::new(code, kind, message)),
        }
    }

    pub fn with_source(mut self, source: impl Into<PathBuf>) -> Self {
        self.diagnostic.source = Some(source.into());
        self
    }

    pub fn with_related_requirement(mut self, requirement: impl Into<String>) -> Self {
        self.diagnostic
            .related_requirements
            .push(requirement.into());
        self
    }

    pub fn diagnostic(&self) -> &Diagnostic {
        &self.diagnostic
    }
}

pub fn io_error(path: &Path, error: impl Display) -> MasterdataError {
    MasterdataError::new(
        "E-IO-ACCESS",
        ErrorKind::Io,
        format!("could not access {}: {error}", path.display()),
    )
    .with_source(path.to_path_buf())
}
