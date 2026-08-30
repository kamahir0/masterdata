# EnumとFlags Enum

Status: Proposed

Domain: Type System

## 概要

本仕様は、通常のEnumとFlags Enumのschema category、underlying integer type、member value、data
representation、capability、およびgenerated C# shapeを定義する。EnumとFlags Enumは、同じunified
type-declaration boundaryを使用する別々のcategoryである。

Enum/Flagsのnumeric valueは、現行schema内のmember valueである。Masterdata binaryをまたぐ永続的な
wire identityではない。この境界を越える外部のsave data、network protocol、external database、public
APIのcompatibilityは、本仕様の外部契約である。

## 用語

- **Normal Enum**: 宣言されたmemberのうち、ちょうど1つのsymbolic valueを表すEnum。
- **Flags Enum**: atomic bit memberのbitwise combinationを表すEnum。`None` はzero-value sentinelである。
- **Underlying**: Enum/Flagsのnumeric representationを決める、明示されたinteger Primitive Type。
- **Atomic bit**: declared underlyingのfixed-width bit patternで、set bitがちょうど1つのvalue。
- **Current schema**: 1つのcurrent type declarationをvalidationした結果。numeric valueのuniquenessはこの
  scopeで適用する。

## 規範要件

### SCHEMA-ENUM-001

EnumおよびFlags Enumのnumeric valueは、current schema内のmember valueとして扱わなければならず（MUST）、
Masterdata binaryをまたぐpersistentなwire identityとして扱ってはならない（MUST NOT）。したがって、この
specificationは、削除したmemberのnumeric valueまたはbitを後続schema revisionでtombstone、
`reservedMembers`、または永久予約として保持することを要求してはならない（MUST NOT）。削除済みvalueの再利用を、
Masterdata schema revisionだけを理由に一律禁止してはならない（MUST NOT）。

このrequirementは、以前のDraftにあった「numeric valueをpersistent wire identityとして予約する」意味を置き換える。
Masterdata外のlong-lived external contractがnumeric valueをidentityとして使用する場合は、そのexternal contractが
別途compatibility ruleを定義する。

### SCHEMA-ENUM-002

EnumおよびFlags Enumは、Value ObjectおよびCustom Typeと同じunified type-declaration boundaryを使用しなければ
ならない（MUST）。1つのYAML documentには、正確に1つのtype declarationだけを含めなければならず（MUST）、
`kind: type`、top-level `name`、およびcategory mappingである `enum` または `flags` を使用しなければならない
（MUST）。同じdeclarationで `enum` と `flags` を同時に使用してはならない（MUST NOT）。

`enum` または `flags` mappingは、`underlying` と `members` を明示的に含めなければならず（MUST）、`members` はmember
mappingのYAML sequenceでなければならない（MUST）。
`underlying` は省略してはならず（MUST NOT）、次の4つのinteger Primitive Typeだけを使用しなければならない（MUST）。

- `int`
- `uint`
- `long`
- `ulong`

`byte`、`sbyte`、`short`、`ushort`、またはinteger以外のPrimitive Typeをunderlyingとして受理してはならない
（MUST NOT）。underlyingのwidth、signedness、range、scalar classificationは[Primitive Types仕様](primitives.md)
に従わなければならない（MUST）。

Canonical declarationは次のとおりである。

```yaml
kind: type
name: ItemRarity
enum:
  underlying: int
  members:
    - name: Common
      value: 1
    - name: Rare
      value: 2
```

```yaml
kind: type
name: Feature
flags:
  underlying: ulong
  members:
    - name: None
      value: 0
    - name: Fire
      value: 1
```

### SCHEMA-ENUM-003

EnumおよびFlags Enumのすべてのmemberは、`name` とnumeric `value`を明示的に宣言しなければならない（MUST）。
implicit numberingまたはauto-incrementを使用してはならない（MUST NOT）。numeric `value`は、承認済みのMasterdata
YAML subsetが定めるinteger scalarとして解釈され、宣言されたunderlyingのrepresentable rangeに収まらなければならない
（MUST）。

`int` または `long` ではnegative valueを使用してもよい（MAY）。`uint` または `ulong` ではnegative valueを受理しては
ならない（MUST NOT）。member nameとnumeric valueは、EnumまたはFlags Enumのcurrent schema内でそれぞれuniqueでなければ
ならない（MUST）。

### SCHEMA-ENUM-004

