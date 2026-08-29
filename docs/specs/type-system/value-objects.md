# Value objects

Status: Draft

Value Objects are intended to wrap exactly one primitive value and provide a distinct schema/domain type. The detailed contract still requires refinement before implementation authority exists.

### SCHEMA-VO-001

Value objects MUST be immutable, equality-capable, and MessagePack serializable.

## Open Questions

- Must every Value Object wrap exactly one primitive value?
- Which primitive types may be used as underlying values?
- Which Value Objects are key-compatible and what ordering contract is required?
- What C# representation is generated for current Unity compatibility?
- What conversions, constructors, parsing helpers, or formatting APIs are generated?
- What nullability/default-value semantics apply?
- What stable serialized representation is required for compatibility?
