# Build pipeline

Status: Draft

The intended pipeline is:

```text
resolve project
 -> load config
 -> discover YAML
 -> parse schema/data
 -> semantic validation
 -> resolve types/indexes/references
 -> generate C#
 -> calculate schema hash
 -> compile or reuse .NET builder
 -> build MasterMemory binary
 -> validate binary
 -> write temporary output
 -> atomically replace final output
```

The Rust core owns the first stages and exposes a `BuildPlan` with validated
documents and a deterministic schema hash. `masterdata-codegen-csharp` owns
structured C# rendering. `masterdata-dotnet` is the only process boundary for
the .NET builder. MasterMemory binary format and Source Generator behavior
MUST remain in .NET dependencies.

Current behavior is intentionally smaller: `prepare_build` validates and
hashes, the C# crate plans a primitive immutable scaffold, and the .NET crate
can run a real builder bridge smoke test. `build_mastermemory` returns an
explicit `DOTNET-MASTERMEMORY-001` not-implemented diagnostic.

Future cache keys SHOULD include the schema hash so data-only changes can
reuse a compiled builder. Output replacement MUST be atomic once binary build
exists.

Open Questions: generated project ownership, cache eviction, and how Unity
asset import should observe atomic output replacement.

