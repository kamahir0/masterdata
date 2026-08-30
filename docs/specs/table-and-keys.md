# Table / Primary Key / Secondary Key仕様

Status: Proposed

Domain: Table / Index

## 概要

本proposalは、project-localなlogical Table、Table schemaとdata documentの関係、persisted fieldのMessagePack key、
Primary Key、およびSecondary Keyのobservable contractを定義する。Tableのidentity boundaryは
[Table identity仕様](compatibility/table-identity.md)が所有し、本proposalはそのboundaryを変更せず、Table schemaの
詳細とkey/index semanticsを追加する。

本proposalは、既存のApproved [Field identity仕様](compatibility/field-identity.md)を直接変更しない。persisted fieldの
独立したField IDをMessagePack専用の`key`へ置き換える提案は、[specification change 0003](../spec-changes/0003-field-identity-to-messagepack-key.md)に
記録する。changeがApprovedかつAppliedになるまで、既存Approved contractはcurrent authorityであり、本proposalは
implementation contractではない。

Primary KeyとSecondary Keyの検索semanticsは、ApprovedのPrimitive Types、Field Modifiers、Value Objectsと、後続のEnum / Flags
仕様が定義するcapabilityを参照する。Referenceの宣言syntaxとtarget resolutionは、本proposalのownerではない。

## 用語

- **Table**: `table` でproject-localに識別されるlogical record collection。
- **Schema document**: `kind: schema` と`table`を持ち、1つのTableのfieldとconstraintを宣言するdocument。
- **Data document**: `kind: data` と`table`を持ち、Tableへrecordを供給するdocument。
- **MessagePack field key**: persisted fieldからgenerated C#のMessagePack `[Key(n)]`へ直接lowerされるnon-negative integer。このproposalでの`key`は、Field IDとは異なり、MessagePack serialization layoutだけを表す。
- **Primary Key**: Tableのrecordを一意に解決する、1つのordered field sequence。
- **Secondary Key**: Tableへ追加する、0個以上のordered field sequence。`nonUnique`により重複許可を選ぶ。
- **selected logical dataset**: Approved [Build Selection仕様](build-selection.md)を適用した後、Tableごとに構成されるrecord集合。

## 規範要件

### SCHEMA-KEY-001

このproposalのpersisted field modelを採用するTableおよびCustom Typeの各persisted fieldは、MessagePack field `key`を宣言しなければ
ならない（MUST）。`key`はnon-negative integerで、同じfield container内でuniqueでなければならない（MUST）。YAML field declarationの
`key: n`は、generated C# memberの`[Key(n)]`へexactly一致する形でlowerされなければならず（MUST）、YAMLのfield declaration orderから
独立していなければならない（MUST）。

`key`はMessagePack serialization layoutだけを表す。`key`からlogical field identity、field rename、field deletion、field addition、
secondary-key identity、reference identity、またはschema migration identityを推測してはならない（MUST NOT）。`key`を変更することは
MessagePack serialization layoutの変更を表すが、他のidentity changeを暗黙に表してはならない。MessagePack `KeyAttribute`または対象backendが
必要とする表現可能範囲を超える追加のupper boundは、このproposalで導入しない。

### SCHEMA-KEY-002

将来GUIが新しいpersisted fieldの`key` defaultを提案する場合、現在activeなfield keyの最大値に1を加えた値をdefault候補にするべきである
（SHOULD）。これはauthoring assistanceであり、MessagePack keyのsemantic allocation ruleではない。authorは提案されたkeyを変更または再採番
してもよく（MAY）、その再採番からlogical field identity、rename、deletion、addition、またはmigration semanticsを導出してはならない
（MUST NOT）。このrequirementはGUIの詳細な編集workflowを定義しない。

### SCHEMA-TABLE-001

1つのproject-local logical `table` identityには、正確に1つのschema documentと、0個以上のdata documentが対応しなければならない（MUST）。
同じlogical `table`を宣言する複数のschema document、またはschema fragmentのmergeはinvalidでなければならない（MUST）。複数のdata documentは
同じ`table`へrecordsを供給してもよい（MAY）。data documentまたはBuild Selection後のselected logical datasetが0件でも、Tableとschemaは存在する
validなempty Tableとして扱わなければならない（MUST）。

