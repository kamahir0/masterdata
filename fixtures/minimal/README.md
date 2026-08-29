# Minimal fixture

This fixed project is used by `cargo xtask cli`, `cargo xtask gui`, and the
integration smoke test. `target/dev-project` is recreated from this directory
before those commands run, so the fixture itself is never edited by a
development session.

The schema and data files intentionally share a source directory but declare
their own `kind` and `table`. A second data file for `item` demonstrates that
one table may be split across multiple YAML files.

