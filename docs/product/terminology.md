# Terminology

Status: Draft

- **Project**: a directory identified by `masterdata.toml`.
- **Source root**: a configured filesystem search root. It has no table or
  schema meaning by itself.
- **Schema**: the logical declaration of tables, fields, types, and related
  constraints. Its exact language is defined by the owning specifications.
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
  path. The current Rust schema model does not define a second `tableId`
  identity; the compatibility implications are tracked by the table-identity
  RFC.
- **Generated C# type name**: a presentation/code-generation name, supplied by
  `csharpName` when present or derived by the generator when absent. It MAY be
  changed independently only where the compatibility specification permits.
- **Field ID**: persistent MessagePack integer-key identity, separate from a
  MasterMemory index number.
- **Value Object**: an immutable domain type that wraps a value according to
  the type-system specification. Whether a particular underlying type is
  allowed is not inferred from this glossary.
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
