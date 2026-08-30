# Field identity仕様（Field identity）

Status: Deprecated

この文書は、specification change 0003によってretiredとなった旧Field ID modelの履歴と、現行canonical ownerへのroutingを記録する。
旧Requirement IDはtraceabilityのために保持するが、以下の旧contractは現在のpersisted fieldに適用されるnormative requirementではない。
現行のpersisted field surfaceとMessagePack key semanticsは、[Table / Primary Key / Secondary Key仕様](../table-and-keys.md)の
`SCHEMA-KEY-001`が所有する。

## Retired requirements

### COMPAT-FIELD-001（Retired）

旧Field Identity modelでは、TableまたはCustom Typeのpersisted fieldにnumeric Field IDを付与し、container内でactive IDとreserved IDを
共有するused-ID namespaceを管理していた。Field IDはfield name、table、Custom Type、およびMasterMemoryのindex numberとは別の
namespaceとして扱われていた。このmodelはspecification change 0003のAppliedによりretiredであり、現在のMessagePack `key`や
logical field identityのvalidationをこのRequirementから導出してはならない（MUST NOT）。

### COMPAT-FIELD-002（Retired）

旧Field Identity modelでは、いったんusedとなったField IDを、field削除後に同じcontainerで再利用しない扱いとしていた。この
deleted-ID reservationとreuse prohibitionはspecification change 0003のAppliedによりretiredであり、現行のMessagePack `key`へ
適用してはならない（MUST NOT）。

### COMPAT-FIELD-003（Retired）

旧Field Identity modelでは、削除したTable fieldをtop-level `reservedFields`、削除したCustom Type fieldを
`custom.reservedFields`へ記録し、Custom Type entryに`id`、`formerName`、`formerType`を保持していた。このreserved-field/tombstone
surfaceはspecification change 0003のAppliedによりretiredであり、現行canonical declarationへ導入してはならない（MUST NOT）。

### COMPAT-FIELD-004（Retired）

旧Field Identity modelでは、field nameのrename時に同じField IDを維持していた。このrename tracking semanticsはspecification change
0003のAppliedによりretiredであり、現行のMessagePack `key`からrename、deletion、addition、またはmigration identityを推論しては
ならない（MUST NOT）。

## 現行canonical owner

現行のTableおよびCustom Type persisted fieldは、`key`、`name`、`type`を含むfield surfaceを使用する。`key`は同じfield container内で
required、non-negative、uniqueなMessagePack serialization metadataであり、generated C#の`[Key(n)]`へ対応する。`key`はlogical field
identity、rename/deletion identity、Secondary Key identity、Reference identity、またはreleased-schema migration identityではない。
このobservable contractは`SCHEMA-KEY-001`がcanonicalに所有し、本仕様は同じsemanticを重複定義しない。

`key`のexact backend attribute shape、upper bound、formatter/resolver、released binary compatibility、およびmigration policyは、
それぞれの将来仕様が所有する。本仕様のretired historyを理由に、現行keyへtombstoneまたはreuse prohibitionを追加してはならない。

## 実装状態と履歴

current scaffoldまたは実装が旧`id`・`reservedFields` shapeを保持している場合、それはApplied delta後のimplementation gapを示す。
実装状態はこの文書のretired Field ID historyまたは`SCHEMA-KEY-001`の代替authorityではない。specification change 0003は、旧Requirement
IDを削除・再利用せずretired historyとして保持し、Custom Type constructor orderを`SCHEMA-CUSTOM-017`のYAML declaration orderへ置換した。

## Open Questions（未解決事項）

- nullability、default value、type change、Custom Type evolutionを含むreleased-schema compatibilityの分類をどの将来仕様で所有するか。
- MessagePack backendのexact attribute、formatter、resolver、およびserialization constructorをどの仕様で定義するか。
- 外部のsave data、network protocol、database、public APIがgenerated numeric keyを長期保存する場合の外部compatibilityをどう扱うか。

これらのOpen Questionは、旧Field ID modelを現行contractへ戻すものではない。
