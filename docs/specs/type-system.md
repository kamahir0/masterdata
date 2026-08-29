# Type system

Status: Draft

The eventual schema model supports the following categories:

- primitives such as bool, signed/unsigned integers, float, double, and
  string;
- immutable value objects wrapping one primitive;
- normal enums with stable numeric wire values;
- `[Flags]` enums for fields only;
- immutable custom records or structs composed of supported types;
- tables whose records are generated as MasterMemory-compatible C# types.

Normative intentions:

### SCHEMA-VO-001

Value objects MUST be immutable, equality-capable, and MessagePack
serializable.

### SCHEMA-ENUM-001

Enum numeric values MUST be treated as persistent wire identities; removed
values MUST NOT normally be reused.

### SCHEMA-FLAGS-001

Flags enums MUST NOT be primary or secondary keys.

### SCHEMA-CUSTOM-001

Generated custom types MUST be immutable.

The initial C# generator only maps primitive field names needed by the minimal
fixture. Custom types are reported as an explicit not-implemented boundary.
