# RFC: Generated C# naming policy

Status: Proposed

## Context

The current C# generator is a scaffold. It derives names from `table`, accepts
an optional `csharpName`, and turns field names into C# properties and
constructor parameters. Name normalization can create collisions, and C# has
reserved keywords and namespace rules that are not domain semantics.

## Problem

Invalid or colliding generated names can fail only after C# compilation, which
makes diagnostics late and difficult to associate with a schema. A complete
naming policy also needs to distinguish a source field/table identity from its
generated presentation name.

## Goals

- define the questions a future naming specification must answer;
- keep obvious invalid output at the code-generation validation boundary; and
- avoid silently choosing a Unicode, escaping, or rename compatibility policy.

## Non-Goals

This RFC does not approve a final naming convention, rename migration policy,
or Unicode identifier policy. It does not add Type System semantics.

## Options

- reject invalid/reserved names and normalization collisions;
- escape or prefix names that are invalid or reserved; or
- require explicit generated names and avoid normalization for ambiguous
  source names.

## Trade-offs

Rejection is predictable and keeps source identity visible, but asks authors to
rename inputs. Escaping preserves more inputs but changes generated API names
and can make compatibility less obvious. Requiring explicit names gives users
control but increases schema noise and does not remove namespace/file
collisions.

## Proposal

The current scaffold performs a conservative validation boundary: namespace,
type, property, and constructor-parameter names must be valid ASCII C#
identifiers; reserved keywords, normalized type/property/parameter collisions,
and case-insensitive generated filename collisions are errors. This is an
implementation guard, not an Approved naming specification. The eventual
product policy must be refined and reviewed before generated API names become
a compatibility promise.

## Compatibility

Changing normalization, escaping, case sensitivity, file naming, or Unicode
handling can change generated public APIs and filenames. A final decision needs
golden tests and an explicit migration policy.

## Open Questions

- Should reserved keywords be rejected or escaped with `@`?
- Is the current ASCII-only policy acceptable for source names and namespaces?
- What Unicode normalization and case-folding rules apply?
- How are `foo-bar` and `foo_bar` collisions resolved?
- Are type, property, constructor-parameter, namespace, and filename
  collisions all errors?
- Is a C# type rename compatible when the `table` identity is unchanged?
- Which generated API and filename forms are stable compatibility surfaces?

## Decision

Pending explicit human approval. The conservative validation boundary may
prevent obviously invalid output while this RFC remains Proposed.
