# Compatibility and stable identity

Status: Draft

Names are presentation and code-generation concerns. Wire compatibility uses
stable identities.

### COMPAT-TABLE-001

For the current scaffold, `table` is the project-local stable table identity
and the generated C# type name is a separate, renameable presentation name.
This requirement does not define global identity, table rename migration,
released-schema compatibility, legacy `tableId` migration, or cross-project
identity.

### COMPAT-FIELD-001

Field IDs are intended for MessagePack integer keys and MUST be unique within a
table.

### COMPAT-FIELD-002

Active field IDs MUST NOT be reused after removal.

### COMPAT-FIELD-003

Removed IDs MAY be retained as `reservedFields` tombstones carrying former
name/type information.

### COMPAT-ENUM-001

Enum numeric values are stable wire identities and removed values MUST NOT
normally be reused.

### COMPAT-INDEX-001

Field IDs and MasterMemory index numbers MUST remain distinct in the model.

The GUI SHOULD auto-assign field IDs and hide that bookkeeping during normal
editing. Advanced users and review tooling still need to see the resulting
stable IDs.

Open Questions: compatibility levels, migration tooling, whether table
identifiers need a globally stable namespace, and what released-schema
compatibility guarantees (if any) should apply to the `table`/`csharpName`
split.
