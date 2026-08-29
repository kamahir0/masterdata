# Compatibility and stable identity

Status: Draft

Names are presentation and code-generation concerns. Wire compatibility uses
stable identities.

- `COMPAT-TABLE-001`: every table SHOULD have a persistent table ID distinct
  from its generated C# name.
- `COMPAT-FIELD-001`: field IDs are intended for MessagePack integer keys and
  MUST be unique within a table.
- `COMPAT-FIELD-002`: active field IDs MUST NOT be reused after removal.
- `COMPAT-FIELD-003`: removed IDs MAY be retained as `reservedFields` tombstones
  carrying former name/type information.
- `COMPAT-ENUM-001`: enum numeric values are stable wire identities and
  removed values MUST NOT normally be reused.
- `COMPAT-INDEX-001`: field IDs and MasterMemory index numbers MUST remain
  distinct in the model.

The GUI SHOULD auto-assign field IDs and hide that bookkeeping during normal
editing. Advanced users and review tooling still need to see the resulting
stable IDs.

Open Questions: compatibility levels, migration tooling, and whether table ID
is mandatory immediately or becomes mandatory at first release.

