# Index identity

Status: Draft

This document isolates compatibility rules concerning schema field identity and generated MasterMemory index metadata.

### COMPAT-INDEX-001

Field IDs and MasterMemory index numbers MUST remain distinct in the model.

## Open Questions

- What stable logical identity does a secondary index have in the source schema?
- How are generated MasterMemory `indexNo` values assigned deterministically?
- Which index changes are wire-compatible, generated-API-compatible, or breaking?
- How do references target indexes without depending on generated numeric index numbers?
