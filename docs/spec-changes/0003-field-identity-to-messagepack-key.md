# 仕様変更: Field IDをMessagePack keyへ置き換える（Specification change）

Status: Applied

<!-- Lifecycle: Draft -> Proposed -> Approved -> Applied、またはRejected。Approved canonical
     specificationを変更する前にhuman approvalが必要である。 -->

## Affected Specifications

- [`docs/specs/compatibility/field-identity.md`](../specs/compatibility/field-identity.md)、Approvedの`COMPAT-FIELD-001`から
  `COMPAT-FIELD-004`。
- [`docs/specs/type-system/custom-types.md`](../specs/type-system/custom-types.md)、Approvedの`SCHEMA-CUSTOM-003`、
  `SCHEMA-CUSTOM-010`、`SCHEMA-CUSTOM-012`、`SCHEMA-CUSTOM-016`。
- [`docs/specs/table-and-keys.md`](../specs/table-and-keys.md)、Approvedの`SCHEMA-KEY-001`および
  `SCHEMA-CUSTOM-017`が参照する新しいpersisted field model。

## 根拠と分類（Source Evidence and Classification）

Human Decisionは、persisted Table fieldおよびCustom Type fieldからMessagePackとは独立したpersistent numeric Field IDを
廃止し、`key`をMessagePack serialization layoutだけのexplicit integer keyとすることを決定した（Decision / Constraint）。
Human Decisionは、`key`をlogical field identity、rename、deletion、addition、secondary-key identity、reference identity、または
schema migration identityとして扱わないことも明示した（Constraint）。

同じDecisionは、Table schemaにおける1 schema documentと0..N data documents、Primary Key / Secondary Keyのsource field-name
参照、generated C# property order、およびMessagePack keyとTable constraintの分離を、別のApproved
[Table / Primary Key / Secondary Key仕様](../specs/table-and-keys.md)へ記録する根拠となる。

これは、既存Approved Field IdentityおよびCustom Typeの意味をcurrent implementationへ合わせて変更する推測ではない。
既存Approved contractに対する、human approval済みのsemantic deltaである。canonical applicationは完了しており、以後は更新済み
canonical specificationがauthorityである。

## 提案する差分（Proposed Delta）

### 1. Approved Field Identityの置き換え

`COMPAT-FIELD-001`から`COMPAT-FIELD-004`が定義する、TableまたはCustom Typeのpersisted fieldに対する次のmodelをretireする。

- active fieldのpersistent `id`
- top-levelまたは`custom`配下の`reservedFields`
- deleted Field IDのtombstone
- used Field IDの再利用禁止
- renameをField IDで追跡するidentity

これらのidentifierはhistoryのために保持できるが、retire後のactive normative requirementとして再利用してはならない（MUST NOT）。
`docs/specs/compatibility/field-identity.md`は、atomic application時に旧requirementのretired historyと、置換先の
`SCHEMA-KEY-001`およびTable/Key specificationへのroutingを記録する。旧Requirementをretired historyとして記録し、このdocumentを
repository lifecycleに従って`Status: Deprecated`へ遷移する。

### 2. MessagePack専用の`key`

persisted Table fieldおよびCustom Type fieldの新しいcanonical surfaceは、次のように`key`を持つ。

```yaml
fields:
  - key: 0
    name: itemId
    type: ItemId
```

`key`は、同じfield container内でuniqueなnon-negative integerであり、generated C#のMessagePack `[Key(n)]`へexactly lowerされる。
field declaration orderから独立し、logical field identityを持たない。このruleのcanonical ownerは
`docs/specs/table-and-keys.md`の`SCHEMA-KEY-001`である。

`key`を変更することはMessagePack serialization layoutの変更を表すだけであり、rename、deletion、addition、secondary-key identity、
reference identity、schema migration identityを暗黙に変更しない。active/deleted fieldのpersistent identity、tombstone、reserved key、
key reuse policyは、このdeltaから導入しない。

### 3. Approved Custom Typeのcanonical surface

`SCHEMA-CUSTOM-003`が定義するCustom Type field entryを、`id`ではなく`key`へ変更し、`custom.reservedFields`をcanonical surfaceから
削除する。

```yaml
kind: type
name: Reward
custom:
  fields:
    - key: 0
      name: itemId
      type: ItemId
```

`SCHEMA-CUSTOM-010`が定義するstable Field ID、Field ID namespace、active/reserved lifecycleはretireする。keyのvalidationと
MessagePack mappingは`SCHEMA-KEY-001`を参照し、Custom Type仕様で同じruleを重複定義しない。

`SCHEMA-CUSTOM-012`が定義するfield ID ascendingのconstructor parameter orderはretireする。代わりに、新しい
`SCHEMA-CUSTOM-017`をCustom Type仕様へ追加し、generated public constructorのparameter orderをYAML `custom.fields` declaration orderとする。
parameter identifier、get-only property、structural equality、hash、constructorのshallow validationなど、field ID orderに依存しない
既存のCustom Type contractは変更しない。

