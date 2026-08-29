# Compatibility仕様

Compatibility ruleは、requirementが一緒にspecification lifecycleを進められるcanonical fileへ
分割して管理する。これにより、1つの広い `Status:` valueが、成熟度の異なる無関係なrequirementへ
誤って適用されることを防ぐ。

- [Table identity仕様](table-identity.md) — current project-local table identity contract。
- [Field identity仕様](field-identity.md) — Draftのfield-IDとtombstone rule。
- [Enum identity仕様](enum-identity.md) — Draftのenum wire-identity rule。
- [Index identity仕様](index-identity.md) — field IDとMasterMemory index numberの区別に関するDraft rule。

documentを分割または移動してもRequirement IDはstableに保つ。directory structureはdocumentationの
organizationに過ぎず、product semantic meaningを持たない。