### SCHEMA-TABLE-002

Tableのproject-local identityは`table`のvalueでなければならず（MUST）、source fileのpath、filename、directory、またはgenerated C# nameから
導出してはならない（MUST NOT）。`table`は次のASCII lowercase kebab-case grammarに従わなければならない（MUST）。

```text
[a-z][a-z0-9]*(?:-[a-z0-9]+)*
```

`csharpName`はoptionalなgenerated C# type-name presentation overrideであり、Table identityではない。`csharpName`を省略した場合、generated
C# type nameは`table`のkebab segmentsを、各segmentの先頭ASCII characterだけuppercaseして連結する、deterministicな変換でなければならない（MUST）。
したがって`item`は`Item`、`item-category`は`ItemCategory`、`battle-pass-v2`は`BattlePassV2`となる。acronym等を指定したい場合は、例えば
`table: http-server`と`csharpName: HTTPServer`を使用する。

generated C# type nameが同一scopeでcollisionする場合はvalidation errorとしなければならず（MUST）、generatorはsuffix、prefix、escape、または
その他のautomatic repairを行ってはならない（MUST NOT）。generated C# declarationとして表現できない`csharpName`もrejectしなければならず（MUST）。
Tableの`csharpName`は、Value Object / Custom Typeのtype declaration naming contractを変更しない。

### SCHEMA-TABLE-003

このproposalのTable field declarationは、次のcanonical surfaceを使用しなければならない（MUST）。各persisted fieldは
`SCHEMA-KEY-001`に従う`key`、source name、およびbase `type`を持つ。

```yaml
kind: schema
table: item-category
csharpName: ItemCategoryMaster

fields:
  - key: 0
    name: id
    type: ItemId

  - key: 1
    name: category
    type: ItemCategory
```

Table source field nameは、Custom Type fieldと同じlowerCamelCase ASCII grammarに従わなければならない（MUST）。

```text
^[a-z][A-Za-z0-9]*$
```

source field nameからgenerated public property identifierへのmappingは、先頭ASCII characterだけをuppercaseし、残りをそのまま保持しなければ
ならない（MUST）。`itemId`は`ItemId`、`fooBar`は`FooBar`となる。snake_case、kebab-case、spaceの除去、re-case、keyword escape、prefix、suffix、
またはcollision disambiguationを自動で行ってはならない（MUST NOT）。invalid C# identifier、reserved keyword、generated identifier collisionは
validation errorとしてrejectしなければならない（MUST）。このTable-specific ruleは、Approved C# Naming仕様へ新しいTable ruleを追加するものではない。

Table fieldのbase typeは、Primitive Type、Value Object、Custom Type、Enum、またはFlags Enumのいずれかでなければならない（MUST）。
Table、Index、Primary Key、Secondary Key、Reference、その他のrelationshipまたはschema constructをfieldのbase typeとして使用してはならない
（MUST NOT）。fieldのshapeには、base typeに対して`T`、`T?`、または`T[]`を適用しなければならず（MUST）、modifierのsemanticsは
[Field Modifiers仕様](type-system/field-modifiers.md)に従わなければならない（MUST）。unknownまたはunsupportedなtype categoryはvalidation errorとして
rejectしなければならない（MUST）。

Table fieldとして使用可能であることは、Primary KeyまたはSecondary Key componentとして使用可能であることを意味しない（MUST NOT）。
Custom TypeおよびFlags Enumは通常のTable fieldのbase typeとして使用できるが、それぞれのowner specificationが定めるとおり、どちらもkey-incompatible
である。Primitive、Value Object、Custom Type、Enum、Flags Enumのsemanticsとcapabilityは、それぞれのowner specificationに従わなければならない（MUST）。

### SCHEMA-TABLE-004

