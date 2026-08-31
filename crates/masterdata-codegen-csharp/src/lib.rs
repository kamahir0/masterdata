//! C# generation boundary.
//!
//! The renderer lowers the resolved Type System and Table models to the
//! approved public C# surface, including MessagePack field keys and the
//! MasterMemory table/key attributes. It does not orchestrate production
//! binary output, caching, or final artifact writes; those belong to the .NET
//! builder boundary described in `masterdata-dotnet`.

mod model;
mod render;

pub use model::{CSharpGenerationPlan, GeneratedFile, GenerationNote};
pub use render::CSharpGenerator;
