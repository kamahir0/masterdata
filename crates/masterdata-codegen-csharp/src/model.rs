use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedFile {
    pub relative_path: PathBuf,
    pub contents: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationNote {
    pub message: String,
    pub placeholder: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CSharpGenerationPlan {
    pub namespace: String,
    pub files: Vec<GeneratedFile>,
    pub notes: Vec<GenerationNote>,
}