YAML `fields` sequenceのdeclaration orderは、human-facing presentation semanticsとして保持しなければならない（MUST）。このorderはGUIのcolumn orderと
generated C# property declaration orderに使用しなければならない（MUST）。generatorはMessagePack `key`、Primary Key component、Secondary Key
component、またはその他のnumeric valueでpropertyをsortしてはならない（MUST NOT）。fieldをreorderするとpresentation orderは変わるが、field value、
record identity、Primary Key semantics、Secondary Key semanticsは変化してはならない（MUST NOT）。各memberの`[Key(n)]`は明示された`key`に従い、
declaration orderから独立しなければならない（MUST）。

### SCHEMA-TABLE-005

生成するTable rowは、次のcategoryとpublic property surfaceを持たなければならない（MUST）。rowは`public sealed partial class`であり、各schema fieldを
`public`な`get; init;` propertyとして公開しなければならない（MUST）。rowを`record`として生成してはならず（MUST NOT）、rowへimplicit structural
equality semanticsを追加してはならない（MUST NOT）。`required` modifierは生成してはならない（MUST NOT）。

```csharp
[MemoryTable("item-category"), MessagePackObject]
public sealed partial class ItemCategoryMaster
{
    [Key(0)]
    [PrimaryKey(keyOrder: 0)]
    public ItemId Id { get; init; }

    [Key(1)]
    [SecondaryKey(0, keyOrder: 0), NonUnique]
    public ItemCategory Category { get; init; }
}
```

この例では、YAML `key`はMessagePack `[Key(n)]`へ、`primaryKey.fields`はMasterMemory `[PrimaryKey(keyOrder: n)]`へ、
`secondaryKeys`はMasterMemory `[SecondaryKey(indexNo, keyOrder: n)]`へ、`nonUnique: true`は`[NonUnique]`へlowerされる。`[MemoryTable]`のtable nameは
logical `table` valueを使用し、`[MessagePackObject]`は明示的integer `[Key(n)]`を使用する形とする。実際のnamespace、resolver登録、serialization
constructor、generator file構成はこのdomain specificationのownerではない。

### SCHEMA-TABLE-006

Data documentの`records`はrecord mappingのsequenceでなければならず（MUST）、schemaで宣言されていないrecord field memberはvalidation errorとして
rejectしなければならない（MUST）。Table fieldへはApproved Field Modifiers仕様を適用しなければならない（MUST）。したがってRequired fieldはentryが必須で
null不可、Nullable fieldはentryが必須でnullまたはvalidな`T`、Array fieldはentryが必須でnull不可かつ`[]` validである。Record metadataとして
Approved Build Selection仕様の`$tags`を許可するが、`$tags`をdomain fieldとして生成またはbinaryへserializeしてはならない（MUST NOT）。

複数のdata documentが同じlogical `table`へ提供するrecordsは、logical Tableへmergeしなければならない（MUST）。file path、file discovery order、data
document order、およびsource record orderをdomain record semanticsまたはbinary semanticsとして扱ってはならない（MUST NOT）。

### SCHEMA-TABLE-007

Build Selectionが適用されるTableでは、selection前のprofile-independent validation、selection、selected logical Table construction、Primary Key / Unique
Secondary Key constraints、Reference integrity、canonical ordering、binary buildの順序を、Approved Build Selection仕様の`BUILD-SELECT-010`から
`BUILD-SELECT-017`に従って解釈しなければならない（MUST）。本requirementはPrimary Key、Secondary Key、Referenceのsyntaxを重複定義しない。

同じfuture Primary Keyまたはunique Secondary Key valueを持つsource recordは、Build Selectionで分離される場合にsource内で共存してもよい（MAY）。
duplicate constraintはsource全体ではなくselected logical datasetへ適用しなければならない（MUST）。

### SCHEMA-TABLE-008

