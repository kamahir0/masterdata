# Compatibility仕様

Compatibility ruleは、requirementが一緒にspecification lifecycleを進められるcanonical fileへ
分割して管理する。これにより、1つの広い `Status:` valueが、成熟度の異なる無関係なrequirementへ
誤って適用されることを防ぐ。

- [Table identity仕様](table-identity.md) — current project-local table identity contract。
- [Field identity仕様](field-identity.md) — tableとCustom Typeに共通する現在ApprovedのField ID、rename、tombstone rule。MessagePack専用`key`への置換proposalは [specification change 0003](../../spec-changes/0003-field-identity-to-messagepack-key.md) に記録する。
- [Enum identity仕様](enum-identity.md) — Draftの、Masterdata外のEnum/Flags external compatibility rule。
- [Index identity仕様](index-identity.md) — logical Secondary Key identityとMasterMemory `indexNo`の区別を追跡するDraft document。active proposalのruleは [Table / Primary Key / Secondary Key仕様](../table-and-keys.md) が所有する。

documentを分割または移動してもRequirement IDはstableに保つ。directory structureはdocumentationの
organizationに過ぎず、product semantic meaningを持たない。
