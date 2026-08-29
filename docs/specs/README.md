# Specifications

Specifications are version-controlled product and domain contracts. They state
observable behavior and compatibility rules in language that a person can
review and a test can exercise. Code may implement only a subset while a
specification is being drafted, but code MUST NOT silently contradict an
`Approved` specification.

This directory is the canonical home for product and domain specifications.
The repository workflow for turning conversation into a proposed change is
documented in [Specification workflow](../contributing/specification-workflow.md).

## Normative and non-normative content

Normative content defines behavior that an implementation is required or
permitted to expose. It lives primarily in `Normative Requirements`,
`Validation Rules`, and explicitly marked compatibility rules. Each normative
requirement has a stable ID.

Non-normative content includes summaries, rationale, examples, implementation
notes, references, and design discussion. It helps people understand a
specification but does not add behavior. An example MUST NOT be the only place
where a product rule is stated.

Specifications are not meeting minutes. Conversation history belongs in the
change or review context, decision rationale belongs in an ADR when it is
important, and the specification contains the resulting behavior only.

## Status lifecycle

The `Status:` header uses exactly one of these values:

- `Draft`: still being organized; it may contain unresolved questions and is
  not an implementation authority.
- `Proposed`: sufficiently structured to review as an implementation
  candidate, but not finally approved by a human maintainer.
- `Approved`: the current normative contract. Implementation work SHOULD use
  this status as its source of truth.
- `Implemented`: an `Approved` contract whose acceptance criteria have
  corresponding implementation, tests, and appropriate fixture evidence. The
  meaning of the requirements does not change when this status is applied.
- `Deprecated`: an old contract retained for history or compatibility context;
  it is no longer a target for new implementation. A replacement or reason
  SHOULD be linked.

The normal progression is `Draft` -> `Proposed` -> `Approved` ->
`Implemented`. A human maintainer may move a specification to `Deprecated`.
Only an explicit human approval (for example, a reviewed repository change or
an equivalent maintainer operation) may move a proposal to `Approved`. An AI
agent MUST NOT make that transition automatically. A review recommendation of
“Approved as Proposed” is evidence for a human reviewer, not the approval
operation itself.

A semantic change to an approved document is a new proposed change. Unchanged
requirement IDs remain stable; replaced or split requirements receive new IDs
and retain a reference to their predecessor. An incomplete implementation
keeps the document `Approved` and reports the gap rather than changing the
contract to match the code.

## Normative language

Use the following words literally and consistently in normative requirements:

- `MUST` / `MUST NOT`: required or forbidden behavior.
- `SHOULD` / `SHOULD NOT`: a strong default with a documented reason for the
  exceptional case.
- `MAY`: permitted behavior or capability; it does not recommend that the
  behavior be used.

The wording must preserve the strength of the evidence. “Can be placed” or
“should be possible” does not by itself mean that all callers SHOULD or MUST
do it. Questions, ideas, preferences, and proposals are not normative until a
separate explicit decision resolves them. If the source conversation does not
settle a default or edge case, keep it as an `Open Question`.

## Requirement ID convention

Requirement IDs use uppercase ASCII segments separated by hyphens and a
three-digit terminal number: `<DOMAIN>-NNN` or `<DOMAIN>-<TOPIC>-NNN` (for
example `PROJECT-001`, `SCHEMA-VO-001`, `INDEX-PRIMARY-001`, and `REF-001`).
GUI requirements use a `GUI-` domain prefix, such as
`GUI-TABLE-EDIT-001`. The domain should identify the canonical owning area.

IDs are allocated by searching all existing specifications before adding one.
Once published, an ID MUST NOT be renamed, reassigned, or reused after
deletion. A changed meaning gets a new ID and a predecessor/deprecation note.
The same normative rule belongs in one canonical specification; other
documents link to its ID instead of copying it. The lightweight
`cargo xtask check-specs` command checks duplicate IDs and malformed headers.

## Open Questions

An `Open Questions` section records unresolved choices, ambiguity, and missing
acceptance detail. It is non-normative. A question that would change behavior
MUST block approval or be explicitly excluded from the current proposal. No
agent may silently resolve it for implementation convenience. Once answered,
the decision is recorded in the appropriate proposed specification change and,
when useful, an ADR.

## Compatibility and traceability

Every change to stable identity, serialized shape, generated API, file
interpretation, or other public behavior considers backward compatibility. The
specification should say whether the change is compatible, requires migration,
or is intentionally breaking; “not applicable” is also a valid explicit
answer.

Tests and fixtures are evidence for requirements, not substitutes for them.
Where a one-to-one mapping is useful, include the requirement ID in a test
name or a nearby comment, for example:

```rust
#[test]
fn vo_001_rejects_invalid_underlying_type() {
    // Covers SCHEMA-VO-001.
}
```

Add a case to `fixtures/minimal`, `fixtures/full`, or `fixtures/invalid` when a
stable end-to-end input makes the rule clearer. A focused unit or integration
test is sufficient for rules that do not need a fixture. Fixture files are
fixed test inputs and MUST NOT be rewritten by CLI or GUI execution.

## Specification change procedure

1. Extract intent from the conversation or request and classify each statement.
2. Read the affected specifications, ADRs, RFCs, and
   [terminology](../product/terminology.md).
3. Update or create a `Draft`/`Proposed` specification with stable IDs,
   explicit open questions, compatibility impact, and test impact.
4. Run `review-spec` against the change. Resolve blocking issues or record why
   a human reviewer accepts them.
5. A human maintainer explicitly approves the proposal by the repository's
   review operation and changes the status to `Approved`.
6. Use `implement-spec` to implement only the approved behavior, synchronize
   tests and fixtures, and run repository verification.
7. Change the status to `Implemented` only after the evidence is complete.

Typo fixes, formatting-only edits, and internal refactors with no public or
domain semantic change may use the ordinary code/document workflow. When the
semantic boundary is uncertain, use the specification workflow.

## RFC, specification, and ADR

- An **RFC** is a comparatively large design proposal while adoption and
  alternatives are still under discussion. It is not an implementation
  authority.
- A **specification** is the adopted product/domain behavior, including its
  normative requirements and compatibility contract.
- An **ADR** records why an important architectural choice was selected. It
  should point to affected requirements but should not become a second copy of
  their semantics.

The usual relationship is RFC -> approved specification, with an ADR added
when the rationale or trade-off is important to preserve.

## Documents

- [Project layout and discovery](project-layout.md)
- [Schema language](schema-language.md)
- [Type system](type-system.md)
- [Indexes and references](index-and-reference.md)
- [Build pipeline](build-pipeline.md)
- [Compatibility](compatibility.md)

The initial Rust implementation currently covers the project contract, YAML
document envelope, basic field identity checks, duplicate `id` checks, schema
hashing, and build-plan formation. Type resolution, indexes, references,
MasterMemory binary generation, and the full GUI remain deliberately
incomplete; their current documents remain `Draft` until refined and approved.
