# Primitive types

Status: Draft

Primitive types are the foundation for schema fields and Value Object underlying values. The current scaffold recognizes a small C#-oriented set, but that implementation is not yet the approved product contract.

## Candidate scope

The existing design discussion includes bool, signed and unsigned integers, float, double, and string. Exact spellings, aliases, serialization semantics, range behavior, and Unity/C# compatibility still require refinement.

## Open Questions

- What is the canonical primitive type vocabulary in YAML?
- Are aliases such as `int` / `int32` accepted, normalized, or rejected?
- Are `decimal`, `char`, byte arrays, GUIDs, dates, or other scalar types in scope?
- Which primitive types are key-compatible and orderable?
- How are numeric overflow and coercion handled when parsing data YAML?
- What nullability model applies to primitive fields?
