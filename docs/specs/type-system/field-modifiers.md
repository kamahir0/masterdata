# Field Modifiers仕様（Field Modifiers）

Status: Proposed

Domain: Type System

## 概要

本proposalは、supported base typeに対するRequired、Nullable、Array valueのfield-levelな表現を
定義する。modifierをtype-name stringへ埋め込まず、fieldのpresenceをfieldに格納されたvalueから
独立して定義する。

## 用語

このdocumentでは、`T` はprimitiveや、固有のspecificationが利用可能になった後のValue Objectなど、
supported field value typeの1つを表す。**base type** はfieldの `type` propertyが指定するtypeである。
**field modifier** はRequired、Nullable、Arrayのいずれかを選ぶfield-levelの選択である。これらの
termは[product terminology（用語）](../../product/terminology.md)を拡張する。

## 規範要件

### TYPE-FIELD-001

すべてのsupported field value typeは、次のmodifier shapeのいずれか1つでなければならない（MUST）。

- `T` — Required
- `T?` — Nullable
- `T[]` — Array

`T?[]` と `T[]?` の組み合わせはサポートしてはならない（MUST NOT）。このproposalではNullableと
Arrayは相互排他的なmodifierである。どちらのmodifierもactiveでない場合、fieldのshapeはRequiredの
`T` となる。

### TYPE-FIELD-002

Field modifierはtype-name stringへ埋め込まず、field-level optionとして表現しなければならない
（MUST）。base type `T` に対するproposed YAML surfaceは次のとおりである。

```yaml
fields:
  - id: 0
    name: rewardItem
    type: ItemId
    nullable: true
  - id: 1
    name: rewards
    type: ItemId
    array: true
```

`type` valueは `?` または `[]` のmodifier suffixなしでbase typeを指定しなければならない（MUST）。
`nullable: true` と `array: true` の両方を設定した場合、schema validation errorとしなければならない（MUST）。

### TYPE-FIELD-003

data record内のfield entryは、Required、Nullable、Arrayのいずれでも存在しなければならない（MUST）。

- Required fieldは、`T` としてvalidなnon-null valueを含まなければならない（MUST）。
- Nullable fieldは `null` を含んでもよい（MAY）。non-nullの場合、そのvalueは `T` としてvalidで
  なければならない（MUST）。fieldを省略した場合はinvalidでなければならない（MUST）。
- Array fieldは、要素が `T` としてvalidなarrayを含まなければならない（MUST）。空array `[]` はvalidで
  なければならず（MUST）、`null` はinvalidでなければならず（MUST）、fieldを省略した場合はinvalidでなければならない（MUST）。

### TYPE-FIELD-004

Nullable fieldとArray fieldは、base typeのkey compatibilityにかかわらず、MasterMemoryのPrimary Key
またはSecondary Keyとして直接利用する場合にkey-incompatibleでなければならない（MUST）。特に、
そのdeclarationがsupportされた場合の `ItemId?` と `ItemId[]` はkey-incompatible shapeである。

このrequirementが定義するのはmodifierがcapabilityへ与える影響だけであり、indexのdeclarationとvalidationは
将来のindex specificationに属する。

### TYPE-FIELD-005

明示的に指定された `nullable: false` または `array: false` optionは受け入れなければならない（MUST）。
各 `false` valueは、対応するoptionが存在しない場合とsemanticに同値でなければならない（MUST）。したがって、
両optionがない場合と、両optionを明示的に `false` とした場合は、同じRequired shapeへresolveしなければならない（MUST）。
このことは、両modifierを `true` でenableした場合にinvalidとするruleを変更しない。equivalenceは独立して適用し、
`nullable: false, array: true` は `T[]`、`nullable: true, array: false` は `T?` へresolveする。

## 検証ルール

このproposalの観測可能なvalidation outcomeは、`TYPE-FIELD-001` から `TYPE-FIELD-005` によって定義する。
対象は、mutually exclusiveなshape、field-level syntax、presence/null/empty-array behavior、key capability、
explicit-false normalizationである。exact diagnostic code、source path、parser-specific YAML node classificationは、
ここでは未割り当てのままとする。

## 受け入れ証拠

| Requirement（要件） | Success observation（成功時の観測） | Failure observation（失敗時の観測） | Suggested evidence（推奨する証拠） |
| --- | --- | --- | --- |
| `TYPE-FIELD-001` | `T`、`T?`、`T[]` が別々のsupported shapeである。 | `T?[]` と `T[]?` がrejectされる。 | Modifier-shape validation tests。 |
| `TYPE-FIELD-002` | field-levelの `nullable` または `array` がshapeを選び、`type` はbase nameのままである。 | suffix syntaxまたは両方のenabled optionがschema validation failureを生む。 | Schema syntax tests。 |
| `TYPE-FIELD-003` | field entryが存在すれば、Required value、nullable `null`、array `[]` が受け入れられる。 | omission、invalid `null`、invalid array shapeがrejectされる。 | Data presence and null/array tests。 |
| `TYPE-FIELD-004` | modifier capabilityがbase primitiveから独立して報告される。 | NullableとArray shapeをdirect keyにできない。 | Key-capability tests after index declarations exist。 |
| `TYPE-FIELD-005` | optionの省略と明示的な `nullable: false` / `array: false` が同じRequired shapeへresolveする。 | 明示的な `false` がrejectされるか、省略時とshapeが変わる。 | Modifier normalization tests。 |

## 互換性

Field modifierはaccepted data shapeを変え、実装後はgenerated C#とserialized representationへ影響する。
既存のfield identity ruleは[field identity仕様（field identity specification）](../compatibility/field-identity.md)が所有し、
field IDとindex numberの分離もそこに含まれる。modifierを追加、削除、変更した場合のreleased-schema
compatibilityとmigration classificationは、引き続きOpen Questionである。

## 例

次はnon-normativeな例である。

```yaml
# Required: the key must be present and the value cannot be null.
rewardItem: 1001

# Nullable: the key must be present, and null is a value.
optionalReward: null

# Array: the key must be present; no values is represented by [].
rewards: []
```

Explicit `nullable: false` と `array: false` は、Required shapeのnon-normativeなsyntax exampleであり、
それぞれoptionを省略した場合と同じ意味を持つ。

次のshapeは、両modifierをenableしているためinvalidである。

```yaml
type: ItemId
nullable: true
array: true
```

## Open Questions（未解決事項）

- `nullable` または `array` に指定されたnon-boolean YAML nodeをparser candidate間でどのように分類し、そのboundaryでcoercionを許可するか。
- arrayとnullに必要なexact YAML node classificationは何か。
- Unity compatibilityを保ちながら、nullable reference typeとnullable value typeをgenerated C#でどう表現するか。
- modifierを追加、削除、変更した場合、どのreleased-schema compatibility classificationを適用するか。
- missing field、invalid null、invalid array elementを特定するexact diagnostic codeとsource pathは何か。
- future type categoryがadditional modifierを導入できるか。導入する場合、初期の3 shapeとどう区別するか。

## 非目標

このproposalは、schema parsing、nullableまたはarray validation、Value Object resolution、index generation、
MessagePack attributes、C# generation、MasterMemory binary generationを実装しない。
