# ADR 0003: .NET is the MasterMemory bridge

Status: Accepted

## Context

MasterMemory v3 Source Generator and binary details belong to the .NET library
ecosystem. Reimplementing those internals in Rust would create compatibility
and maintenance risk.

## Decision

Rust prepares validated inputs, generated C# scaffolding, and a schema hash.
`masterdata-dotnet` owns .NET process invocation and will eventually invoke a
repository builder that compiles C# and produces/validates MasterMemory binary.

## Consequences

The process boundary must be explicit and testable. The current repository
includes a dependency-free builder smoke test, while the actual Source
Generator integration remains an explicit not-implemented stage. Future cache
reuse can be keyed by schema hash.

