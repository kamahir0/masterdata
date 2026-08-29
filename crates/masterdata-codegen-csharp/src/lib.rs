//! C# generation boundary.
//!
//! The renderer deliberately produces a small immutable C# scaffold today. It
//! does not emit MasterMemory attributes or binary-format code; those belong to
//! the .NET builder boundary described in `masterdata-dotnet`.

mod model;
mod render;

pub use model::{CSharpGenerationPlan, GeneratedFile, GenerationNote};
pub use render::CSharpGenerator;
