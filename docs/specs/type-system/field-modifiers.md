# Field Modifiers仕様（Field Modifiers）

Status: Approved

Domain: Type System

## 概要

本仕様は、supported base typeに対するRequired、Nullable、Array valueのfield-levelな表現と、dataでの
presence、generated C# representation、およびArrayのsequence semanticsを定義する。modifierをtype-name stringへ
埋め込まず、fieldのpresenceをfieldに格納されたvalueから独立して扱う。ここで定義するArray semanticsは、table
fieldとCustom Type fieldを含むすべてのArray fieldに適用する。

## 用語

このdocumentでは、`T` はPrimitive Type、Value Object、Custom Type、または各categoryの仕様が利用を許可したEnum /
Flags Enumなど、supported field value typeの1つを表す。**base type** はfieldの `type` propertyが指定するtypeで
あり、modifierを適用する前のtypeである。**field modifier** はRequired、Nullable、Arrayのいずれかを選ぶfield-level
の選択である。これらのtermは[product terminology（用語）](../../product/terminology.md)を拡張する。

## 規範要件

### TYPE-FIELD-001

すべてのsupported field value typeは、次のmodifier shapeのいずれか1つでなければならない（MUST）。

- `T` — Required
- `T?` — Nullable
- `T[]` — Array

`T?[]` と `T[]?` の組み合わせはサポートしてはならない（MUST NOT）。この仕様ではNullableとArrayは相互排他的な
modifierである。どちらのmodifierもactiveでない場合、fieldのshapeはRequiredの `T` となる。Custom Typeのfieldにも
このrequirementを適用する。

### TYPE-FIELD-002

Field modifierはtype-name stringへ埋め込まず、field-level optionとして表現しなければならない（MUST）。base type
`T` に対するcanonical YAML surfaceは次のとおりである。

```yaml
fields:
  - key: 0
    name: rewardItem
    type: ItemId
    nullable: true
  - key: 1
    name: rewards
    type: ItemId
    array: true
```

`type` valueは `?` または `[]` のmodifier suffixなしでbase typeを指定しなければならない（MUST）。
`nullable: true` と `array: true` の両方を設定した場合、schema validation errorとしなければならない（MUST）。

### TYPE-FIELD-003

data recordまたはCustom Type data mapping内のfield entryは、Required、Nullable、Arrayのいずれでも存在しなければ
ならない（MUST）。

- Required fieldは、`T` としてvalidなnon-null valueを含まなければならない（MUST）。
- Nullable fieldは `null` を含んでもよい（MAY）。non-nullの場合、そのvalueは `T` としてvalidでなければならない
  （MUST）。fieldを省略した場合はinvalidでなければならない（MUST）。
- Array fieldは、要素が `T` としてvalidなarrayを含まなければならない（MUST）。空array `[]` はvalidでなければならず
  （MUST）、`null` はinvalidでなければならず（MUST）、fieldを省略した場合はinvalidでなければならない（MUST）。

### TYPE-FIELD-004

Nullable fieldとArray fieldは、base typeのkey compatibilityにかかわらず、MasterMemoryのPrimary Keyまたは
Secondary Keyとして直接利用する場合にkey-incompatibleでなければならない（MUST）。特に、そのdeclarationがsupport
された場合の `ItemId?` と `ItemId[]` はkey-incompatible shapeである。Value Object、Custom Type、Primitiveなど、base
type categoryはこのmodifier effectを変更しない。

このrequirementが定義するのはmodifierがcapabilityへ与える影響だけであり、indexのdeclarationとvalidationは将来の
index specificationに属する。

### TYPE-FIELD-005

明示的に指定された `nullable: false` または `array: false` optionは受け入れなければならない（MUST）。各 `false`
valueは、対応するoptionが存在しない場合とsemanticに同値でなければならない（MUST）。したがって、両optionがない場合
と、両optionを明示的に `false` とした場合は、同じRequired shapeへresolveしなければならない（MUST）。このことは、
両modifierを `true` でenableした場合にinvalidとするruleを変更しない。equivalenceは独立して適用し、
`nullable: false, array: true` は `T[]`、`nullable: true, array: false` は `T?` へresolveする。

### TYPE-FIELD-006

Nullable shapeのgenerated C# representationは、base typeのcategoryに応じて次のとおりでなければならない（MUST）。

- Primitive value type、Value Object、Custom Type、Enum、またはFlags Enumのnullable fieldは `T?` とする。
- Required `string` fieldは `string` とし、nullable `string` fieldは `string?` とする。
- Required `string` fieldの `null` はinvalidであり、Nullable `string` fieldの `null` はvalidである。

ArrayとNullableは相互排他的であるため、`ImmutableArray<T>?` というfield shapeは存在してはならない（MUST NOT）。

### TYPE-FIELD-007

すべてのArray fieldのschema shape `T[]` は、`T` の順序付きimmutable sequenceを意味しなければならない（MUST）。
generated C#のpublic representationは `ImmutableArray<T>` に統一しなければならず（MUST）、mutableな `T[]` または
`IReadOnlyList<T>` をこのfield shapeのpublic representationとして採用してはならない（MUST NOT）。このruleはtable field、
Custom Type field、および将来のsupported field value typeに等しく適用する。

### TYPE-FIELD-008

Array fieldのdataとgenerated stateは、次のvalidityおよびimmutability semanticsを満たさなければならない（MUST）。

- schema上の `[]` はvalidな空sequenceであり、generated C#では `ImmutableArray<T>.Empty` に対応する。
- `default(ImmutableArray<T>)` または `ImmutableArray<T>.IsDefault == true` のstateはinvalidである。
- generated valueの構築後、public API経由でArrayの要素を変更できてはならない（MUST NOT）。
- sequence orderを保持しなければならず（MUST）、public representationまたは外部からのmutationによってsequenceのcontentsが
  変化してはならない（MUST NOT）。

