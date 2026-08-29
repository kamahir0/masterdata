# Invalid fixture

This project intentionally omits the required stable `project.id` field from
`masterdata.toml`. It is used to verify that project/config errors are
reported as structured diagnostics instead of being silently accepted.

Future invalid cases will live beside this one: duplicate primary keys,
invalid references, invalid schemas, and reserved-field reuse.

