# Specification workflow

Workflow status: Active

This document defines the repository process for turning design conversation
into reviewable, implementable behavior. It is a process document, not a
meeting transcript, so it uses `Workflow status` rather than the product
specification lifecycle. Product and domain semantics still require an
`Approved` specification before implementation.

## The canonical path

```text
conversation or request
        |
        v
extract intent and evidence
        |
        v
read existing specs, ADRs, RFCs, and terminology
        |
        v
classify statements and preserve their strength
        |
        v
Draft/Proposed specification change
        |
        v
review-spec
        |
        v
explicit human approval
        |
        v
Approved Specification
        |
        v
implement-spec -> tests / implementation / fixtures -> verification
        |
        v
Implemented (only when evidence is complete)
```

Conversation is evidence for refinement, not permanent specification. An AI
agent MUST NOT promote a proposal, preference, question, or remembered context
to approved behavior without explicit evidence. Long conversation logs do not
belong in a specification; retain only the decision rationale that deserves an
ADR.

## Statement classification

`refine-spec` classifies each relevant statement before it writes normative
text. A single conversation may contain several classes.

| Class | Meaning | Normative by itself? |
| --- | --- | --- |
| `Decision` | An explicit choice that the speaker has settled, such as “we will do this”. | It is evidence for a proposed rule, but still goes through review and human approval. |
| `Requirement` | A desired capability or outcome the product should provide. | No fixed strength; derive MUST/SHOULD/MAY only from the wording and context. |
| `Constraint` | A boundary, prohibition, or condition, such as “we do not want this”. | A clear constraint can become MUST NOT or another explicit rule in the proposal. |
| `Preference` | A favored option, priority, or taste without a binding commitment. | No. Keep it non-normative unless explicitly promoted. |
| `Proposal` | A candidate solution offered for consideration. | No. Compare it or record it as an open choice. |
| `Idea` | An exploratory possibility or brainstorming thought. | No. Do not implement it as a requirement. |
| `Question` | A request for information or clarification. | No. Answer it only with evidence; otherwise track it. |
| `Open Question` | An unresolved decision or ambiguity retained for follow-up. | No. It blocks approval when its answer would change behavior. |
| `Rejected` | An option or behavior explicitly ruled out. | It is a constraint against silently reintroducing that behavior. |

The phrases “this might be okay”, “it may be better to”, and “what about X?”
are normally a `Preference`, `Proposal`, or `Question`, not a `Decision`.
Phrases such as “we will do X”, “we do not want Y”, and “X is mandatory” can
be `Decision` or `Constraint` evidence when their scope is clear.

## Preserve intent strength

Normative words are not interchangeable:

- `MUST` and `MUST NOT` are reserved for explicit requirements or constraints.
- `SHOULD` and `SHOULD NOT` express a strong recommendation, not a mere
  capability or possibility.
- `MAY` expresses permission or capability; it does not recommend the behavior.

For example, “we want every table to be placeable in the same directory” does
not justify recommending one shared directory. A faithful proposed
normalization can say that data-file location MUST NOT determine table
identity and that multiple table data files MAY coexist in one directory. It
must not turn “possible” into “SHOULD be stored there”.

When strength, default behavior, error severity, ordering, nullability, or an
edge case is unspecified, preserve the ambiguity in `Open Questions`. Do not
invent a default for implementation convenience.

## Roles and boundaries

### refine-spec

`refine-spec` answers “what should become a specification?” It reads the
conversation and repository context, identifies the canonical affected spec,
classifies statements, detects conflicts, assigns stable IDs, and produces a
Draft/Proposed change. If the canonical document is already `Approved` or
`Implemented`, the semantic delta MUST be written to a separate
`docs/spec-changes/` artifact (or an RFC when alternatives are still open);
the canonical document MUST NOT be edited to mix in unapproved behavior. It
must keep requirement IDs and runtime diagnostic codes in separate namespaces,
and it must not promote current code behavior to a requirement without source
evidence. It may recommend an RFC or ADR. It does not implement a product
feature and does not set `Approved` automatically.

### review-spec

