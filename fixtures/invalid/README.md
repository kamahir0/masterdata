# Invalid fixture

This project intentionally omits the required stable `project.id` field from
`masterdata.toml`. It is used to verify that project/config errors are
reported as structured diagnostics instead of being silently accepted.

Additional invalid source snippets live beside this project for focused parser,
validator, and code-generation tests: missing/unknown `kind`, duplicate table
declarations, active/reserved field-ID collisions, and invalid generated C#
identifiers. A field named `id` is intentionally not an implicit primary key.
