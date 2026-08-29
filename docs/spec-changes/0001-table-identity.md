# Specification change: adopt current table identity boundary

Status: Applied

## Affected Specifications

- [`docs/specs/compatibility.md`](../specs/compatibility.md),
  `COMPAT-TABLE-001`.
- [`docs/specs/schema-language.md`](../specs/schema-language.md), current
  scaffold identity explanation.
- [`docs/product/terminology.md`](../product/terminology.md), table identity
  glossary entry.

## Source Evidence and Classification

The repository-hardening task explicitly settled the current-scaffold
direction: `table` is the project-local stable logical table identity,
`csharpName` is the generated C# type-name override/presentation name, and
`tableId` is unnecessary at this stage. This is a human `Decision`, not an
inference from the existing implementation.

Global table identity, table rename migration, released-schema compatibility,
legacy `tableId` migration, and cross-project identity remain `Open Question`s.

## Proposed Delta

For the current scaffold, canonical specification text records the accepted
`table`/`csharpName` distinction and does not define a second `tableId`
identity. No unresolved compatibility question is promoted to a requirement.

## Compatibility

The current repository has no released schema format. The accepted direction
does not establish a migration promise or a global identity namespace.

## Acceptance and Implementation Impact

The typed Rust schema model, fixtures, terminology, and generated C# identity
comments must agree with the canonical direction. Existing tests confirm that
source location does not determine table identity. This change does not
implement indexes, references, type resolution, or rename migration.

## Open Questions

- Does a project eventually need a globally stable table identity?
- What migration behavior applies when `table` changes?
- Should legacy `tableId` input be rejected, preserved as non-semantic
  metadata, or supported during a migration window?
- Is cross-project table identity required?

## Review

`review-spec` concerns are resolved for the current-scaffold decision. The
remaining compatibility questions are intentionally retained above.

## Approval Record

The human decision in the repository-hardening task explicitly approved the
current-scaffold direction. The delta was applied atomically to the canonical
documentation and RFC 0001 was moved to `Accepted`. This artifact is retained
as the durable audit record and is `Applied`.
