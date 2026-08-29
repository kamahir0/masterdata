# Compatibility specifications

Compatibility rules are split into canonical files whose requirements can move through the specification lifecycle together. This avoids one broad `Status:` value accidentally applying to unrelated requirements at different maturity levels.

- [Table identity](table-identity.md) — current project-local table identity contract.
- [Field identity](field-identity.md) — Draft field-ID and tombstone rules.
- [Enum identity](enum-identity.md) — Draft enum wire-identity rules.
- [Index identity](index-identity.md) — Draft distinction between field IDs and MasterMemory index numbers.

Requirement IDs remain stable when a document is split or moved. Directory structure is documentation organization only and has no product semantic meaning.
