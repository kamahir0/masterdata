# Schema language

Status: Draft

## Document envelope

Every YAML source document MUST declare its meaning itself:

```yaml
kind: schema
table: item
```

or:

```yaml
kind: data
table: item
```

An unknown or missing `kind` is a parse error. A table MAY receive data from
one or more data documents. Data files are merged by declared table identity,
not by filename or directory.

## Planned schema shape

```yaml
kind: schema
table: item
csharpName: ItemMaster
fields:
  - id: 0
    name: id
    type: ItemId
reservedFields:
  - id: 1
    formerName: oldName
    formerType: string
```

The Rust AST keeps schema declarations typed (`SchemaDocument`,
`FieldDefinition`, and `ReservedField`) so future type/index/reference
features do not collapse into an unstructured map.

The current scaffold uses `table` as the project-local logical table identity.
`csharpName`, when present, is a generated C# type-name override and is not a
second table identity. The previously shown `tableId` field was not consumed by
the Rust model or generator and is intentionally absent from the current
scaffold. The compatibility implications of this direction are tracked in
[`docs/rfcs/0001-table-identity.md`](../rfcs/0001-table-identity.md), which is
not approved.

## YAML subset open questions

The product has not yet approved a YAML subset. Refinement MUST leave these
choices visible rather than deriving them from the current parser:

- whether anchors and aliases are allowed;
- whether merge keys are allowed;
- whether multiple documents separated by `---` are allowed in one file;
- whether custom tags are allowed;
- how duplicate mapping keys are diagnosed;
- how numeric and timestamp-looking scalars are interpreted; and
- whether unknown fields are rejected, ignored, or preserved for round-trip
  editing.

Open Questions also include whether comments, formatting, and quoting must be
preserved when the GUI writes YAML back. Parser/library selection is tracked
separately in [`docs/rfcs/0002-yaml-parser-library.md`](../rfcs/0002-yaml-parser-library.md).
