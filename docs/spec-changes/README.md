# Specification changes

This directory stores durable proposals that change an existing canonical
specification. It is intentionally separate from `docs/specs/`: an
`Approved`/`Implemented` canonical document must not contain a semantic change
before a human has approved it.

## Lifecycle

1. `refine-spec` records the evidence, affected requirement IDs, proposed
   delta, compatibility impact, and open questions in a new artifact.
2. `review-spec` independently checks the artifact against the source request,
   canonical specifications, ADRs, terminology, current implementation, and
   testability.
3. A human maintainer explicitly approves or rejects the artifact.
4. After approval, the delta is applied atomically to the canonical
   specification, tests/fixtures are updated as appropriate, and the artifact
   records the approval. The canonical document then remains `Approved` until
   implementation evidence justifies `Implemented`.

An artifact with `Status: Draft` or `Status: Proposed` is not an
implementation contract. `implement-spec` must use the canonical approved
document after the merge, not the proposal. For a large change whose options
are still being compared, use `docs/rfcs/` and route the adopted behavior into
this workflow.

Use [_template.md](_template.md) and a monotonically allocated filename such as
`0001-table-identity.md`. Proposal numbers are history and must not be reused.
