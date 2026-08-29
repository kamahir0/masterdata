---
name: review-spec
description: Independently audit a Draft or Proposed specification change against its source intent and repository contracts before human approval.
---

# review-spec

## Purpose

Use this skill to challenge a specification change produced by refinement. The
reviewer should not share the author's assumptions: inspect the target change,
the original request or conversation when available, related specifications,
ADRs, RFCs, and the terminology glossary. The result is a review report, not a
status transition.

`Approved as Proposed` means “the reviewer found no blocking issue for human
consideration”. It MUST NOT change `Status: Proposed` to `Status: Approved`.
Only a human maintainer performs that approval operation.

## Inputs and scope

Required inputs are the target spec change and its path. Preferably also
provide the source request/conversation and the affected test or fixture plan.
If the source is missing, say so and treat intent fidelity as unverified; do
not fill the gap from memory.

Read:

1. `AGENTS.md`.
2. The entire target specification, including status, IDs, Open Questions,
   examples, and Non-Goals.
3. Related `docs/specs` and the canonical owner of each referenced ID.
4. Related `docs/adr`, `docs/rfcs`, and `docs/product/terminology.md`.
5. Existing tests/fixtures only as evidence of current behavior, never as
   permission to override an approved contract.

If the change targets an `Approved` or `Implemented` canonical document,
require a separate proposal artifact under `docs/spec-changes/` (or an RFC for
alternative comparison). A canonical approved document must not contain a
mixture of approved old behavior and unapproved new behavior.

Use `rg` to search for duplicate IDs, terminology variants, and conflicting
normative phrases. Keep the review focused on semantics; do not demand a
specification for a typo or a purely internal refactor.

## Review checklist

Record a finding with file, requirement ID, evidence, impact, and a concrete
resolution when a check fails.

### Intent fidelity

- Does every normative rule have evidence in the supplied request or an
  existing approved contract?
- Were `Preference`, `Proposal`, `Idea`, `Question`, and `Open Question`
  kept non-normative?
- Were rejected choices kept out?
- Did the wording preserve the speaker's scope and certainty?
- Does the proposal avoid promoting current implementation behavior, a
  diagnostic code, or a test name into a requirement without source evidence?

### Internal consistency

- Are status, Summary, Normative Requirements, Validation Rules,
  Compatibility, Examples, Open Questions, and Non-Goals consistent?
- Are requirement IDs unique within the change and stable across edits?
- Does a requirement contradict another requirement in the same document?
- Does the proposal distinguish requirement definitions from references, and
  are referenced IDs owned by exactly one canonical document?

### Cross-spec consistency

- Does the change agree with all affected approved and proposed specs?
- Does it respect ADR architecture, especially core/CLI/GUI and the .NET
  bridge boundary?
- Is the canonical owner clear, with links rather than copied normative text?
- Are duplicate or conflicting requirements present elsewhere?
- Does the existing implementation already contain semantics not authorized by
  the target specification, requiring removal or a separate proposal instead
  of retroactive approval?

### Terminology consistency

- Do terms follow `docs/product/terminology.md`?
- Are distinctions such as table/file, field ID/index number,
  unique/non-unique, and source/generated artifact preserved?
- Does the change introduce a term without defining or routing it?

### Normative strength

- Is every `MUST`, `MUST NOT`, `SHOULD`, `SHOULD NOT`, and `MAY` justified?
- Has a capability been strengthened into a recommendation or requirement?
- Are recommendations clearly labeled as such and supported by a reason?
- Are examples or implementation notes accidentally written as normative
  behavior?
- Are runtime diagnostic codes visibly distinct from requirement IDs, with no
  shared namespace or misleading numeric suffix reuse?

### Testability

- Can each normative requirement be observed and tested?
- Are success, validation failure, error, and compatibility outcomes specified
  enough to write an acceptance test?
- Is a requirement ID mapped to a test name or nearby comment where useful?
- Is fixture coverage requested only where an end-to-end fixture adds value?
- Do requirement IDs in test names/comments actually describe the behavior they
  cover, rather than merely borrowing a number?

### Backward compatibility

- Does the change affect stable table/field/enum identity, serialized data,
  generated APIs, file interpretation, or migration?
- Is compatibility explicitly stated, including “not applicable” when true?
- Are deprecated IDs and migration/replacement notes preserved?

### Unresolved ambiguity

- Are defaults, nullability, ordering, error severity, missing data, and edge
  cases either specified by evidence or listed as Open Questions?
- Has any Open Question been silently answered in prose, examples, or a test
  plan?
- Would an unanswered question change implementation behavior enough to block
  approval?

### Implementation leakage

- Does the spec define observable behavior rather than a convenient data
  structure, algorithm, crate layout, or shell command?
- Does GUI text stay at the GUI behavior boundary and leave domain semantics
  in `masterdata-core`?
- Does it avoid reimplementing MasterMemory internals or moving .NET process
  invocation into another crate?

### Unrequested behavior

- Did the change add features, defaults, validations, UI states, or migration
  behavior not requested or supported by an approved contract?
- Were adjacent ideas placed in Non-Goals, an RFC, or Open Questions instead?
- Is any semantic change mixed directly into an Approved/Implemented canonical
  file instead of being isolated in a durable proposal artifact?

## Required output

Use this structure so a human can make the approval decision quickly:

### Blocking Issues

Issues that make the proposal unsafe, ambiguous, contradictory, untestable,
over-specified, or unsupported by the source. Include `None identified` when
empty.

### Non-blocking Issues

Editorial, traceability, or maintainability concerns that do not prevent
approval. Include `None identified` when empty.

### Questions

Questions for the author or human maintainer, especially unresolved behavior
that is not yet a blocking issue. Include `None identified` when empty.

### Approved as Proposed

State `Yes` or `No`, followed by a short reason. This is a review
recommendation only; explicitly state that human approval and the status
transition remain outstanding.

Also include a compact verdict table or list covering Intent fidelity,
Internal consistency, Cross-spec consistency, Terminology consistency,
Normative strength, Testability, Backward compatibility, Unresolved ambiguity,
Implementation leakage, and Unrequested behavior.

## Safeguards

- Never solve an Open Question while reviewing it.
- Never normalize a weak statement into a stronger requirement for neatness.
- Never mark the spec `Approved` or `Implemented`.
- Never review only the changed paragraph when an ID or term is shared across
  specifications.
- Do not turn current code behavior into a new requirement without evidence.
- Do not treat implementation, diagnostics, or mismatched test labels as
  authority for a requirement.
