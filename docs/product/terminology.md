# Terminology

Status: Draft

- **Project**: a directory identified by `masterdata.toml`.
- **Source root**: a configured filesystem search root. It has no table or
  schema meaning by itself.
- **Schema**: the logical declaration of tables, fields, types, and related
  constraints. Its exact language is defined by the owning specifications.
- **Primitive Type**: a scalar type with a directly declared value domain. The
  supported vocabulary is owned by the Primitive Types specification.
- **Field Modifier**: a field-level choice that changes a base type into a
  Required, Nullable, or Array field. Its syntax and presence rules are owned
  by the Field Modifiers specification.
- **Required**: a field shape whose data entry is present and contains a
  non-null value of its base type.
- **Nullable**: a field shape whose data entry is present and may contain
  `null` or a value of its base type.
- **Array**: a field shape whose data entry is present and contains zero or
  more values of its base type; an empty array represents no values.
- **Underlying Type**: the single primitive wrapped by a Value Object. It is
  not inferred from a source file path or generated type name.
- **Type Declaration**: a YAML document that declares one named type category.
- **Type Capability**: an observable permission or behavior, such as direct
  key compatibility, that a type may expose. The type-system specifications
  define capabilities without requiring a particular implementation data
  structure.
- **Schema document**: a YAML file with `kind: schema` that declares a table's
  stable identity and fields.
- **Data**: values and records supplied for tables; data does not redefine the
  meaning of a schema.
- **Data document**: a YAML file with `kind: data` that contributes records to
  the declared table.
- **Table**: a logical collection of records identified by the declared
  project-local `table` identity, independent of a source file's path.
- **Record**: one instance or row belonging to a table.
- **Field**: a named, typed member of a record with a stable field identity
  where the schema requires one.
- **Schema AST**: typed Rust structures representing schema declarations.
- **Data AST**: typed Rust structures for data document shape with YAML values
  retained at field leaves until type resolution.
- **Table identity**: the project-local stable identity carried by the `table`
  field. It is distinct from a generated C# type name and from a source file
  path. The current scaffold does not define a second `tableId` identity;
  global identity, rename migration, released compatibility, legacy migration,
  and cross-project identity remain open in the table-identity RFC.
- **Generated C# type name**: a presentation/code-generation name, supplied by
  `csharpName` when present or derived by the generator when absent. It MAY be
  changed independently only where the compatibility specification permits.
- **Field ID**: persistent MessagePack integer-key identity, separate from a
  MasterMemory index number.
- **Value Object**: an immutable domain type that wraps one underlying value.
  The Value Objects specification proposes that the underlying value is one
  primitive and defines its generated representation and capabilities.
- **Enum**: a named type whose declared members have stable numeric values
  when the type-system and compatibility specifications require them.
- **Flags Enum**: an enum intended to represent bitwise combinations; its
  permitted use as a field or key is defined by the type-system specification.
- **Custom Type**: a user-declared composite type made from supported types.
- **Index**: a lookup structure declared for a table; its key shape and
  generated behavior belong to the index specification.
- **Primary Key**: the index that identifies records for a table, subject to
  the owning specification's cardinality rules.
- **Secondary Key**: an additional lookup key for a table.
- **Unique**: an index property requiring at most one matching record.
- **Non-Unique**: an index property allowing multiple matching records.
- **MasterReference**: a declared relationship from one table field to another
  table/index.
- **Fixture**: a fixed, version-controlled test input. Tools may copy it to a
  development project, but normal CLI/GUI execution MUST NOT rewrite it.
- **Generated Artifact**: a reproducible output such as generated C# or a
  binary, derived from source inputs and not authoritative for edits.
- **Source of Truth**: the canonical input that humans edit and review. For
  project schema/data, the YAML documents are the source of truth.
- **Build plan**: the validated, hashed input passed from Rust toward the .NET
  builder boundary.
- **Schema source-content hash**: a deterministic hash of schema source bytes
  in the current scaffold. It is not a semantic schema hash or a builder cache
  key.
- **Semantic schema hash**: a future hash of canonical parsed/resolved schema
  meaning; it is not currently implemented.
- **Builder cache key**: a future composite identity for reusable builder
  output. It is distinct from both source-content and semantic schema hashes.
- **Builder**: the .NET-side process that will eventually compile generated C#,
  invoke MasterMemory v3 Source Generator, and write a binary.
