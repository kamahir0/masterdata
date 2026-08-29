# Index and reference model

Status: Draft

Tables are planned to support primary keys, composite primary keys, multiple
secondary indexes, unique/non-unique secondary indexes, and composite
secondary indexes wherever MasterMemory makes them practical. Field identity
and index number are separate concepts.

Planned requirement IDs:

### INDEX-PRIMARY-001

A table MAY declare one primary key, including a composite key.

### INDEX-SECONDARY-001

A table MAY declare multiple secondary indexes.

### INDEX-UNIQUE-001

Uniqueness is an explicit property, not inferred from field names.

### REF-001

A MasterReference MUST name a source field and target table/index.

### REF-002

A unique target resolves to one; a non-unique target resolves to many.

### REF-003

References MUST be validated during build and generated helpers MUST receive a
`MemoryDatabase` from the caller rather than storing one on a master record.

Open Questions: exact YAML syntax, nullability, missing-reference severity,
and the generated helper naming policy.