binary build前に、selected logical TableのrecordsをPrimary Keyのascending orderでcanonicalに並べなければならない（MUST）。composite Primary Keyでは、
宣言されたcomponent orderに従うlexicographic ascending orderを使用しなければならない（MUST）。YAML record order、data file discovery order、data document
orderを理由にbinary semanticsを変更してはならない（MUST NOT）。同じfinal selected logical datasetから生成されるartifactは同じbinary semanticsを持たなければならない
（MUST）。exact byte-for-byte identity、artifact version、cache key、compressionは別のartifact/build specificationのownerである。

### INDEX-PRIMARY-001

各Tableは、exactly 1つのPrimary Keyを宣言しなければならない（MUST）。Primary Keyを`id`というfield nameから暗黙に推論してはならない（MUST NOT）。
canonical surfaceは次のとおりである。

```yaml
primaryKey:
  fields: [id]
```

`primaryKey.fields`はnon-emptyなfield-name sequenceでなければならず（MUST）、各componentはcurrent Table fieldの`name`へresolveしなければならない
（MUST）。unknown field、duplicate component、または`nonUnique` propertyをPrimary Keyへ指定した場合はvalidation errorとしなければならない（MUST）。
Primary Keyはpersistent Field IDまたはMessagePack `key`を参照せず、resolved current field symbolsのordered sequenceである。

### INDEX-PRIMARY-002

Primary Key componentのsequence orderはkey semanticsでなければならない（MUST）。componentをnumeric key、field declaration order、またはalphabetical orderへ
自動sortしてはならない（MUST NOT）。

```yaml
primaryKey:
  fields: [region, id]
```

この場合、comparison orderはconceptually `(region ASC, id ASC)`であり、composite equalityとlookup semanticsもこのordered sequenceに従う。

### INDEX-PRIMARY-003

Primary Key componentはRequired scalarで、key-compatibleかつcomparison-capableでなければならない（MUST）。このproposalのcurrent supported componentは
`int`、`uint`、`long`、`ulong`、`string`、Value Object、およびnormal Enumである。`bool`、`float`、`double`、Flags Enum、Custom Type、Nullable、Arrayは
Primary Key componentとして使用してはならない（MUST NOT）。Primitive capabilityは[Primitive Types仕様](type-system/primitives.md)、modifier effectは
[Field Modifiers仕様](type-system/field-modifiers.md)、Value Object capabilityは[Value Objects仕様](type-system/value-objects.md)、Enum capabilityは
[Enum / Flags仕様](type-system/enums.md)を参照する。このrequirementはPrimary Key syntaxを他のspecへ追加しない。

### INDEX-SECONDARY-001

Tableは0個以上のSecondary Keyを宣言してもよい（MAY）。canonical propertyは`indexes`ではなく`secondaryKeys`であり、各entryはorderedな
`fields` sequenceを持たなければならない（MUST）。`fields` memberは必須であり、1個以上のcurrent Table field nameを含まなければならない（MUST）。
したがって、`fields: []`はinvalidであり、0-component Secondary Keyにspecial semanticsを与えてはならない（MUST NOT）。各componentはcurrent Table fieldへ
resolveしなければならず（MUST）、unknown fieldまたは同じentry内のduplicate componentはvalidation errorとしてrejectしなければならない（MUST）。
Secondary Key entryにlogical `name`を指定してはならない（MUST NOT）。

Primary KeyとSecondary Keyのcomponent cardinalityは、いずれも1個以上である。

```text
primaryKey.fields       = 1..N
secondaryKeys[*].fields = 1..N
```

```yaml
secondaryKeys:
  - fields: [category]
  - fields: [category, rarity]
    nonUnique: true
```

### INDEX-SECONDARY-002

v1のSecondary Keyはpersistent logical IDを持たず、current Table field namesからresolveしたordered field-symbol sequenceでなければならない（MUST）。同じ
ordered field sequenceを複数回宣言してはならず（MUST NOT）、Primary Keyと完全に同じordered sequenceのSecondary Keyも宣言してはならない（MUST NOT）。
Field IDまたはMessagePack `key`をSecondary Key identityとして使用してはならない（MUST NOT）。

### INDEX-SECONDARY-003

