# RFC: Table identity and generated C# names

Status: Accepted

## Context

The initial schema examples contained both `table: item` and
`tableId: catalog.item`, while the Rust implementation used `table` for schema
and data association and the C# scaffold used `csharpName` for generated type
names. The second identity was not consumed by the model, validation, or
generator, so keeping it in examples creates an ambiguity before indexes and
compatibility are implemented.

## Problem

Future work needs a stable table identity for schema/data association and a
renameable generated C# presentation name. It must not infer identity from a
file or directory path, and it must not silently introduce a global namespace
or migration promise.

## Goals

- make the current scaffold's identity boundary explicit;
- keep `table` and generated `csharpName` distinct in purpose; and
- preserve room for a future project/global identity if a later proposal
  demonstrates a need for it.

## Non-Goals

This RFC does not decide index identity, MessagePack compatibility, table
renames, migrations, generated API naming policy, or a global identity format.

## Options

1. Use `table` as the project-local stable identity and `csharpName` as the
   generated C# type-name override.
2. Keep a separate required `tableId` and define a migration/namespace model
   for it.
3. Derive identity from the source path or generated C# name.

## Trade-offs

Option 1 matches the current core behavior and removes an unused duplicate
concept, but may require a later migration if projects need globally stable
identities. Option 2 makes that distinction explicit but adds a second value
before its semantics are needed. Option 3 conflicts with the existing file
location boundary and makes moves or renames domain changes.

## Proposal

For the current scaffold, use `table` as the project-local stable table
identity, treat `csharpName` as a renameable generated C# presentation name,
and remove the unused `tableId` field from the typed schema model and fixtures.
The `table`/`csharpName` distinction is documented in terminology and schema
language. The explicit human decision recorded for this repository hardening
task accepts this direction for the current scaffold. This acceptance does
not decide global identity, rename migration, released-schema compatibility,
legacy `tableId` migration, or cross-project identity.

## Compatibility

The repository has no released schema format in this scaffold. Removing an
unused, ignored model field is a source-shape cleanup for the current fixtures,
but accepting or rejecting legacy `tableId` input in a released parser remains
an Open Question. No global identity or rename migration is defined here.

## Open Questions

- Does a project eventually need a globally stable table identity?
- If so, is it the same value as `table`, or a separately versioned identity?
- What compatibility and migration behavior applies when `table` changes?
- Should the parser reject legacy `tableId`, preserve it as non-semantic
  metadata, or support it during a migration window?

## Decision

Accepted for the current scaffold by explicit human decision in the repository
hardening task. The canonical Draft specifications record the adopted
`table`/`csharpName` distinction; this RFC remains the rationale and option
comparison. RFC `Accepted` is not the same lifecycle state as a product
specification `Approved`, and unresolved compatibility questions remain open.
