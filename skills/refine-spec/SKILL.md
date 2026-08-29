---
name: refine-spec
description: Turn a design conversation or request into a traceable Draft/Proposed specification change without promoting assumptions, ideas, or questions into approved behavior.
---

# refine-spec

## Purpose

Use this skill when a conversation, issue, or request may change product,
domain, compatibility, or user-visible GUI behavior. Its job is to answer
“what should become a specification?” It produces a reviewable Draft/Proposed
change and a concise refinement report. It does not implement product
behavior, and it MUST NOT set a specification to `Approved` automatically.

Typo fixes, formatting-only changes, and internal refactors with no public or
domain semantic effect normally do not need this skill. If that boundary is
unclear, use this skill and record the uncertainty.

## Required context

1. Read `AGENTS.md` before changing repository files.
2. Read the relevant files under `docs/specs` and the relevant files under
   `docs/adr` before proposing a semantic change. Read `docs/rfcs` when an
   alternative design is still being compared.
3. Read `docs/product/terminology.md` and use its terms. If a term conflicts
   with the repository glossary, expose the conflict rather than inventing a
   synonym.
4. Search existing requirement IDs and related wording with `rg` before
   allocating an ID or adding a new rule.
5. Identify whether the request affects the CLI, GUI, `masterdata-core`,
   `masterdata-codegen-csharp`, `masterdata-dotnet`, fixtures, or tests. This
   is impact analysis, not permission to implement them.

If a referenced conversation, issue, or source document is unavailable, state
that it was not provided. Do not reconstruct it from memory or from an
unstated assumption.

## Procedure

### 1. Extract evidence

Read the request as evidence and separate what was explicitly said from what
would merely be convenient. Preserve speaker intent and scope. A statement
can be important without being normative.

Classify every relevant statement as one of:

- `Decision`: an explicit settled choice. It can support a proposed rule but
  still needs review and human approval.
- `Requirement`: a desired capability or outcome. Do not assign MUST,
  SHOULD, or MAY without examining its wording and context.
- `Constraint`: a boundary, prohibition, or condition. A clear prohibition can
  support `MUST NOT` in the proposal.
- `Preference`: a favored option or priority without a binding commitment.
- `Proposal`: a candidate solution offered for consideration.
- `Idea`: an exploratory possibility or brainstorming thought.
- `Question`: a request for clarification or information.
- `Open Question`: an unresolved choice or ambiguity that must remain visible.
- `Rejected`: an option or behavior explicitly ruled out; do not reintroduce
  it silently.

The phrases “this might be okay”, “it may be better to”, and “what about X?”
are not `Decision` evidence by themselves. “We will do X”, “we do not want Y”,
and “X is mandatory” may be a `Decision` or `Constraint` when their scope is
clear.

### 2. Compare repository contracts

Locate the canonical owning specification. Prefer updating one existing
requirement over copying the same semantic rule into multiple documents. Check
for conflicts with approved and proposed specifications, ADR decisions, GUI
boundaries, and the terminology glossary. Never silently overwrite a conflict;
report both sides and identify the human decision needed. If the canonical
document is `Approved` or `Implemented`, do not edit it with a semantic delta:
write a separate durable proposal under `docs/spec-changes/` (or an RFC when
alternatives are still being compared). The canonical document remains the
only authority until a human-approved atomic merge.

An `Accepted` RFC records a selected design direction, but it is not itself a
product specification and does not authorize implementation. After a human
decision, route the adopted behavior into the canonical specification (or a
specification-change artifact when an existing canonical document is affected)
without copying unresolved compatibility decisions into it.

### 3. Preserve normative strength

Use the weakest wording supported by the evidence:

- `MUST` / `MUST NOT` for explicit requirements and constraints.
- `SHOULD` / `SHOULD NOT` for an explicitly strong recommendation and its
  reason.
- `MAY` for a permission or capability; it is not a recommendation.

Do not turn “possible” into “recommended”, or “recommended” into “required”.
For example, a request to allow all table files to coexist in one directory
can support “file location MUST NOT determine table identity” and “multiple
table data files MAY coexist in one directory”. It does not support “all table
data files SHOULD be stored in one directory” unless that recommendation was
explicitly made.

