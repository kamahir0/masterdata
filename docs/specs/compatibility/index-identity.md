# Index identity仕様（Index identity）

Status: Draft

この文書は、schema field identityとgenerated MasterMemory index metadataに関するcompatibility ruleを
切り出して管理する。

### COMPAT-INDEX-001

Field IDとMasterMemory index numberはmodel内で別物として保持しなければならない（MUST）。

## Open Questions（未解決事項）

- source schemaにおいて、secondary indexが持つstable logical identityは何か。
- generated MasterMemory `indexNo` をdeterministicに割り当てる方法は何か。
- どのindex changeがwire-compatible、generated-API-compatible、またはbreakingか。
- generated numeric index numberに依存せず、referenceがindexを指定するにはどうするか。
