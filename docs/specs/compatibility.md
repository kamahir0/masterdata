# Compatibility and stable identity

Status: Draft

Names are presentation and code-generation concerns. Wire compatibility uses
stable identities.

### COMPAT-TABLE-001

The current proposed direction is for `table` to be the project-local stable
table identity and for the generated C# type name to remain a separate,
renameable presentation name. This direction is not approved as a broader
compatibility contract until the table identity proposal is reviewed.

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
identifiers need a globally stable namespace, and whether the proposed
`table`/`csharpName` split should be adopted as the compatibility contract.