Do not invent unspecified defaults, edge-case behavior, error severity,
ordering, nullability, migration policy, or implementation constraints. Keep
each unresolved item in `Open Questions`. Do not resolve a question merely to
make an implementation plan easier.

### 4. Draft the change

Use `docs/specs/_template.md` for a new domain specification and
`docs/gui/_template.md` for a new GUI surface. Give every new normative
requirement a stable ID. IDs use uppercase segments and a three-digit terminal
number, such as `SCHEMA-VO-001` or `GUI-TABLE-EDIT-001`.

Before assigning an ID, search all existing specifications. IDs are never
reused after deletion. If an existing requirement changes meaning, keep the
old ID for its old meaning where history requires it and allocate a new ID
with a predecessor/deprecation note. A reference to an existing requirement is
not a new requirement.

Mark a new document `Draft` while it is being organized, or `Proposed` when it
is ready for review. A semantic change to an existing Approved/Implemented
document belongs in a `docs/spec-changes/` artifact whose status is `Draft` or
`Proposed`; do not downgrade the canonical document. `Proposed` is not
approval. Do not change a document to `Approved` as part of this skill.

### 5. Assess downstream work

Describe compatibility and implementation impact without designing missing
semantics. Identify likely acceptance tests and fixture needs. Say when a unit
test is enough; do not require a fixture for every small rule. For GUI behavior,
identify relevant states and interactions without duplicating core domain
logic.

If the design compares substantial alternatives, recommend an RFC. If a
chosen architectural boundary or trade-off needs durable rationale, recommend
an ADR. Neither recommendation is a silent decision.

### 6. Preserve namespace and implementation boundaries

Requirement IDs describe normative specification rules. Runtime diagnostic
codes describe observed failures and MUST use a visibly separate namespace
(for example `PROJECT-001` versus `E-PROJECT-NOT-FOUND`). Never use a
diagnostic code as a requirement ID or infer a requirement from an existing
diagnostic/test name. Existing code behavior is evidence to inspect, not
authority to promote into a product rule.

During refinement, explicitly audit current behavior for assumptions that are
not supported by the owning specification, such as a field named `id` implying
a primary key or a diagnostic code being treated as a requirement ID. Such
behavior is a removal/reporting candidate, not evidence that the assumption is
approved.

## Required output

Start with the source evidence and statement classifications, then provide
these headings exactly. Use “None identified” when a section is empty.

### Affected Specifications

- File, current status, and affected requirement IDs.

### Confirmed Decisions

- Only explicit decisions/constraints supported by the supplied evidence.

### New Requirements

- New normative requirements with stable IDs and precise MUST/SHOULD/MAY
  wording.

### Changed Requirements

- Existing IDs, old meaning versus proposed meaning, and compatibility impact.

### Open Questions

- Every unanswered point that could change behavior, including missing source
  context. Do not answer it implicitly.

### Potential ADRs

- Architectural decisions that may deserve an ADR, or “None identified”.

### Compatibility Impact

- Stable identity, serialized shape, generated API, file interpretation, and
  migration implications; explicitly say when not applicable.

### Implementation Impact

- Likely crates, GUI/Tauri boundary, .NET adapter, tests, fixtures, and
  verification. This is a plan, not implementation.

End by stating the proposed document status (`Draft` or `Proposed`) and the
human approval action still required. A concise change summary is mandatory.

## Non-negotiable safeguards

- Conversation alone is never a permanent specification.
- Do not use stale memory from earlier conversations as evidence.
- Do not promote `Preference`, `Proposal`, `Idea`, `Question`, or `Open
  Question` to normative behavior without an explicit decision.
- Do not strengthen `MAY` to `SHOULD`, `SHOULD` to `MUST`, or a possibility to
  a recommendation.
- Do not add unspecified defaults or edge cases.
- Do not duplicate a rule owned by another specification.
- Do not silently overwrite conflicting specifications or ADRs.
- Do not mix a semantic change into an Approved/Implemented canonical document;
  use a separate change artifact.
- Do not treat an `Accepted` RFC as a substitute for an `Approved` canonical
  specification.
- Do not treat a runtime diagnostic code, test number, or current behavior as a
  specification requirement without explicit evidence.
- Do not mark a Draft/Proposed specification `Approved` automatically.
- Do not implement product features as part of refinement.