Secondary Key componentは、Primary Key componentと同じく、Required、scalar、key-compatible、comparison-capableでなければならない（MUST）。
NullableおよびArray fieldをSecondary Key componentとして使用してはならず（MUST NOT）。componentのcapability判定は`INDEX-PRIMARY-003`が参照するowner
specificationに従わなければならない（MUST）。

### INDEX-UNIQUE-001

Secondary Keyのuniquenessは`nonUnique` propertyで明示しなければならず（MUST）、field name、field order、または宣言の有無から暗黙に推論してはならない
（MUST NOT）。`nonUnique`のsemanticsは次のとおりである。

- omissionは`false`へresolveし、unique Secondary Keyとする。
- `nonUnique: false`はunique Secondary Keyとする。
- `nonUnique: true`はduplicate key valueを許可するnon-unique Secondary Keyとする。

Primary Keyは常にuniqueであるため、`primaryKey`に`nonUnique`を指定してはならない（MUST NOT）。`nonUnique: true`はgenerated MasterMemory
`[NonUnique]`へlowerされなければならない（MUST）。

### INDEX-SECONDARY-004

`secondaryKeys`のdeclaration orderは、generated MasterMemory `indexNo`のzero-based ordinalへlowerされなければならない（MUST）。例えば、最初の
Secondary Keyは`indexNo 0`、次は`indexNo 1`となる。Secondary Keyのreorderによってgenerated `indexNo`が変化してもよい（MAY）。`indexNo`はbackend / codegen
detailであり、Masterdata semantic identity、MessagePack field identity、またはcompatibility identityとして扱ってはならない（MUST NOT）。
Secondary Key内のcomponent orderは、MasterMemory `[SecondaryKey(indexNo, keyOrder: n)]`の`keyOrder`へlowerされなければならない（MUST）。

### INDEX-SECONDARY-005

Secondary Keyのlogical `name` overrideが存在しないため、同じTableでSecondary Keyをlowerしたgenerated query APIのnameまたはsignatureがcollisionする場合は
schema validation errorとしなければならない（MUST）。generatorはsuffix、prefix、escape、または別の自動disambiguationを行ってはならない（MUST NOT）。
例えばfield `fooAndBar`と、field sequence `[foo, bar]`が同じquery APIを生成する場合はrejectする。`[category]`と`[category, rarity]`のようなprefix
sequenceは、backend上でdistinctなAPIを生成できる限り禁止してはならない（MUST NOT）。

## 検証ルール

このproposalのschema-time validationは、logical Tableごとのschema document数、Table identityとgenerated C# name、field key/name、field declaration order、
Primary Keyの存在・resolution・component order・capability、Secondary Keyのshape・identity・uniqueness・indexNo lowering、およびgenerated query API
collisionを対象とする。data-time validationは、record mapping、schema-unknown member、Field Modifierのpresence、Build Selection後のPrimary Key / unique
Secondary Key制約を対象とする。

Primary Key / Secondary Key componentのcapabilityをこのdocumentで再計算してはならず（MUST NOT）、各owner specificationのapproved capabilityを参照する。
Build Selection後のconstraint適用順は`BUILD-SELECT-010`および`BUILD-SELECT-011`が所有し、Reference validationは`BUILD-SELECT-012`が所有する。

`SCHEMA-KEY-001`の`key`は、YAML source上のMessagePack integer keyとgenerated `[Key(n)]`の対応を検証する。active/deleted fieldのlogical identity、tombstone、
key reuse policyはこのproposalで定義せず、[specification change 0003](../spec-changes/0003-field-identity-to-messagepack-key.md)のapproval/applicationまで
既存Approved Field Identity仕様をcurrent authorityとする。

## 受け入れ証拠

