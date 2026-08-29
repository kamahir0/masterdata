# Field Modifiers

Status: Proposed

Domain: Type System

## Summary

This proposal defines the field-level representation of Required, Nullable,
and Array values for a supported base type. It keeps modifiers out of the
type-name string and defines field presence independently from the value
stored in a field.

## Terminology

For this document, `T` means one supported field value type, such as a
primitive or a Value Object after its own specification is available. A
**base type** is the type named by the field's `type` property. A **field
modifier** is the field-level choice between Required, Nullable, and Array.
These terms extend [product terminology](../../product/terminology.md).

## Normative Requirements

### TYPE-FIELD-001

Every supported field value type MUST have one of these modifier shapes:

- `T` — Required;
- `T?` — Nullable; or
- `T[]` — Array.

The combinations `T?[]` and `T[]?` MUST NOT be supported. Nullable and Array
are mutually exclusive modifiers in this proposal. When neither modifier is
active, the field has the Required shape `T`.

### TYPE-FIELD-002

Field modifiers MUST be expressed as field-level options, not embedded in the
type-name string. For a base type `T`, the proposed YAML surface is:

```yaml
fields:
  - id: 0
    name: rewardItem
    type: ItemId
    nullable: true
  - id: 1
    name: rewards
    type: ItemId
    array: true
```

The `type` value MUST name the base type without `?` or `[]` modifier
suffixes. Setting both `nullable: true` and `array: true` MUST be a schema
validation error.

### TYPE-FIELD-003

The field entry in a data record MUST be present for Required, Nullable, and
Array fields alike.

- A Required field MUST contain a non-null value valid for `T`.
- A Nullable field MAY contain `null`; when it is non-null, the value MUST be
  valid for `T`. Omitting the field MUST be invalid.
- An Array field MUST contain an array whose elements are valid for `T`.
  An empty array `[]` MUST be valid, `null` MUST be invalid, and omitting the
  field MUST be invalid.

### TYPE-FIELD-004

A Nullable field and an Array field MUST be key-incompatible for direct
MasterMemory Primary Key or Secondary Key use, regardless of the key
compatibility of their base type. In particular, `ItemId?` and `ItemId[]` are
key-incompatible shapes when those declarations are supported.

This requirement defines the modifier effect on capability; the declaration
and validation of indexes belong to the future index specification.

## Validation Rules

The observable validation outcomes for this proposal are defined by
`TYPE-FIELD-001` through `TYPE-FIELD-004`: mutually exclusive shapes,
field-level syntax, presence/null/empty-array behavior, and key capability.
The exact diagnostic codes, source paths, and parser-specific YAML node
classification remain unassigned here.

## Acceptance Evidence

| Requirement | Success observation | Failure observation | Suggested evidence |
| --- | --- | --- | --- |
| `TYPE-FIELD-001` | `T`, `T?`, and `T[]` are distinct supported shapes. | `T?[]` and `T[]?` are rejected. | Modifier-shape validation tests. |
| `TYPE-FIELD-002` | Field-level `nullable` or `array` selects the shape while `type` remains the base name. | Suffix syntax or both enabled options produce a schema validation failure. | Schema syntax tests. |
| `TYPE-FIELD-003` | Required values, nullable `null`, and array `[]` are accepted when the field entry exists. | Omission, invalid `null`, or invalid array shape is rejected. | Data presence and null/array tests. |
| `TYPE-FIELD-004` | Modifier capability is reported independently of the base primitive. | Nullable and Array shapes cannot be used as direct keys. | Key-capability tests after index declarations exist. |

## Compatibility

Field modifiers change the accepted data shape and will affect generated C#
and serialized representation once implemented. Existing field identity rules
remain owned by the [field identity specification](../compatibility/field-identity.md),
including the separation between field IDs and index numbers. Released-schema
compatibility and migration classification for adding or changing a modifier
remain Open Questions.

## Examples

The following examples are non-normative illustrations:

```yaml
# Required: the key must be present and the value cannot be null.
rewardItem: 1001

# Nullable: the key must be present, and null is a value.
optionalReward: null

# Array: the key must be present; no values is represented by [].
rewards: []
```

The following shape is invalid because it enables both modifiers:

```yaml
type: ItemId
nullable: true
array: true
```

## Open Questions

- Are explicit `nullable: false` and `array: false` accepted, or must an
  inactive option be omitted?
- What exact YAML node classification is required for arrays and nulls across
  parser candidates?
- How are nullable reference types and nullable value types represented in
  generated C# while preserving Unity compatibility?
- What released-schema compatibility classification applies when a modifier
  is added, removed, or changed?
- What exact diagnostic code and source path identify a missing field,
  invalid null, or invalid array element?
- Can future type categories introduce additional modifiers, and if so how
  will they remain distinct from the three initial shapes?

## Non-Goals

This proposal does not implement schema parsing, nullable or array validation,
Value Object resolution, index generation, MessagePack attributes, C#
generation, or MasterMemory binary generation.
