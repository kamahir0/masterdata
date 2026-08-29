# Field identity

Status: Draft

This document isolates the field-identity rules that still require refinement and explicit approval. Existing scaffold behavior is evidence to review, not authority for these Draft requirements.

### COMPAT-FIELD-001

Field IDs are intended for MessagePack integer keys and MUST be unique within a table.

### COMPAT-FIELD-002

Active field IDs MUST NOT be reused after removal.

### COMPAT-FIELD-003

Removed IDs MAY be retained as `reservedFields` tombstones carrying former name/type information.

## Non-normative product direction

The GUI is expected to make stable-ID bookkeeping unobtrusive during normal editing while still allowing advanced users and review tooling to inspect persisted IDs. Exact allocation, visibility, migration, and wire-compatibility behavior remains to be specified.

## Open Questions

- Is numeric MessagePack key generation mandatory for all persisted schema fields?
- What is the exact allocation policy for new field IDs?
- Is reuse permanently forbidden, or only forbidden within a defined compatibility window?
- Which tombstone metadata is required versus optional?
- How do nullability, default values, type changes, and custom-type evolution affect compatibility?
