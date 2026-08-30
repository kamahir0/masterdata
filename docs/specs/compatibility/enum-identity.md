# Enum identity仕様（Enum identity）

Status: Draft

この文書は、Enum/Flagsのcurrent schema semanticsではなく、Masterdata外のlong-lived external contractに関する将来の
compatibility ruleを切り出して管理する。current schemaにおけるEnum/Flagsのnumeric value、key/comparison capability、data
representationは、[EnumとFlags Enum仕様](../type-system/enums.md)がcanonical ownerである。特に、`SCHEMA-ENUM-001` はnumeric
valueをMasterdata binaryをまたぐpersistent identityとして扱わないことを定め、このDraft文書がそれに反するruleを上書きしては
ならない。

## Retired requirement history

`COMPAT-ENUM-001` は、以前この文書で提案されていた「Enumのnumeric valueをstableなwire identityとし、削除valueを再利用しない」
というDraft requirementのidentifierである。その意味は `SCHEMA-ENUM-001` によって明示的に置き換えられた。identifierはhistoryのために
保持し、再利用してはならない（MUST NOT）。

## Open Questions（未解決事項）

- generated Enum/Flags numeric valueを外部のsave data、network protocol、external database、public APIが永続化した場合に、どの
  external compatibility ruleを適用するか。
- 外部契約でmember rename、member deletion、numeric value変更、deleted value再利用をどのようにreportするか。
- external compatibility analysisでFlagsのbit valueをどのように扱うか。