| Requirement | Success observation | Failure observation |
| --- | --- | --- |
| `SCHEMA-KEY-001` | persisted fieldごとにnon-negativeでuniqueな`key`があり、generated memberの`[Key(n)]`が同じ値を使う。field declaration orderを変えてもkey mappingが変わらない。 | `key`欠落、negative、同一container内のduplicate、またはYAML keyと異なる`[Key(n)]`が生成される。`key`からField ID/rename/migration identityが推測される。 |
| `SCHEMA-KEY-002` | GUIの新field default候補がactive keyの最大値+1となる。authorがkeyを変更してもよい。 | GUI候補が必ずserialization identityを固定する、またはkey再採番からlogical field lifecycleを推測する。 |
| `SCHEMA-TABLE-001` | 同じ`table`にschema 1件とdata 0件以上を結合でき、0-record source/selected Tableもvalidなままschema/APIを持つ。 | schemaが2件、schema fragment merge、またはempty Tableが必ずrejectされる。 |
| `SCHEMA-TABLE-002` | validな`item-category`がproject-local identityとなり、未指定時に`ItemCategory`、override時に`HTTPServer`などdeterministicなtype nameになる。 | path/filenameがidentityを変える、invalid `table` grammarが受理される、generated type collisionがsuffix/escapeで修復される。 |
| `SCHEMA-TABLE-003` | Primitive、Value Object、Custom Type、Enum、Flags EnumをTable fieldのbase typeとして使用でき、例えば`Reward`または`Feature`へresolveする。`T`、`T?`、`T[]`はField Modifiers仕様どおりに解決される。 | Table、Index、Primary Key、Secondary Key、Reference等のschema construct、unknown type category、unsupported type referenceがfield base typeとして受理される。Custom Type/Flags Enumをfieldとして許可したことだけを理由にkey componentとして許可する。 |
| `SCHEMA-TABLE-004` | YAML field orderがGUI columnとgenerated property declaration orderへ反映され、`[Key(n)]`は明示keyを保持する。 | propertyがkeyやalphabetical順へsortされ、field reorderがrecord/key semanticsを変更する。 |
| `SCHEMA-TABLE-005` | rowが`public sealed partial class`、`get; init;` property、`[MemoryTable]`、`[MessagePackObject]`、`[Key(n)]`を持つ。 | record、mutable setter、`required`、row structural equality、またはMessagePack/Primary/Secondary attributesの誤ったmappingが生成される。 |
| `SCHEMA-TABLE-006` | 複数data documentのrecordsが同じTableへmergeされ、schema-unknown memberはrejectされる。Required/Nullable/Arrayと`$tags`がowner仕様どおりに扱われる。 | source file orderがdomain semanticsを変える、unknown fieldをignoreする、`$tags`がrow/binary fieldになる。 |
| `SCHEMA-TABLE-007` | selection後のselected logical datasetに対してのみPK/unique constraintとReference validationが適用される。 | source全体でprofile-separated duplicateを先にrejectする、またはselection前にdataset constraintを適用する。 |
| `SCHEMA-TABLE-008` | selected rowsがPrimary Keyのdeclared orderでcanonical sortされ、source order/file splitで同じdatasetのbinary semanticsが変わらない。 | record/file discovery orderがbinary semanticsを変える、またはcomposite component orderを無視する。 |
| `INDEX-PRIMARY-001` | `primaryKey.fields`が存在し、non-emptyでcurrent field symbolsへresolveし、Tableごとにexactly 1つのPrimary Keyとなる。 | missing Primary Key、empty fields、unknown field、duplicate component、`id` magic inference、`nonUnique`が受理される。 |
| `INDEX-PRIMARY-002` | `[region, id]`がその順のcomposite keyとして解決され、comparison/lookupがlexicographic orderを使う。 | componentが自動sortされ、declaration/key numeric orderがPrimary Key orderを上書きする。 |
| `INDEX-PRIMARY-003` | int/uint/long/ulong/string、Value Object、normal EnumのRequired scalarがcomponentになる。 | bool/float/double、Flags、Custom、Nullable、Array、またはnon-key/non-comparison typeがPrimary Keyになる。Custom Type/Flags Enumを通常のTable fieldとして許可したことだけを理由にcomponentへ昇格させる。 |
| `INDEX-SECONDARY-001` | `secondaryKeys`を0件以上宣言でき、各entryが1個以上のcurrent field nameからなるordered `fields`を持ち、`indexes`またはsecondary `name`を要求しない。`[category]`、`[category, rarity]`が受理される。 | `fields`欠落、`fields: []`、unknown field、同一entry内のduplicate component、canonical propertyが`indexes`へ戻る、またはname overrideがidentityとして追加される。 |
| `INDEX-SECONDARY-002` | 同じordered shapeやPrimary Keyと同じshapeがduplicateとしてrejectされ、MessagePack key/Field IDをidentityに使わない。 | key変更やField IDからsecondary identityが導出される、duplicate shapeが共存する。 |
| `INDEX-SECONDARY-003` | Primary Keyと同じRequired scalar capability ruleが適用され、Nullable/Arrayがrejectされる。 | modifierまたはbase capabilityによって禁止されたcomponentが受理される。 |
| `INDEX-UNIQUE-001` | omission/`false`がunique、`true`がnon-uniqueとなり、`true`が`[NonUnique]`へlowerされる。 | uniquenessがfield nameやdeclaration presenceから推論される、Primary Keyへ`nonUnique`が付く、またはunique duplicateが許可される。 |
| `INDEX-SECONDARY-004` | declaration順がindexNo 0, 1, 2へlowerされ、component順がkeyOrderへlowerされる。reorderでindexNoが変わってもlogical identityとは扱われない。 | indexNoがpersistent identity/compatibility identityになる、またはcomponent orderが失われる。 |
| `INDEX-SECONDARY-005` | generated query API name/signature collisionがschema validation errorになる。prefix shapeはbackend上distinctなら受理される。 | collisionがsuffix/prefixで隠される、またはSecondary Key `name`で回避する。 |

