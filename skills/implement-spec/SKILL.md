---
name: implement-spec
description: Implement and verify behavior from an explicitly Approved specification while reporting specification gaps instead of inventing semantics.
---

# implement-spec

## Purpose and gate

Use this skill only when the requested behavior is identified by one or more
specification IDs whose canonical documents have `Status: Approved`. The
approved specification—not the conversation, an idea, a Draft, a Proposed
document, or an implementation convenience—is the input contract.

If the target is `Draft`, `Proposed`, or `Deprecated`, stop before changing
product code and report that human approval or a replacement specification is
required. If the target is already `Implemented`, verify the evidence or
report the remaining gap instead of silently changing semantics.

This skill does not auto-approve specifications and does not change their
meaning. A status may become `Implemented` only after the acceptance evidence
and repository checks are complete.

## Required preparation

1. Read `AGENTS.md`.
2. Confirm every target specification ID, canonical file, exact status, and
   predecessor/deprecation relationship.
3. Read the full target spec, related specifications, relevant ADRs, RFC
   outcome, and `docs/product/terminology.md`.
4. Extract the normative requirements, validation rules, compatibility notes,
   non-goals, and any non-blocking Open Questions.
5. Search the repository for current implementations, tests, fixtures, codegen
   snapshots/golden files, GUI commands, and .NET bridge calls affected by the
   IDs.

Do not treat a current implementation, fixture, or generated artifact as
authority when it disagrees with an approved specification. Do not implement a
Draft Type System or any other unapproved future feature merely because its
document exists.

## Implementation flow

### 1. Build an acceptance matrix

For each requirement ID, record:

- the observable behavior;
- success and failure conditions;
- compatibility expectation;
- unit/integration/GUI test;
- fixture or generated artifact evidence, if useful;
- the source file and code boundary expected to own it.

Use requirement IDs in test names or nearby comments when that makes the
mapping clear. A test must verify the approved wording, not silently choose an
answer to an Open Question.

### 2. Identify architecture impact

Keep domain logic in `masterdata-core`, with CLI and GUI sharing that core.
GUI code must not discover files or interpret YAML semantics; use a Tauri
command that calls core. Keep .NET process invocation in
`masterdata-dotnet`. Do not reimplement MasterMemory internals, binary format,
or Source Generator behavior in Rust. Do not move the semantic rule to a
convenient adapter or duplicate it in several layers.

### 3. Plan tests and fixtures

Prefer tests first when the behavior is testable. Add or update
`fixtures/minimal`, `fixtures/full`, or `fixtures/invalid` when a stable
end-to-end input communicates the rule. A focused unit or integration test is
enough for a small rule. Fixtures are fixed inputs; copy them through
`cargo xtask` and never let normal CLI/GUI execution rewrite them.

For generated C#, update snapshot/golden evidence only when it is part of the
approved behavior. Do not add a snapshot merely to bless an unapproved output.

### 4. Implement and verify

Implement the smallest change that satisfies the acceptance matrix. Then run
the relevant unit, integration, frontend, GUI, codegen, and .NET bridge checks.
Finish with `cargo xtask check-all` when the environment supports it. If a
check cannot run, record the exact reason and do not claim complete
verification.

### 5. Reconcile specification and implementation

Before changing status, review every requirement ID against the implementation
and test evidence. The specification may be updated only for a separately
approved semantic change; implementation work must not rewrite the contract to
make an incomplete implementation appear correct.

Change `Status: Approved` to `Status: Implemented` only when:

- all in-scope acceptance criteria have evidence;
- tests and appropriate fixtures are synchronized;
- compatibility behavior is verified or explicitly documented;
- repository checks pass, or unavailable checks are explicitly reported; and
- no unresolved Specification Gap affects the claimed behavior.

## Specification Gap protocol

If an Approved specification leaves behavior necessary for implementation
undefined—such as nullable value objects, missing-reference severity, ordering,
or an error policy—do not choose a domain rule silently. Report:

```text
Specification Gap
- Spec ID / file:
- Missing decision:
- Why implementation cannot proceed safely:
- Non-semantic implementation work that can proceed:
- Proposed route: refine-spec (and review-spec before approval)
```

An internal non-semantic choice, such as a private helper name or allocation
strategy that cannot affect observable behavior, may be made normally. If the
choice could affect public behavior, compatibility, diagnostics, ordering,
serialization, or a user-visible GUI state, it is a specification gap.

## Required completion report

Report:

- target specification IDs and their before/after status;
- acceptance criteria and test/fixture mapping;
- implementation boundaries changed;
- compatibility impact;
- commands run and results, including `cargo xtask check-all`;
- any unimplemented boundary or Specification Gap.

## Non-negotiable safeguards

- Never implement directly from conversation when an approved spec is
  required.
- Never promote a Draft or Proposed spec during implementation.
- Never resolve an Open Question by coding a default.
- Never weaken or strengthen normative language to fit the code.
- Never duplicate core domain semantics in CLI, GUI, or the .NET adapter.
- Never claim `Implemented` without test and verification evidence.
