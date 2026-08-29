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
tableId: catalog.item
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

Open Questions: schema document splitting, explicit table merge policy, and
whether declarations should use YAML tags or a versioned envelope.

