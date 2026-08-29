# EnumとFlags

Status: Draft

通常のEnumとFlags Enumは別のschema categoryであり、numeric representationとkey capabilityを
明示的に仕様化する必要がある。

### SCHEMA-ENUM-001

Enumのnumeric valueはpersistentなwire identityとして扱わなければならず（MUST）、削除したvalueは
通常再利用してはならない（MUST NOT）。

### SCHEMA-FLAGS-001

Flags Enumはprimary keyまたはsecondary keyであってはならない（MUST NOT）。

## Open Questions（未解決事項）

- どのunderlying integer typeをサポートするか。
- 明示的なnumeric valueを必須とするか。
- Flags Enumでは `None = 0` を必須または推奨とするか。
- named compositeを除き、Flags valueは2の累乗でなければならないか。
- 通常のEnumは常にkey-compatibleか。
- rename、削除、numeric value変更にどのcompatibility behaviorを適用するか。