`SCHEMA-TABLE-003` のbase-type boundaryは、次のfield fragmentで観測できる。`Reward`はvalidなCustom Type、`Feature`はvalidなFlags Enumとして、
通常のTable fieldに使用できる。

```yaml
fields:
  - key: 0
    name: reward
    type: Reward
  - key: 1
    name: features
    type: Feature
```

次のtype referenceは、Table fieldのbase typeとしてはrejectされなければならない。

```yaml
fields:
  - key: 0
    name: nestedTable
    type: Table
  - key: 1
    name: unknownValue
    type: UnknownTypeCategory
```

このfield-type acceptanceと、`Reward`または`Feature`をPrimary Key / Secondary Key componentとしてrejectするoutcomeは別の観測である。
後者は`INDEX-PRIMARY-003`および`INDEX-SECONDARY-003`のcapability validationが所有する。

`INDEX-SECONDARY-001` のshapeは、次のsuccess/failure evidenceで観測できる。

```yaml
# success
secondaryKeys:
  - fields: [category]
  - fields: [category, rarity]

# failure: zero components
secondaryKeys:
  - fields: []

# failure: duplicate component
secondaryKeys:
  - fields: [category, category]

# failure: unknown field
secondaryKeys:
  - fields: [unknownField]
```

## 互換性

本proposalは、current Approved Field IdentityとApproved Custom Typeのpersisted field semanticsを変更するため、単独ではimplementation authorityではない。
`key` modelへの移行は[specification change 0003](../spec-changes/0003-field-identity-to-messagepack-key.md)のhuman approvalとatomic applicationを必要とする。
適用後は、generated C# member、MessagePack serialization layout、Custom Type constructor order、deleted-field representationへ影響する。

v1は、schema revision Aで生成したC#とbinary Bを組み合わせるcross-schema-version compatibilityを保証しない。coherent artifact setは
`schema A -> generated C# A -> binary A`である。明示的なMessagePack keyはbackward compatibility、forward compatibility、またはmulti-client-version
compatibilityを保証しない。persistent serialization key、deleted-key reservation、type-change compatibility、schema version range、migration policyは、
将来の独立したbinary compatibility specificationのscopeである。

