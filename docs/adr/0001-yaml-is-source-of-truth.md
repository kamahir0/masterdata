# ADR 0001: YAML is the source of truth

Status: Accepted

## Context

Generated C# and MasterMemory binary artifacts are derived outputs. Treating a
generated artifact as authoritative would make review, regeneration, and
compatibility tracking unreliable.

## Decision

YAML schema and data documents are the canonical source of truth. Generated C#,
builder output, caches, and MasterMemory binaries are reproducible artifacts
and MUST NOT be the authority for edits.

## Consequences

Changes are reviewable in text and can be regenerated in CI. The system needs
stable IDs and compatibility validation in YAML. GUI edits must write YAML (or
an explicit future transaction format), never silently patch generated output.

