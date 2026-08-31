//! C# generation boundary.
//!
//! The renderer produces the approved Type System declarations and the current
//! immutable Table row scaffold. It does not emit MasterMemory index/source
//! generator or binary-format code; those belong to the .NET builder boundary
//! described in `masterdata-dotnet`.

mod model;
mod render;

pub use model::{CSharpGenerationPlan, GeneratedFile, GenerationNote};
pub use render::CSharpGenerator;
