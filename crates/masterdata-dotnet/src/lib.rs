//! Boundary for invoking the .NET builder.
//!
//! All process invocation and the internal Rust/.NET handoff protocol are
//! kept in this crate. Rust remains the semantic validator; the .NET side
//! owns the actual MasterMemory/MessagePack build and reload.

mod bridge;
mod protocol;

pub use bridge::{
    BridgeSmokeReport, BridgeSmokeStatus, DotnetBridge, DotnetProbe, MasterMemorySpikeReport,
};
pub use protocol::{
    BUILD_PROTOCOL_VERSION, MASTERMEMORY_VERSION, MESSAGEPACK_VERSION, MasterMemoryBuildReport,
    MasterMemoryBuildRequest, MasterMemoryTableReport, NormalizedField, NormalizedRecord,
    NormalizedTable,
};
