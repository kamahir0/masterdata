# Value Objects

Status: Proposed

Domain: Type System

## Summary

This proposal defines a Value Object as a named immutable wrapper around one
primitive value. It defines the unified type-declaration boundary, scalar data
representation, generated C# category, equality, capability inheritance,
directional implicit-conversion configuration, and underlying-value
`ToString()` behavior. Enums, Flags Enums, and Custom Types remain separate
future type categories.

## Terminology

The terms Value Object, Primitive Type, Type Declaration, and Generated C#
Type Name follow [product terminology](../../product/terminology.md). The
primitive capability table is owned by [Primitive Types](primitives.md), and
field-level Nullable/Array behavior is owned by [Field Modifiers](field-modifiers.md).

`SCHEMA-VO-001` is retained from the earlier Draft type-system overview and
is refined here as the canonical Value Object requirement. Approval and
implementation should preserve the dependency on the referenced primitive and
field-modifier contracts without merging their file-level statuses.

## Normative Requirements

### SCHEMA-VO-001

A Value Object MUST be immutable, equality-capable, and serializable through
the repository's future MessagePack integration. The generated representation
and serialization details in this proposal MUST preserve those observable
properties without making generated artifacts the source of truth.

### SCHEMA-VO-002

A Value Object MUST wrap exactly one underlying primitive type. Its underlying
type MUST be selected from the primitive vocabulary defined by
`TYPE-PRIMITIVE-001`.

### SCHEMA-VO-003

A Value Object MUST NOT wrap another Value Object, an Enum, a Flags Enum, a
Custom Type, an Array shape, or a Nullable shape. A Value Object's underlying
type is a primitive before field-level modifiers are applied.

### SCHEMA-VO-004

Value Objects MUST use the unified type-declaration model rather than a
Value-Object-only top-level declaration family. One YAML document MUST contain
exactly one type declaration, and a type document's path or filename MUST NOT
determine the type identity.

The proposed surface for one Value Object declaration is:

```yaml
kind: type
name: ItemId
valueObject:
  underlying: int
```

This surface reserves the same type-document boundary for future Enum, Flags
Enum, and Custom Type categories without defining those categories here.

### SCHEMA-VO-005

The data representation of a Value Object MUST use the same scalar
representation as its underlying primitive. For example, an `ItemId` wrapping
`int` is represented as `itemId: 1001`, not as a wrapper object containing a
`value` property.

### SCHEMA-VO-006

