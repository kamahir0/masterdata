# Enum identity

Status: Draft

This document isolates enum compatibility rules that are not yet approved.

### COMPAT-ENUM-001

Enum numeric values are stable wire identities and removed values MUST NOT normally be reused.

## Open Questions

- Which underlying integer types are supported?
- Is numeric reuse absolutely forbidden or only forbidden within a compatibility window?
- How are renamed enum members represented in compatibility reports?
- How are Flags enum values handled differently, if at all, for compatibility analysis?