Table identityはproject-localな`table`のvalueに基づき、`csharpName`はpresentation overrideに過ぎない。Secondary Keyの`indexNo`とMessagePack field
`key`は、いずれもlogical field identityまたはreleased-schema compatibility identityではない。profile、path、file split、record order、tag orderが選択後の
同じlogical datasetを変えない限り、Build SelectionとBinary semanticsは[Build Selection仕様](build-selection.md)に従う。

## 例

### Table schema

```yaml
kind: schema
table: item-category
csharpName: ItemCategoryMaster

fields:
  - key: 0
    name: id
    type: ItemId
  - key: 1
    name: category
    type: ItemCategory
  - key: 2
    name: displayName
    type: string

primaryKey:
  fields: [id]

secondaryKeys:
  - fields: [category]
    nonUnique: true
  - fields: [category, displayName]
```

### Data documents

```yaml
kind: data
table: item-category
records:
  - id: 1001
    category: Weapon
    displayName: Sword
```

同じ`table`を宣言する別のdata documentのrecordsもこのlogical Tableへmergeされる。recordのfield orderはdomain semanticsを持たず、Approved Build
Selectionの`$tags`を含めてもgenerated row/binary fieldにはならない。

### Composite Primary Key

```yaml
primaryKey:
  fields: [region, id]
```

このsequenceは`region`を先に比較し、同じregion内で`id`を比較する。field declaration orderやMessagePack `key`の数値順へ自動変換しない。

### Generated C# lowering

MasterMemory v3の現行公式例は、Table rowへ`[MemoryTable]`、propertyへ`[PrimaryKey]`、`[SecondaryKey(indexNo, keyOrder)]`、`[NonUnique]`を付与する。
MessagePack-CSharpの現行公式例は、`[MessagePackObject]`と整数`[Key(n)]`を使用する。本proposalの明示key profileでは、概念的に次のようにlowerする。

```csharp
[MemoryTable("item-category"), MessagePackObject]
public sealed partial class ItemCategoryMaster
{
    [Key(0)]
    [PrimaryKey(keyOrder: 0)]
    public ItemId Id { get; init; }

    [Key(1)]
    [SecondaryKey(0, keyOrder: 0), NonUnique]
    public ItemCategory Category { get; init; }

    [Key(2)]
    public string DisplayName { get; init; }
}
```

これはgenerated public surfaceの例であり、namespace、resolver、formatter、serialization constructor、exact generated file layoutを追加で定義するものではない。

## Open Questions（未解決事項）

- Table fieldの`key`へMessagePack-CSharp `KeyAttribute`のruntime/APIが要求する正確なinteger upper boundがある場合、それをどのvalidation boundaryで表現するか。
- `csharpName`のnamespace、generated filename、filesystem case collision、およびreleased API rename migrationをどのcompatibility policyで扱うか。
- Primary Key / Secondary Keyのvalidation failureへ、どのDiagnostic Code、source span、error presentationを割り当てるか。
- Referenceのexact declaration syntax、Secondary Keyをtargetとして指定する方式、およびmissing-reference severityをどう定義するか。
- generated C#のnamespace、MessagePack resolver registration、serialization constructor、exact formatterのshapeをどう定義するか。
- released schema間でMessagePack binaryを互換にする必要が生じた場合、どの独立したbinary compatibility仕様とmigration policyを採用するか。
- Enum / Flags Enumの詳細仕様が利用可能になった後、normal Enum capabilityの依存関係と実装順序をどう管理するか。

## 非目標

このproposalは、Approved Field IdentityまたはApproved Custom Typeを直接変更・適用しない。Field IDからMessagePack keyへの移行はspecification change 0003に委譲する。
また、Referenceのexact YAML syntax、missing-reference severity、generated helper naming、cross-schema MasterMemory binary compatibility、released-schema migration、
schema version negotiation、exact diagnostic wording、GUIの詳細UX、parser、Rust AST/IR、validator、C# code generator、.NET builder、MasterMemory内部、MessagePack
formatter/resolver、cache、compression、artifact layout、Enum / Flags Enumの詳細仕様、またはIndex以外のMasterReferenceを実装・確定しない。