このdeep immutability requirementは、Array storageとそのpublic access pathに適用する。element type自身のcategory-specific
validityは、そのtypeのowner specificationが定義する。

### TYPE-FIELD-009

Array fieldを含むvalueのequalityおよびhash semanticsでは、Arrayをreference identityで比較してはならない（MUST NOT）。
Array equalityはsequence equalityでなければならず（MUST）、same element count、same ordering、corresponding elements
equalのすべてを満たす場合にのみequalとしなければならない（MUST）。sequence contentsとorderがequalityへ寄与しなければ
ならず、equalなsequenceはequalなhash codeを持たなければならない（MUST）。

## 検証ルール

この仕様の観測可能なvalidation outcomeは、`TYPE-FIELD-001` から `TYPE-FIELD-009` によって定義する。対象は、
mutually exclusiveなshape、field-level syntax、presence/null/empty-array behavior、key capability、explicit-false
normalization、nullable C# mapping、ordered immutable Array representation、default state、deep immutability、sequence
equalityおよびhash consistencyである。exact diagnostic code、source path、parser-specific YAML node classificationは、
ここでは未割り当てのままとする。

## 受け入れ証拠

| Requirement（要件） | Success observation（成功時の観測） | Failure observation（失敗時の観測） | Suggested evidence（推奨する証拠） |
| --- | --- | --- | --- |
| `TYPE-FIELD-001` | `T`、`T?`、`T[]` が別々のsupported shapeとしてPrimitive、Value Object、Custom Type等へ適用できる。 | `T?[]` と `T[]?` がrejectされる。 | Modifier-shape validation tests。 |
| `TYPE-FIELD-002` | field-levelの `nullable` または `array` がshapeを選び、`type` はbase nameのままである。 | suffix syntaxまたは両方のenabled optionがschema validation failureを生む。 | Schema syntax tests。 |
| `TYPE-FIELD-003` | field entryが存在すれば、Required value、nullable `null`、array `[]` が受け入れられる。 | omission、invalid `null`、invalid array shapeがrejectされる。 | Data presence and null/array tests。 |
| `TYPE-FIELD-004` | modifier capabilityがbase typeのkey capabilityから独立して報告される。 | NullableとArray shapeをdirect keyにできない。 | Key-capability tests after index declarations exist。 |
| `TYPE-FIELD-005` | optionの省略と明示的な `nullable: false` / `array: false` が同じRequired shapeへresolveする。 | 明示的な `false` がrejectされるか、省略時とshapeが変わる。 | Modifier normalization tests。 |
| `TYPE-FIELD-006` | value-type nullable fieldが `T?`、nullable stringが `string?`、required stringが `string` として生成される。 | required stringのnullが受理される、またはArrayにNullable representationが生成される。 | Generated C# type-mapping and nullability tests。 |
| `TYPE-FIELD-007` | table fieldとCustom Type fieldを含むArray fieldが順序付きimmutable sequenceとして扱われ、public typeが `ImmutableArray<T>` になる。 | mutable `T[]`、`IReadOnlyList<T>`、または順序を失うrepresentationが公開される。 | Generated API and sequence-order tests。 |
| `TYPE-FIELD-008` | `[]` と `ImmutableArray<T>.Empty` がvalidで、生成後のpublic APIから要素を変更できない。 | `default(ImmutableArray<T>)` がvalidとして扱われる、nullが受理される、または外部mutationでcontentsが変化する。 | Empty/default-state and immutability tests。 |
| `TYPE-FIELD-009` | 同じcount・order・equal elementsを持つArrayがequalで、equal sequenceがequal hashを持つ。 | reference identityだけで判定される、order違いがequalになる、またはequal sequenceのhashが異なる。 | Sequence equality and hash-consistency tests。 |

## 互換性

Field modifierとArray representationはaccepted data shape、generated C# API、将来のserialized representationへ影響する。
特に、`ImmutableArray<T>`、field presence、null、Array order、default stateはpublic behaviorとして扱う。persisted fieldの
MessagePack `key` ruleは[Table / Primary Key / Secondary Key仕様](../table-and-keys.md)の`SCHEMA-KEY-001`が所有し、`key`は
serialization metadataであってlogical field identityではない。modifierまたはArray representationの変更に対するreleased-schema
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

Explicit `nullable: false` と `array: false` は、Required shapeのnon-normativeなsyntax exampleであり、それぞれoptionを
省略した場合と同じ意味を持つ。

次のshapeは、両modifierをenableしているためinvalidである。

```yaml
type: ItemId
nullable: true
array: true
```

Custom Type fieldでは、同じmodifier semanticsを使用する。

```yaml
custom:
  fields:
    - key: 0
      name: note
      type: string
      nullable: true
    - key: 1
      name: tags
      type: string
      array: true
```

## Open Questions（未解決事項）

- `nullable` または `array` に指定されたnon-boolean YAML nodeをparser candidate間でどのように分類し、そのboundaryでcoercionを許可するか。
- Array要素、`null`、および `ImmutableArray<T>.IsDefault` に必要なexact YAML node classificationは何か。
- modifierまたはArray representationを追加、削除、変更した場合、どのreleased-schema compatibility classificationを適用するか。
- missing field、invalid null、invalid array elementを特定するexact diagnostic codeとsource pathは何か。
- future type categoryがadditional modifierを導入できるか。導入する場合、初期の3 shapeとどう区別するか。

## 非目標

この仕様は、schema parsing、nullableまたはArray validation、Value Object resolution、Custom Type resolution、index
generation、MessagePack attributes、C# generation、MasterMemory binary generationを実装しない。
