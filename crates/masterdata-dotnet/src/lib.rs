//! Boundary for invoking the .NET builder.
//!
//! All process invocation is kept in this crate. The current builder is a
//! contract smoke test only; it intentionally does not claim to produce a
//! MasterMemory binary.

mod bridge;

pub use bridge::{BridgeSmokeReport, BridgeSmokeStatus, DotnetBridge, DotnetProbe};
