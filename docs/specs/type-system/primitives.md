# Primitive Types

Status: Proposed

Domain: Type System

## Summary

This proposal defines the initial primitive type vocabulary, direct key
compatibility, strict scalar validation, and the initial string rules. It does
not define a YAML parser dialect or the implementation data structures used to
represent capabilities.

## Terminology

The terms Primitive Type, Field, and Value Object follow
[product terminology](../../product/terminology.md). Field-level presence and
modifier behavior belongs to [Field Modifiers](field-modifiers.md).

## Normative Requirements

### TYPE-PRIMITIVE-001

The initial primitive type vocabulary MUST support exactly these canonical
type names:

| Type name | Value domain |
| --- | --- |
| `bool` | Boolean values |
| `int` | Signed 32-bit integer values |
| `uint` | Unsigned 32-bit integer values |
| `long` | Signed 64-bit integer values |
| `ulong` | Unsigned 64-bit integer values |
| `float` | Single-precision floating-point values |
| `double` | Double-precision floating-point values |
| `string` | String values |

The canonical names in this table are the names used when a field declares a
primitive type.

### TYPE-PRIMITIVE-002

The initial primitive type vocabulary MUST NOT support `byte`, `sbyte`,
`short`, `ushort`, `decimal`, `char`, `Guid`, or `DateTime`. A future
specification MAY add further primitive types without changing the meaning of
the initial vocabulary.

### TYPE-PRIMITIVE-003

A data scalar for a field declared with a supported primitive MUST match the
declared primitive's scalar category and representable value. Validation MUST
NOT implicitly coerce a scalar between primitive types. In particular, `1.0`
MUST NOT be accepted as `int`, a negative value MUST NOT be accepted as
`uint`, and a value outside the declared integer range MUST NOT be accepted.

The exact boundary between a YAML parser's scalar classification and this
validation rule remains an Open Question; the rule does not authorize the
type system to reinterpret a parser classification silently.

### TYPE-PRIMITIVE-004

The integer ranges for the initial vocabulary MUST be the ranges of their
named domains: `int` is -2^31 through 2^31-1, `uint` is 0 through 2^32-1,
`long` is -2^63 through 2^63-1, and `ulong` is 0 through 2^64-1. Values
outside those ranges MUST be rejected rather than narrowed, wrapped, or
otherwise converted.

### TYPE-PRIMITIVE-005

When a primitive is evaluated for direct MasterMemory Primary Key or Secondary
Key compatibility, `int`, `uint`, `long`, `ulong`, and `string` MUST be
classified as key-compatible. `bool`, `float`, and `double` MUST be
classified as key-incompatible.

This requirement defines the primitive capability only; the declaration and
validation of indexes belong to the future index specification.

### TYPE-PRIMITIVE-006

The empty string `""` MUST be a valid `string` value. Primitive string
validation MUST NOT reject a value solely because it is empty. Whether a field
accepts `null` is owned by the field modifier rules, not by the `string`
primitive itself.

## Validation Rules

The observable validation outcomes for this proposal are defined by
`TYPE-PRIMITIVE-001` through `TYPE-PRIMITIVE-006`: unsupported names,
scalar-category mismatches, integer range violations, invalid direct key
capability, and empty-string acceptance. Exact diagnostic codes and
source-location mapping are not assigned by this proposal.

## Acceptance Evidence

| Requirement | Success observation | Failure observation | Suggested evidence |
| --- | --- | --- | --- |
| `TYPE-PRIMITIVE-001` | Each initial canonical name resolves to its declared domain. | A name outside the initial vocabulary is not treated as one of these primitives. | Type vocabulary table test. |
| `TYPE-PRIMITIVE-002` | Future-only names remain outside the initial profile. | Each listed excluded name is rejected as an initial primitive. | Unsupported-name validation test. |
| `TYPE-PRIMITIVE-003` | A scalar with the declared category and representation is accepted. | `1.0` for `int`, a negative `uint`, or an implicit type conversion is rejected. | Strict scalar validation tests. |
| `TYPE-PRIMITIVE-004` | Boundary values for each integer domain are accepted. | Values just outside each range are rejected without wrapping or narrowing. | Integer boundary tests. |
| `TYPE-PRIMITIVE-005` | The five listed key-compatible primitives are classified as compatible. | `bool`, `float`, and `double` are classified as incompatible. | Capability classification test. |
| `TYPE-PRIMITIVE-006` | `""` is accepted as a string value. | No failure is reported solely because a string is empty. | Empty-string validation test. |

## Compatibility

This proposal adds no implementation or released-schema migration. Primitive
names and scalar representations will affect generated C# and serialized data
once implemented, so released-schema compatibility is an Open Question. The
exact YAML numeric grammar and parser behavior must be decided before an
implementation claims full scalar compatibility.

## Examples

The following are non-normative examples of the intended validation boundary:

```yaml
count: 1
ratio: 1.0
name: ""
```

`count: 1` can be an `int`, `ratio: 1.0` can be a floating-point value, and
an empty `name` remains a valid string. A field's declared type determines
which of those values is valid; the examples do not define a YAML parser
dialect.

## Open Questions

- Which YAML scalar classifications are authoritative when parser libraries
  disagree?
- Are hexadecimal, octal, binary, exponent, and other non-decimal numeric
  forms accepted for integer and floating-point primitives?
- Are `NaN`, positive infinity, and negative infinity accepted for `float` and
  `double`?
- How are timestamp-looking scalars treated when the declared type is
  `string` or a numeric primitive?
- Should the proposed .NET-style 32-bit and 64-bit range mapping in
  `TYPE-PRIMITIVE-004` be the product contract, or should another target
  domain define these ranges?
- Will future compatibility aliases for primitive names be allowed?
- What exact diagnostic code and source span should represent each scalar
  validation failure?

## Non-Goals

This proposal does not implement a Rust type registry, scalar parser,
nullable/array validator, enum, flags enum, custom type, index, reference,
MessagePack generator, or production binary builder.
