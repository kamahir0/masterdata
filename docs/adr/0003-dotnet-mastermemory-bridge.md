# ADR 0003: .NET is the MasterMemory bridge

Status: Accepted

## Context

MasterMemory v3 Source Generator and binary details belong to the .NET library
ecosystem. Reimplementing those internals in Rust would create compatibility
and maintenance risk.

## Decision

Rust prepares validated inputs, generated C# scaffolding, and a schema
source-content hash.
`masterdata-dotnet` owns .NET process invocation and will eventually invoke a
repository builder that compiles C# and produces/validates MasterMemory binary.

## Consequences

The process boundary must be explicit and testable. The repository includes a
dependency-free builder smoke test and an isolated, hand-written MasterMemory
v3 technical spike that exercises Source Generator, binary build, reload, and
lookup. Production schema-driven generation remains an explicit
not-implemented stage. Future cache reuse must use a semantic schema/cache-key
design rather than treating the schema source-content hash as semantic
identity.