Normal Enumは、少なくとも1つのmemberを持たなければならない（MUST）。normal Enumはrepresentableなinteger valueを任意に
使用してもよく（MAY）、numeric valueのgap、signed underlyingでのnegative valueを許可しなければならない（MUST）。
numeric valueが `0` のmemberを要求してはならず（MUST NOT）、`0` に特別なdomain semanticsを割り当ててはならない
（MUST NOT）。

Generated C#におけるnormal Enumのlanguage-level `default` valueは、declared memberに対応しないvalueでもよい（MAY）。
ただしMasterdata source dataは、declaredされていないnormal Enum valueを構成してはならない（MUST NOT）。

Normal Enumはkey-compatibleでなければならず（MUST）、comparison-capableでなければならない（MUST）。comparisonは
memberのunderlying numeric valueと、declared underlying Primitive Typeのnatural ascending orderに従わなければならない
（MUST）。member declaration orderはcomparisonへ影響してはならない（MUST NOT）。

### SCHEMA-ENUM-005

Normal EnumのMasterdata data representationはsymbolic member nameでなければならない（MUST）。valueは、宣言された
memberのうちちょうど1つへcase-sensitiveにresolveしなければならない（MUST）。raw numeric valueをEnum memberへ暗黙に
変換してはならない（MUST NOT）。したがって、`rarity: 2` はinvalidであり、`rarity: Rare` は `Rare` が宣言されている
場合にvalidである。

Approved YAML subsetに従う限り、plain stringとquoted stringのどちらも、decoded symbolic member nameが同じなら同じvalue
を表してよい（MAY）。quote styleはEnumのdomainまたはbinary semanticsを変更してはならない（MUST NOT）。

### SCHEMA-ENUM-006

EnumおよびFlags Enumのtype nameとmember nameは、次のexact ASCII lexical ruleに一致しなければならない（MUST）。

```text
^[A-Z][A-Za-z0-9]*$
```

このspecificationは、Enum/Flagsのtype nameとmember nameへのこのruleのcanonical ownerである。`A`、`ID`、`URL`、
`REWARD`、`ItemRarity`、`HTTP`、`XML2` はvalidである。非ASCII character、separator、lowercaseまたはdigitで始まる
nameはinvalidである。Unicode normalization、transliteration、re-casing、prefix、suffix、`@` escape、その他の
automatic repairを行ってはならない（MUST NOT）。

Generated C# type identifierはsource `name`をそのまま使用しなければならない（MUST）。member nameがvalidなgenerated
C# identifierとしてemissionできない、C# reserved keywordと衝突する、同じtype内でduplicateになる、またはenclosing
generated type等のgenerated declarationとcollisionする場合は、schemaまたはcode-generation validation errorとしなければ
ならない（MUST）。collisionを自動suffix、prefix、escape、またはnormalizationで解決してはならない（MUST NOT）。

Member declaration orderはgenerated C# sourceで保持しなければならない（MUST）。numeric valueによって自動reorderしては
ならない（MUST NOT）。ただしmember declaration orderは、memberのdomain meaning、equality、comparison、selection、または
binary semanticsを変更してはならない（MUST NOT）。

### SCHEMA-ENUM-007

Normal EnumおよびFlags Enumのgenerated C# typeは、対応するunderlying C# integer typeを直接指定するpublic `enum`で
なければならない（MUST）。declared numeric valueを明示的にemissionしなければならず（MUST）、implicit member allocationや
generator-owned memberの追加を行ってはならない（MUST NOT）。

Normal Enumには`System.FlagsAttribute`を付与してはならず（MUST NOT）。Flags Enumには`System.FlagsAttribute`を付与しなければ
ならない（MUST）。attributeのsource spellingは異なってもよい（MAY）が、public C# API semanticsは同一でなければならない
（MUST）。helper method、extension method、custom wrapper、parsing API、runtime validatorは、このspecificationのgenerated
minimum APIに含めない。

### SCHEMA-FLAGS-001

Flags Enumは常にkey-incompatibleでなければならず（MUST）、MasterMemoryのPrimary Keyまたはsecondary/index key componentへ
直接使用可能なtypeとして扱ってはならない（MUST NOT）。contained underlying、member count、member values、field modifier、または
declaration optionによってkey-compatibleへ昇格させてはならない（MUST NOT）。Primary KeyまたはIndexのsyntaxはこのspecification
で定義しない。

### SCHEMA-FLAGS-002

