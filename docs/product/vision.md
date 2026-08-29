# Product vision

Status: Draft

`masterdata` is a local-first authoring and build system for Unity projects
that use MasterMemory. YAML is the human-editable source of truth. A Rust
application core provides one set of project, schema, data, and validation
semantics to both the CLI and the Tauri desktop application. A narrow .NET
adapter owns the MasterMemory-specific compilation and binary build.

The product should make a cloneable repository understandable to both a human
developer and an AI agent: behavior is specified in Git, generated artifacts
are reproducible, errors have structured locations, and unsupported features
are visible rather than silently approximated.

## Success criteria

- A developer can discover and validate a project without opening Unity.
- The same validation result is available from CLI and GUI.
- YAML files can be split or moved without changing table identity.
- Schema evolution has explicit stable IDs and compatibility checks.
- MasterMemory internals remain delegated to the .NET ecosystem.

## Non-goals for the initial setup

- Full schema language implementation
- MasterMemory Source Generator or binary-format reimplementation
- Production-grade table editor
- Code signing, notarization, or distribution automation

Open product questions are tracked in the relevant specification rather than
being hidden in initial code.

