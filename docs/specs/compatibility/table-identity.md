# Table identity

Status: Implemented

## Summary

The current scaffold has one project-local logical identity for a table. Generated C# naming is a separate presentation concern.

### COMPAT-TABLE-001

For the current scaffold, `table` is the project-local stable table identity and the generated C# type name is a separate, renameable presentation name.

This requirement does not define global identity, table rename migration, released-schema compatibility, legacy `tableId` migration, or cross-project identity.

## Implementation evidence

- Schema and data documents associate by their declared `table` value rather than source path.
- The C# generator uses `csharpName` only as a generated type-name override while retaining the source `table` identity separately.
- Project discovery tests verify that source directory placement cannot relabel a document.

The accepted design rationale is recorded in [RFC 0001](../../rfcs/0001-table-identity.md), and the applied approval record is [specification change 0001](../../spec-changes/0001-table-identity.md).

## Open Questions

- Does a project eventually need a globally stable table identity?
- What compatibility and migration behavior applies when `table` changes?
- Should legacy `tableId` input be rejected, preserved as non-semantic metadata, or supported during a migration window?
- Is cross-project table identity required?
