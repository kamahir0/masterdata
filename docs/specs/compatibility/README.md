# Compatibility仕様

Compatibility ruleは、requirementが一緒にspecification lifecycleを進められるcanonical fileへ
分割して管理する。これにより、1つの広い `Status:` valueが、成熟度の異なる無関係なrequirementへ
誤って適用されることを防ぐ。

- [Table identity仕様](table-identity.md) — current project-local table identity contract。
- [Field identity仕様](field-identity.md) — `Status: Deprecated`。旧stable numeric Field ID、rename、tombstoneのretired historyと、現在のMessagePack `key` ownerへのroutingを記録する。specification change 0003はAppliedである。
- [Enum identity仕様](enum-identity.md) — Draftの、Masterdata外のEnum/Flags external compatibility rule。
- [Index identity仕様](index-identity.md) — logical Secondary Key identityとMasterMemory `indexNo`の区別を追跡するDraft document。active ruleは [Approved Table / Primary Key / Secondary Key仕様](../table-and-keys.md) が所有する。

documentを分割または移動してもRequirement IDはstableに保つ。directory structureはdocumentationの
organizationに過ぎず、product semantic meaningを持たない。
