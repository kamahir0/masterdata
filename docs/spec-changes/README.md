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
3. The artifact is `Proposed` while review is pending. A human maintainer
   explicitly moves it to `Approved` or `Rejected`; an AI-generated artifact
   is never auto-approved.
4. After explicit approval, the delta is applied atomically to the canonical
   specification and tests/fixtures are updated as appropriate. The artifact
   then moves to `Applied` and records the approval and changed canonical
   requirements. The canonical document remains `Approved` until
   implementation evidence justifies `Implemented`; if it was previously
   `Implemented`, applying a semantic delta returns it to `Approved` first.

`Draft` means the artifact is incomplete. `Proposed` means it is reviewable but
not approved. `Approved` records the human decision while the canonical merge
is still pending. `Applied` records that the approved delta is present in the
canonical document. `Rejected` records a declined proposal. A proposal is not
an implementation contract before it is `Applied`; `implement-spec` must use
the canonical approved document after the merge, not the proposal. For a large
change whose options are still being compared, use `docs/rfcs/` and route the
adopted behavior into this workflow.

Use [_template.md](_template.md) and a monotonically allocated filename such as
`0001-table-identity.md`. Proposal numbers are history and must not be reused.