`review-spec` is an independent challenge pass. It compares the proposed text
with the source request when available, checks related specs and ADRs, tests
normative strength and testability, and reports blocking issues, non-blocking
issues, questions, and whether it is suitable for human approval. Its
“Approved as Proposed” result is a recommendation only; it does not change the
specification status. It also checks whether an existing implementation already
contains unapproved semantics, whether requirement IDs and test names actually
match, and whether an unapproved change was mixed into an approved canonical
document.

### implement-spec

`implement-spec` starts from an explicitly `Approved` specification. It maps
requirement IDs to acceptance criteria, tests, fixtures, affected crates, GUI
boundaries, and the .NET adapter, then implements and verifies the approved
behavior. It treats diagnostic codes as a separate namespace from requirement
IDs, checks acceptance/test traceability, and does not treat incorrect current
behavior as authority. A `docs/spec-changes/` proposal, Draft, or Proposed
document is never an implementation input. It does not design missing
semantics from conversation. A missing rule is reported as `Specification Gap`
and sent back to refinement.

## Approval and lifecycle

The lifecycle and exact status meanings are defined in
[the specification index](../specs/README.md). In particular:

- `Draft` and `Proposed` are not implementation authority.
- Only a human maintainer's explicit approval can produce `Approved`.
- An AI-generated draft is never automatically promoted.
- `Implemented` means the approved meaning was implemented and evidenced; it
  is not permission to weaken or reinterpret the specification.
- A specification-change artifact uses `Draft` -> `Proposed` -> `Approved` ->
  `Applied`, or `Rejected`. `Applied` records the atomic canonical merge; it
  is not an implementation status.
- A semantic change starts another proposed change artifact. A typo,
  formatting-only change, or internal refactor with no public/domain semantic
  change may use a normal workflow.

## Approved specification changes

An approved canonical document is immutable with respect to unapproved
semantics. When refinement discovers a semantic change, create a durable
artifact under [`docs/spec-changes`](../spec-changes/README.md), link the
affected requirement IDs, and leave the canonical `Approved`/`Implemented`
document unchanged. Use `review-spec` on the artifact. After explicit human
approval, apply the approved delta atomically to the canonical document and
keep the proposal as an audit record with `Status: Applied`. Only then may
implementation begin; the proposal itself is never an implementation contract.
If the canonical document was `Implemented`, applying a semantic delta returns
it to `Approved` until the new acceptance evidence is complete. This prevents
an `Approved` document from simultaneously claiming old requirements are
approved and new requirements are not.

## Normalization and ownership

Before adding a requirement, search all existing IDs and read the owning spec.
One semantic rule has one canonical home. A related document should link to
that requirement or summarize its relationship without copying a second
normative version. Conflicts are surfaced to the reviewer; they are never
silently overwritten.

Use [product terminology](../product/terminology.md) as the vocabulary
baseline. If a term has competing meanings, record the ambiguity as an Open
Question or refine the terminology document before relying on it.

## Tests and fixtures

Every approved behavior receives a proportionate verification plan. Prefer a
requirement ID in a test name or nearby comment when it clarifies traceability.
Use `fixtures/minimal`, `fixtures/full`, or `fixtures/invalid` for stable
end-to-end cases where a fixture is useful; a focused unit test is enough for
small or internal rules. Fixtures remain fixed inputs and are copied into
`target/dev-project` by repository tooling rather than edited during a run.

`implement-spec` runs the relevant tests and finishes with
`cargo xtask check-all` when the environment supports it. The final report
states any unavailable checks and any explicitly unimplemented boundary.

## Keeping the workflow healthy

The three skills are repository artifacts under `skills/` and are reviewed and
versioned like code. When an incident reveals a recurring failure mode—such as
an agent changing `MAY` into `SHOULD`—add a focused rule or regression example
to the relevant skill and, where useful, to this document. Skill improvements
must preserve the role boundaries and the human approval gate.

## RFC and ADR routing

Use an RFC when a significant design is still comparing alternatives. Once a
choice is adopted, put the actual product/domain behavior in a specification.
Use an ADR for the reason an architectural option was chosen, especially when
it affects crate boundaries, schema representation, identity, compatibility,
or an external bridge. Neither an RFC nor an ADR silently overrides an
approved specification.