Flags Enumは、numeric valueが正確に `0` でnameが正確に `None` であるmemberを、ちょうど1つ定義しなければならない（MUST）。
`None` を省略、rename、nonzero valueへの変更してはならず（MUST NOT）、別のzero-valued memberを定義してはならない（MUST NOT）。
したがって、generated Flags EnumのC# `default` valueは `None` に対応する。

`None = 0` 以外のすべてのFlags memberは、declared underlyingのfixed-width bit patternにおいてset bitが正確に1つでなければ
ならない（MUST）。この判定は単なる「positive power of two」ではなく、declared widthのbit patternで行わなければならない
（MUST）。したがって、signed `int` のhighest bitを表す `-2147483648`、unsigned `uint` のhighest bitを表す `2147483648`、
および対応する64-bit underlyingのhighest bitは、atomic bitとしてvalidになり得る。複数bitを持つnamed composite memberは
v1では受理してはならない（MUST NOT）。

### SCHEMA-FLAGS-003

Flags EnumのMasterdata data representationは、宣言されたsymbolic member nameのYAML sequenceでなければならない（MUST）。
block sequenceとflow sequenceは使用してよい（MAY）。raw numeric mask、または`"Fire | Ice"`のようなcustom string expressionを
受理してはならない（MUST NOT）。選択されたnonzero atomic memberのruntime/numeric valueは、bitwise ORで構成しなければならず
（MUST）。未宣言memberまたはundefined bitをsource/build processingで構成してはならない（MUST NOT）。

zero Flags valueには、`features: [None]` というちょうど1つのcanonical data representationを使用しなければならない（MUST）。
`features: []` および `features: [None, Fire]` はinvalidでなければならない（MUST）。`None` はnonzero memberと組み合わせては
ならない（MUST NOT）。

Flags data sequenceはunordered set semanticsを持たなければならない（MUST）。したがって、`[Fire, Ice]` と `[Ice, Fire]` は
同じFlags valueを表す。memberのsequence orderをsemanticへ使用してはならず（MUST NOT）、同一memberのduplicateはsilent
deduplicationせずrejectしなければならない（MUST）。

### SCHEMA-FLAGS-004

Flags Enumはv1ではcomparison-incompatibleでなければならず（MUST）。Flagsのbit combinationへdomain orderingを与えては
ならず（MUST NOT）。descending、custom comparer、culture、またはper-type comparison optionを追加してはならない（MUST NOT）。

### SCHEMA-ENUM-008

EnumおよびFlags Enumは、他のsupported field value typeと同じく、[Field Modifiers仕様](field-modifiers.md)の `T`、`T?`、
`T[]` におけるbase typeとして使用してもよい（MAY）。使用する場合のNullableとArrayのmutual exclusion、field presence、null、
empty Array、Array representationは[Field Modifiers仕様](field-modifiers.md)が定義する。Flags Enumのscalar representationが
sequenceであるため、Flags EnumのArray fieldはその仕様に従うnested sequence structureとなり、別のshorthandを導入してはならない
（MUST NOT）。

## 検証ルール

### Schema-time validation

次はtype declarationのvalidationでrejectしなければならない（MUST）。

- `underlying` の欠落、または許可されないunderlying type。
- `enum` と `flags` の同時指定、またはunified declaration envelopeの不備。
- memberの欠落、member `name` またはnumeric `value` の欠落、implicit numbering。
- declared underlyingのrange外、またはunsigned underlyingでのnegative value。
- duplicate member name、normal Enumのduplicate numeric value。
- normal Enumがzero memberを持たないことだけを理由にrejectすること。
- Flagsの`None = 0`欠落、複数zero member、または`None`のrename/nonzero化。
- Flagsのzero以外のmemberがexactly one set bitでないこと、named composite member。
- invalid type/member identifier、reserved keyword、またはgenerated declaration collision。

### Data-time validation

次はMasterdata dataのvalidationでrejectしなければならない（MUST）。

- unknownまたはcase-mismatchedなnormal Enum symbolic member。
- raw numeric normal Enum value。
- unknownまたはduplicateなFlags member。
- raw numeric Flags mask、custom string expression、undefined bit。
- empty Flags sequence、または`None` とnonzero memberの混在。

Enum/FlagsのYAML scalar・sequence syntaxは[Masterdata YAML subset仕様](../yaml-subset.md)が所有し、integer width/rangeと
numeric scalar classificationは[Primitive Types仕様](primitives.md)が所有する。本仕様はそれらのownerを上書きしない。

