# Index identity仕様（Index identity）

Status: Draft

このdocumentは、logical Secondary Key identityとgenerated MasterMemory index metadataのcompatibility boundaryを追跡する。
Table / Primary Key / Secondary Keyのcurrent Approved specificationは、[Table / Primary Key / Secondary Key仕様](../table-and-keys.md)がcanonical ownerである。

### COMPAT-INDEX-001

このDraft requirementは、以前提案されていた「Field IDとMasterMemory index numberをmodel内で別物として保持する」というruleのhistoryである。
Field IDをMessagePack `key`へ置き換えたApplied modelでは、旧meaningをcurrent normative requirementとして使用しない。このidentifierは履歴のために
保持し、再利用してはならない（MUST NOT）。

現在のApproved/Applied modelでは、MessagePack field `key`のsemanticsは`SCHEMA-KEY-001`、Secondary Key declaration orderからgenerated `indexNo`へのloweringは
`INDEX-SECONDARY-004`が所有する。`indexNo`はbackend / codegen detailであり、logical Secondary Key identityまたはcompatibility identityではない。

## Open Questions（未解決事項）

- どのindex changeがwire-compatible、generated-API-compatible、またはbreakingか。
- generated numeric index numberに依存せず、referenceがindexを指定するにはどうするか。