The generated C# representation of every Value Object MUST be a `readonly
struct`. A Value Object MUST NOT be selectable as a `class`, `record`, or
`record struct` through per-type representation options.

The exact accessibility, member names, constructor signature, and MessagePack
attributes are not fixed by this requirement.

### SCHEMA-VO-007

A Value Object MUST generate equality behavior. For each capability explicitly
defined for the underlying primitive by an approved type-system specification,
a Value Object MUST inherit that capability rather than independently opting
in or out. In particular:

- key compatibility MUST be inherited from the underlying primitive's
  `TYPE-PRIMITIVE-005` classification; and
- ordering or comparison capability MUST be inherited when the underlying
  primitive provides that capability.

Therefore, `ItemId(int)` and `UserCode(string)` are key-compatible, while
`Ratio(double)` and `Enabled(bool)` are key-incompatible. Nullable and Array
field modifiers still make a field shape key-incompatible under
`TYPE-FIELD-004`.

### SCHEMA-VO-008

A Value Object declaration MUST be able to independently enable or disable
implicit conversion in each of these directions:

- underlying primitive to Value Object; and
- Value Object to underlying primitive.

All four combinations of those two independent capabilities MUST be
representable. When no conversion setting is supplied, both capabilities MUST
be disabled. The exact YAML property names and placement used to express these
choices are not fixed by this requirement.

### SCHEMA-VO-009

The directional implicit-conversion settings MUST affect only generated C# API
conversion behavior. They MUST NOT alter key compatibility, comparison
capability, equality, MessagePack wire identity, underlying primitive
identity, or field modifier semantics. Key compatibility continues to be
determined by `TYPE-PRIMITIVE-005` and `TYPE-FIELD-004`, independently of
conversion settings.

### SCHEMA-VO-010

The generated `ToString()` for a Value Object MUST return the textual
representation of its underlying value. It MUST NOT use a type-name-wrapped
debug representation such as `ItemId(1001)` as the default contract. The
culture, provider, and other formatting details of that textual
representation remain outside this requirement.

## Validation Rules

The observable validation outcomes for this proposal are defined by
`SCHEMA-VO-001` through `SCHEMA-VO-010`: underlying type and wrapper
restrictions, one-document/one-declaration structure, scalar representation,
readonly-struct category, equality, inherited capabilities, directional
conversion configuration, conversion isolation, and underlying-value
`ToString()` behavior. Exact diagnostic codes and the final MessagePack
attribute shape remain unassigned.

## Acceptance Evidence

| Requirement | Success observation | Failure observation | Suggested evidence |
| --- | --- | --- | --- |
| `SCHEMA-VO-001` | A Value Object is immutable, equality-capable, and serializable through the approved integration. | A mutable, non-equality-capable, or non-serializable representation is rejected. | Generated representation and serialization tests. |
| `SCHEMA-VO-002` | One declared primitive is accepted as the underlying type. | A missing or non-primitive underlying type is rejected. | Type declaration validation tests. |
| `SCHEMA-VO-003` | A direct primitive wrapper is accepted. | Nested, enum, custom, array, or nullable wrappers are rejected. | Forbidden-wrapper tests. |
| `SCHEMA-VO-004` | One type document yields one named type declaration independent of its path. | Multiple declarations or path-derived identity are rejected. | Type document structure tests. |
| `SCHEMA-VO-005` | A Value Object field uses the underlying primitive scalar representation. | A wrapper object with a `value` property is not required or accepted as the defined representation. | Data representation tests. |
| `SCHEMA-VO-006` | Generated output uses a readonly struct category. | A per-type class, record, or record struct selection is not accepted. | C# generation golden/compile test. |
| `SCHEMA-VO-007` | Equality and inherited capability outcomes match the underlying primitive. | An independent capability override produces a mismatch. | Capability inheritance tests. |
| `SCHEMA-VO-008` | The declaration model represents `false/false`, `true/false`, `false/true`, and `true/true` independently; omitted settings produce `false/false`. | Enabling one direction implicitly enables the other, or omitted settings enable either direction. | Conversion-capability model tests. |
| `SCHEMA-VO-009` | Changing conversion settings changes only the generated C# conversion surface. | Key compatibility, comparison, equality, wire identity, underlying identity, or field modifier behavior changes with conversion settings. | Conversion-isolation tests. |
| `SCHEMA-VO-010` | A Value Object's `ToString()` returns the underlying textual value, such as `"1001"` for an `ItemId` wrapping `1001`. | The default result adds a type-name wrapper such as `ItemId(1001)`. | Generated API behavior test, with formatting cases added after the culture policy is decided. |

## Compatibility

The scalar data representation avoids adding a Value Object wrapper node, but
the generated C# API and MessagePack wire shape can still affect compatibility.
This proposal does not define implicit migration, released-schema
compatibility, constructor compatibility, or field-ID behavior. Those choices
remain Open Questions and are not implementation authority until this proposal
is approved.

## Examples

The following examples are non-normative:

```yaml
# type declaration
kind: type
name: UserCode
valueObject:
  underlying: string
```

```yaml
# data value in a table record
userCode: "player-001"
```

The following is a non-normative candidate syntax for the two directional
conversion settings. It illustrates four independent states but intentionally
does not establish the property names or their placement as part of the
contract:

```yaml
kind: type
name: ItemId
valueObject:
  underlying: int
  conversions:
    fromUnderlyingImplicit: false
    toUnderlyingImplicit: false
```

The following declarations are invalid under this proposal because the
underlying type is not a primitive:

```yaml
kind: type
name: NestedId
valueObject:
  underlying: ItemId
```

```yaml
kind: type
name: NullableId
valueObject:
  underlying: int?
```

## Open Questions

- What exact YAML properties and placement express the two directional
  implicit-conversion settings?
- Should explicit conversion operators or helper APIs also be generated, and
  what compatibility guarantees would they have?
- Which culture, format provider, or invariant-formatting policy applies to
  the textual representation returned by `ToString()` for numeric underlying
  values?
- What exact constructor and public member API must a generated readonly
  struct expose?
- What MessagePack attributes and generated shape are required by MasterMemory
  and Unity-compatible C# projects?
- How do nullable reference types and nullable value types differ in the
  generated C# contract?
- Which ordering/comparison capabilities exist for each primitive, especially
  for `bool`, `float`, and `double`? This includes the ordering contract for
  finite floating-point values; the primitive specification rejects non-finite
  values.
- What custom validation constraints may be added without changing the
  primitive wrapper contract?
- How are Value Object additions, underlying-type changes, and renames
  classified against released schemas?
- Are the exact `kind`, `name`, and `valueObject.underlying` key names final,
  or should the unified type declaration use another spelling?

## Non-Goals

This proposal does not implement a type registry, AST/IR resolver, Value
Object parser, nullable/array validator, readonly-struct generator, MessagePack
generator, key generator, Enum, Flags Enum, Custom Type, Index,
MasterReference, or production binary builder.