## 受け入れ証拠

| Requirement（要件） | Success observation（成功時の観測） | Failure observation（失敗時の観測） | Suggested evidence（推奨する証拠） |
| --- | --- | --- | --- |
| `SCHEMA-ENUM-001` | current schemaのmember削除後、後続schemaでnumeric valueを再利用しても、旧valueのtombstone/reservationだけを理由にrejectされない。external contractのための別compatibility ruleはこのspecに混入しない。 | 削除valueを永久予約する、`reservedMembers` を要求する、またはMasterdata binary identityとして扱う。 | Cross-revision identity tests and compatibility-boundary review。 |
| `SCHEMA-ENUM-002` | `kind: type`、top-level `name`、`enum`または`flags`、明示されたunderlying/membersを持つ1 document/1 declarationがresolveする。`int`、`uint`、`long`、`ulong`が受理される。 | underlying欠落、`float`/`string`/`byte`、複数type declaration、または`enum`と`flags`の同時指定が受理される。 | Declaration envelope and underlying validation tests。 |
| `SCHEMA-ENUM-003` | すべてのmemberが明示的valueを持ち、signed typeのrepresentable negative valueとunsigned typeの非negative valueがrange内で受理される。 | member valueの省略、auto-increment、range外、unsigned negative value、またはduplicate member/numeric valueが受理される。 | Member-value and integer-range validation tests。 |
| `SCHEMA-ENUM-004` | `Common = 1`、`Rare = 10`、`Legendary = 100` のnormal Enumが受理され、宣言順によらずnumeric orderで比較される。zero memberなし、gap、signed negative valueが許可され、normal Enumがkey-compatibleとなる。 | zero memberが必須化される、member orderがcomparisonを決める、normal Enumがkey-incompatibleになる、またはgap/negative signed valueがrejectされる。 | Normal Enum capability and ordering tests。 |
| `SCHEMA-ENUM-005` | `rarity: Rare` がdeclared memberへresolveし、`"Rare"` も同じdecoded nameなら同じvalueを表す。 | `rarity: 2`、unknown name、またはcase違いが受理される。 | Symbolic Enum data-validation tests。 |
| `SCHEMA-ENUM-006` | `ID`、`URL`、`REWARD`、`HTTP`、`ItemRarity`、`XML2` と同じsource nameがそのままgenerated type/member identifierとなり、member declaration orderがC# sourceで保持される。 | style上のPascalCase判定でALL-CAPSをreject、nameをre-case/escape、reserved/collisionを自動修復、またはnumeric orderへreorderする。 | Enum/Flags lexical, collision, and declaration-order tests。 |
| `SCHEMA-ENUM-007` | normal Enumが`public enum ... : int`、Flags Enumが`[System.Flags] public enum ... : uint`となり、explicit valuesが出力される。 | custom wrapper/helper、implicit member allocation、normal EnumへのFlags attribute、またはFlags Enumへのattribute欠落。 | Generated C# API/compile inspection。 |
| `SCHEMA-FLAGS-001` | underlyingやmember valuesにかかわらずFlags Enumがkey-incompatibleとして分類される。 | Flags EnumがPrimary Keyまたはsecondary/index key componentとして許可される。 | Capability-boundary validation tests。 |
| `SCHEMA-FLAGS-002` | `None = 0` がちょうど1つあり、`1`、`2`、signed highest bitなどexactly-one-set-bitのmemberがunderlying widthに従って受理される。 | `None`欠落、別zero member、`None != 0`、`3`のようなnamed composite、またはsigned highest bitの誤reject。 | Flags bit-pattern validation tests。 |
| `SCHEMA-FLAGS-003` | `[None]` がzero value、`[Fire, Ice]` と`[Ice, Fire]` が同じOR valueとして受理される。 | `[]`、`[None, Fire]`、`[Fire, Fire]`、raw `3`、`"Fire | Ice"`、unknown member、またはsequence order依存が受理される。 | Flags data representation and set-semantics tests。 |
| `SCHEMA-FLAGS-004` | Flags Enumにdomain comparison capabilityが付与されない。 | numeric/bit combinationからorderingを導出、またはcustom comparer/descending optionを追加する。 | Capability and scope review。 |
| `SCHEMA-ENUM-008` | Enum/Flags fieldが`T`、`T?`、`T[]`としてField Modifiers仕様どおりに扱われ、Flags arrayがnested sequenceとして表現される。 | Nullable/Array semanticsを再定義、またはnested sequenceを避ける未承認shorthandを導入する。 | Cross-spec field-modifier tests。 |