`SCHEMA-CUSTOM-016`が定義する`custom.reservedFields`、deleted Field ID、reserved ID reuse prohibition、およびrename時のField ID保持は
retireする。Custom Typeのfield name renameは、Field IDで追跡する新しいpersistent identityを作らない。

### 4. Table / Key仕様との適用関係

このdeltaは、Table/Primary/Secondary Key仕様の承認記録を代替するものではない。Table/Key仕様は別途human approval済みであり、
`SCHEMA-KEY-001`および新しいTable/Key semanticsは、このdeltaのatomic application完了後のcanonical implementation authorityである。

### 5. Atomic application

human maintainerの明示的な承認後、repository lifecycleが定めるcanonical mergeとして次をatomicに実施した。

1. `docs/specs/compatibility/field-identity.md`の`COMPAT-FIELD-001`から`COMPAT-FIELD-004`をretired historyとして保持し、
   current Field ID contractのownerではないことを明示する。
2. `docs/specs/type-system/custom-types.md`のCustom Type declaration、field identity、constructor order、reserved field requirementを、
   上記の新surfaceへ更新する。既存のstructural/equality/modifier semanticsは変更しない。
3. `docs/specs/schema-language.md`、compatibility index、directory indexを新しいcanonical ownerへroutingする。
4. canonical merge完了後にartifactのstatusを`Applied`へ移す。

上記のcanonical applicationとartifactの`Applied`遷移は、一つの変更として完了している。

## 互換性（Compatibility）

これはApproved specificationへの意図的なbreaking semantic changeである。current `id`、`reservedFields`、Field ID rename/tombstone
semanticsを前提とするschema、Custom Type constructor、またはcompatibility toolingは、delta application後に更新対象となる。
一方、`key`はMessagePack layoutだけを表し、cross-schema binary compatibilityを保証しない。schema revision Aとgenerated C# / binary Bを
組み合わせる保証、deleted serialization keyのreservation、type-change migration、schema version negotiationはこのdeltaで追加しない。

`SCHEMA-KEY-001`が定めるkeyは、Field IDのreplacement identityではない。numeric keyのreorderまたはreuseから、field rename、delete、add、
secondary-key identity、reference identity、released-schema compatibilityを推論してはならない。external save data、network protocol、
external database、public APIなどがgenerated numeric keyを長期保存する場合のcompatibilityは、別の外部契約で扱う。

## 受け入れと実装への影響（Acceptance and Implementation Impact）

このdeltaをAppliedにしたcanonical mergeでは、少なくとも次をobservableに確認できなければならない。

- TableおよびCustom Typeの新しいpersisted field surfaceに`id`または`reservedFields`を要求せず、`key`をMessagePack `[Key(n)]`へ対応付ける。
- 同じcontainerのduplicateまたは欠落した`key`をrejectし、field declaration orderとは独立してkeyを保持する。
- `key`をField ID、rename/deletion identity、secondary-key identity、reference identity、schema migration identityとして使用しない。
- Custom Type constructor parameter orderがfield ID ascendingではなく、YAML `custom.fields` declaration orderとなる。
- Custom Typeの1-field/multi-field、mapping、structural equality、Array sequence equality、modifier、およびshallow constructor validationは変わらない。
- 旧`COMPAT-FIELD-*`と`SCHEMA-CUSTOM-*`のidentifierがhistory上追跡でき、同じidentifierを別の意味で再利用しない。

影響する実装boundaryは、schema parser/model、field/type resolver、validator、C# generator、MessagePack/MasterMemory builder、および
関連するschema/compatibility testである。ただし、このartifactの作成ではproduction implementation、fixture、test、またはbuilderを変更しない。

## 未解決事項（Open Questions）

- MessagePack/C# backendのexact formatter、resolver、serialization constructor、generated file layoutをどの別specificationで所有するか。
- released-schema間のbinary compatibility、persistent serialization key reservation、type/modifier change migrationを将来どの独立specificationで定義するか。
- Table/Secondary KeyをtargetにするReferenceのexact syntaxと、field rename/deletionを外部long-lived contractでどう扱うか。

これらは、今回の`key`がMessagePack専用であること、またField IDをcurrent v1 identityにしないことを変更しない。

## レビュー（Review）

このartifactは、Approved Field IdentityおよびApproved Custom Typeへ適用されたsemantic deltaのaudit recordである。
`docs/specs/table-and-keys.md`のTable/Key specificationとともに`review-spec`を実行し、human maintainerの明示的な承認を得たうえで
atomic applicationを完了した。

## 承認記録（Approval Record）

このtaskでhuman maintainerが、Enum / FlagsおよびTable / KeysのApproved化とあわせて、本artifactのdeltaを明示的に承認した。
deltaはField Identity、Custom Type、schema-language、compatibility routingへatomicに適用済みであり、本artifactを`Status: Applied`とする。
