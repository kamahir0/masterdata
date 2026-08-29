# Full reference fixture

This directory is the planned dogfood project for the complete schema model.
It is intentionally descriptive while the initial parser and builder are
still being expanded. The eventual fixture will exercise:

- value objects and immutable custom types
- normal and flags enums
- primary and composite primary keys
- unique, non-unique, and composite secondary indexes
- one-to-one and one-to-many `MasterReference`

The current files use declarations that the core can preserve in its schema
AST. Type resolution, index materialization, reference helpers, and
MasterMemory generation remain open implementation work.

