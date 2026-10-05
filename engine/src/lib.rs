//! YAML source authority and shared MasterData semantics.
pub mod clipboard;
pub mod codegen;
pub mod creation;
pub mod delivery;
pub mod initializer;
pub mod instrument;
pub mod migration;
pub mod native;
pub mod project;
pub mod semantic;
pub mod source;
pub mod type_migration;
pub mod workspace;

#[derive(Clone, Debug, thiserror::Error)]
#[error("{code}: {message}")]
pub struct Error {
    pub code: &'static str,
    pub message: String,
}

impl Error {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
