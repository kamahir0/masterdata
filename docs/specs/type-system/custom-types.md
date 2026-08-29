# Custom types

Status: Draft

Custom types are intended to compose supported schema types into immutable generated C# values. Their exact shape and recursion rules are not yet approved.

### SCHEMA-CUSTOM-001

Generated custom types MUST be immutable.

## Open Questions

- Which field types may custom types contain?
- How are `record` versus `struct` choices represented in YAML?
- Are recursive custom types forbidden entirely or only where C# value-type recursion is invalid?
- How are field IDs assigned and serialized inside custom types?
- What nullability/default-value rules apply?
- Which custom types, if any, may be key-compatible?