## 互換性

Enum/Flagsのtype name、member name、underlying type、numeric value、member declaration order、およびgenerated public enum shapeは、
generated C# sourceまたはそのconsumerに影響する。member declaration orderの変更はgenerated source orderを変更するが、numeric
meaning、domain、equality、comparison、selection、Masterdata binary semanticsを変更しない。

Enum/Flagsのnumeric valueはMasterdata binaryをまたぐpersistent identityではない。Masterdata binaryはcurrent schema/sourceから
再生成され、numeric value変更、member rename、member deletion、deleted valueの再利用を、このspecificationだけで自動migrationまたは
一律breaking changeとして扱わない。save data、network protocol、external database、public APIなどがgenerated numeric valueを外部
契約として永続化する場合、そのcompatibility classificationは本仕様の外部で定義する。

したがって、schema languageは、member rename、member deletion、numeric value変更、またはdeleted numeric valueの再利用を
許可してもよい（MAY）。これらをMasterdata binaryのautomatic migrationで解決することは要求しない。

Normal Enumのkey capabilityとunderlying integer orderは[Primitive Types仕様](primitives.md)を参照する。Flags Enumのkey/comparison
incompatibilityは本仕様が所有する。Table、Primary Key、Index、Referenceのexact syntaxは、それぞれの将来specificationが所有する。
Field ModifierとYAML syntaxも、それぞれのowner specificationを変更しない。

## 例

### Normal Enum

```yaml
kind: type
name: ItemRarity
enum:
  underlying: int
  members:
    - name: Common
      value: 1
    - name: Rare
      value: 10
    - name: Legendary
      value: 100
```

```yaml
rarity: Rare
```

このEnumでは、C#のlanguage-level `default(ItemRarity)` が `0` となっても、`0` がdeclared memberであることは要求されない。
Masterdata dataは `Common`、`Rare`、`Legendary` のいずれかをsymbolic nameで指定する。

```csharp
public enum ItemRarity : int
{
    Common = 1,
    Rare = 10,
    Legendary = 100,
}
```

このgenerated C#では、`default(ItemRarity) == 0` となり得る。これはMasterdata source dataでundefined valueを構成できることを意味しない。

### Flags Enum

```yaml
kind: type
name: Feature
flags:
  underlying: uint
  members:
    - name: None
      value: 0
    - name: Fire
      value: 1
    - name: Ice
      value: 2
```

```yaml
features: [Fire, Ice]
```

`features: [None]` はzero valueを表す。`features: [Ice, Fire]` は同じFlags valueを表すが、`features: []` および
`features: [None, Fire]` はinvalidである。

```csharp
[System.Flags]
public enum Feature : uint
{
    None = 0,
    Fire = 1,
    Ice = 2,
}
```

### Invalid declarations and data

```yaml
kind: type
name: InvalidFeature
flags:
  underlying: int
  members:
    - name: None
      value: 0
    - name: FireAndIce
      value: 3
```

`3` は2つのset bitを持つためnamed compositeであり、v1ではinvalidである。次のnormal Enum dataもinvalidである。

```yaml
rarity: 10
```

## Open Questions（未解決事項）

- Enum/Flagsのruntime failureに割り当てるexact Diagnostic Code、source span、exception、CLI presentationをどう定義するか。
- Enum/Flags fieldのMessagePack/MasterMemory exact attribute、formatter、resolver、serialization shapeをどう定義するか。
- 将来のTable、Index、Reference specificationで、normal Enum capabilityをどのkey positionへ適用するかをどう表現するか。
- generated numeric valueをMasterdata外のsave data、network protocol、external database、public APIが永続化した場合の外部compatibility ruleをどう定義するか。

これらは、closedなcurrent-schema Enum/Flags semanticsを再びOpen Questionにするものではない。

## 非目標

このspecificationは、Enum/Flagsのparser、validator、resolver、C# code generator、MasterMemory binary builder、MessagePack
integrationのimplementation、Table/Index/Reference syntax、external save/network/database compatibility systemを定義しない。
Flagsの任意のbit maskをC# castで手動生成した場合のruntime guardも、Masterdata source validationの範囲外である。
