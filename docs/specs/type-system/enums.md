# Enums and flags

Status: Draft

Normal enums and Flags enums are distinct schema categories whose numeric representation and key capabilities require explicit specification.

### SCHEMA-ENUM-001

Enum numeric values MUST be treated as persistent wire identities; removed values MUST NOT normally be reused.

### SCHEMA-FLAGS-001

Flags enums MUST NOT be primary or secondary keys.

## Open Questions

- Which underlying integer types are supported?
- Are explicit numeric values mandatory?
- Is `None = 0` required or recommended for Flags enums?
- Must Flags values be powers of two except named composites?
- Are normal enums always key-compatible?
- What compatibility behavior applies to rename, removal, and numeric-value changes?
