# RFCs

An RFC is a comparatively large design proposal while adoption, alternatives,
and trade-offs are still being discussed. It is not an approved product
specification and MUST NOT be used by `implement-spec` as an implementation
authority.

## RFC lifecycle

- `Draft`: the proposal is being organized and may contain incomplete
  alternatives or open questions.
- `Proposed`: the proposal is ready for review but has no adopted decision.
- `Accepted`: a human maintainer selected the proposal direction. The adopted
  product behavior still belongs in a canonical `docs/specs` document.
- `Rejected`: the proposal was explicitly declined and is retained for
  history.
- `Superseded`: a later RFC replaced the proposal; the successor should be
  linked from the document.

RFC status is separate from product specification status. In particular,
`Accepted` does not make a product specification `Approved`, and `Implemented`
is never an RFC status. An accepted RFC must be reflected in the canonical
specification through the approved-specification change workflow before it is
an implementation authority.

When a proposal is selected, its adopted behavior moves into the relevant
`docs/specs` document. The RFC may remain as rationale and should link to the
resulting requirement IDs. If the architectural reason is important to keep,
record it separately in `docs/adr` rather than duplicating normative rules.

Use `docs/spec-changes/` for a focused semantic delta to an existing
`Approved` or `Implemented` canonical specification when the alternatives are
already understood. A change artifact is durable review input, but it is not
an implementation authority until a human-approved atomic merge updates the
canonical specification.

Use [_template.md](_template.md) for new RFCs and the
[specification workflow](../contributing/specification-workflow.md) for the
conversation-to-review path.
