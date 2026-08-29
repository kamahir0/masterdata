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
 -> calculate schema source-content hash
 -> compile or reuse .NET builder
 -> build MasterMemory binary
 -> validate binary
 -> write temporary output
 -> atomically replace final output
```

The Rust core owns the first stages and exposes a `BuildPlan` with validated
documents and a deterministic schema source-content hash. `masterdata-codegen-csharp` owns
structured C# rendering. `masterdata-dotnet` is the only process boundary for
the .NET builder. MasterMemory binary format and Source Generator behavior
MUST remain in .NET dependencies.

Current behavior is intentionally smaller: `prepare_build` validates and
calculates a schema source-content hash, the C# crate plans a primitive immutable
scaffold, and the .NET crate can run both a bridge smoke test and a separate
MasterMemory v3 technical spike. Production schema-driven binary generation
remains an explicit not-implemented boundary.

Generated identifier validation is a conservative scaffold guard. The complete
C# naming policy is tracked in the Proposed
[`C# naming RFC`](../rfcs/0003-csharp-naming.md), not silently defined here.

## Output and identity boundaries

The current configuration treats `build.output` as the generated C# output
directory. An optional `build.binary_output` can identify a future
MasterMemory binary destination, while `build.cache` identifies a separate
cache directory. These paths are represented separately in the build plan and
must not be inferred from one another. The final configuration contract,
atomic replacement details, and Unity placement policy remain Open Questions.

The following identities are distinct and MUST NOT be conflated:

- **Schema source-content hash**: a hash of schema source bytes used for diagnostics and
  change detection in the current scaffold. Whitespace/comments can change it.
- **Semantic schema hash**: a future hash of parsed, resolved, canonical
  schema meaning. It is not implemented until the type system and YAML subset
  are specified.
- **Builder cache key**: a future composite identity that may include the
  semantic schema hash, C# generator version, MasterMemory version, MessagePack
  version, builder protocol version, and target/runtime compatibility inputs.

The current `BuildPlan` exposes only the schema source-content hash. It MUST NOT be
described or used as a semantic builder cache key. Generated C# output,
MasterMemory binary output, and cache directory are separate concepts; the
eventual configuration shape remains an Open Question. Output replacement MUST
be atomic once binary build exists.

Open Questions: generated project ownership, cache eviction, and how Unity
asset import should observe atomic output replacement. The product-level
symlink follow/ignore policy for source discovery is also unresolved; the
current traversal guard does not follow symlink entries to prevent cycles.
