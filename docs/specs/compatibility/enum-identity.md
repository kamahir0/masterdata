# Enum identity仕様（Enum identity）

Status: Draft

この文書は、まだ承認されていないenum compatibility ruleを切り出して管理する。

### COMPAT-ENUM-001

Enumのnumeric valueはstableなwire identityであり、削除したvalueは通常再利用してはならない（MUST NOT）。

## Open Questions（未解決事項）

- どのunderlying integer typeをサポートするか。
- numeric valueの再利用を絶対に禁止するのか、それともcompatibility window内だけ禁止するのか。
- renameされたenum memberをcompatibility reportでどう表現するか。
- Flags enumのvalueをcompatibility analysisでどのように扱うか。
