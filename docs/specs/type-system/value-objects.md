# Value Objects仕様（Value Objects）

Status: Proposed

Domain: Type System

## 概要

本proposalは、Value Objectをkey-compatibleなprimitive scalarへnominal type identityを与える型として定義する。
unified type-declaration boundary、scalar data representation、generated C# category、equality、capability、
directional implicit-conversion configuration、underlying-value `ToString()` behavior、およびValue Objectと
Custom Typeのsemantic boundaryを定める。Custom Typeのstructural value semanticsは[Custom Type仕様](custom-types.md)
が所有する。

## 用語

Value Object、Primitive Type、Type Declaration、Generated C# Type Nameというtermは
[product terminology（用語）](../../product/terminology.md)に従う。primitive capability tableは[Primitive Types仕様](primitives.md)が
所有し、field-levelのNullable/Array behaviorは[Field Modifiers](field-modifiers.md)が所有する。Custom Typeの
structural value、mapping representation、field IDは[Custom Type仕様](custom-types.md)が所有する。

`SCHEMA-VO-001` は以前のDraft type-system overviewから引き継ぎ、ここでcanonical Value Object requirementとして
refineする。approvalとimplementationでは、参照するprimitiveおよびfield-modifier contractへのdependencyを
保ったまま、それぞれのfile-level statusを混在させない。

## 規範要件

### SCHEMA-VO-001

Value Objectはimmutable、equality-capable、かつrepositoryの将来のMessagePack integrationを通じて
serializableでなければならない（MUST）。このproposalにおけるgenerated representationとserialization
detailは、generated artifactをeditのSource of Truthにせず、これらのobservable propertyを保たなければ
ならない（MUST）。

### SCHEMA-VO-002

Value Objectは、key-compatibleなprimitive scalarにnominal type identityを与え、正確に1つのunderlying primitive
typeをwrapする型でなければならない（MUST）。underlying typeは `TYPE-PRIMITIVE-005` が定義するkey-compatibleな
次の5つのprimitiveから選ばなければならない（MUST）。

- `int`
- `uint`
- `long`
- `ulong`
- `string`

したがって、`bool`、`float`、`double` をunderlyingとするValue Object declarationはinvalidでなければならない
（MUST）。Value Objectであること自体は、実際にPrimary KeyまたはSecondary Keyとして使用されることを要求しない。

### SCHEMA-VO-003

Value Objectは、別のValue Object、Enum、Flags Enum、Custom Type、Array shape、Nullable shapeをunderlyingにしては
ならない（MUST NOT）。Value Objectのunderlying typeは、field-level modifierを適用する前の、
`SCHEMA-VO-002`で許可されたprimitiveである。1つ以上のnamed fieldからなるstructural valueは、field数にかかわらず
Value ObjectではなくCustom Typeである。

### SCHEMA-VO-004

Value Objectは、Value Object専用のtop-level declaration familyではなくunified type-declaration modelを
使用しなければならない（MUST）。1つのYAML documentには、正確に1つのtype declarationだけを含めなければ
ならず（MUST）、type documentのpathまたはfilenameがtype identityを決定してはならない（MUST NOT）。

1つのValue Object declarationにおけるcanonical surfaceは次のとおりである。

```yaml
kind: type
name: ItemId
valueObject:
  underlying: int
```

`kind` memberは `type` でなければならない（MUST）。declarationはValue Objectを識別するtop-levelの
`name` memberと、top-levelの `valueObject` mappingを含まなければならない（MUST）。`valueObject` mappingは
`underlying` memberを含まなければならない（MUST）。このsurfaceは、Enum、Flags Enum、Custom Typeにも同じ
type-document boundaryを使用するunified modelのValue Object形式である。

### SCHEMA-VO-005

Value Objectのdata representationは、underlying primitiveと同じscalar representationを使用しなければ
ならない（MUST）。例えば、`int` をwrapする `ItemId` は `itemId: 1001` と表現し、`value` propertyを含む
wrapper objectとしては表現しない。

### SCHEMA-VO-006

すべてのValue Objectのgenerated C# representationは、次のminimum public APIを持つpublic `readonly struct`
でなければならない（MUST）。Equality interfaceとmembersの詳細は `SCHEMA-VO-011` が定義する。

```csharp
public readonly struct ItemId : IEquatable<ItemId>
{
    public int Value { get; }
    public ItemId(int value)
    {
        Value = value;
    }
}
```

underlying typeを `T`、Value Object nameを `N` とすると、generated typeは `public readonly struct N` で
なければならず（MUST）、public setterなしの `public T Value { get; }` を公開し（MUST）、argumentを `Value` に
格納するpublic constructor `N(T value)` を公開しなければならない（MUST）。Value Objectは、typeごとの
representation optionによって `class`、`record`、または `record struct` として選べてはならない（MUST NOT）。
`N` へ到達するsource nameのmappingは、[C#命名仕様](csharp-naming.md)が管理する。Value Objectの固定property
`Value`、equality member、`ToString()` memberとのcollisionも、同仕様のgenerated member collision ruleに従う。
generated source formattingとMessagePack attribute / serialization memberの詳細は、このrequirementの対象外である。

### SCHEMA-VO-007

Value Objectは、`SCHEMA-VO-002`で許可されたunderlying primitiveがkey-compatibleであることを継承し、常に
key-compatibleでなければならない（MUST）。Value Object declarationごとにkey compatibilityをopt inまたはopt out
してはならず（MUST NOT）、実際にindexへ使用されているかどうかはこのcapabilityを変更しない。

orderingまたはcomparison capabilityは、approved type-system specificationがunderlying primitiveについて明示的に
定義した場合に、そのcapabilityを継承しなければならない（MUST）。comparisonの具体的なinterface、operator、または
floating-point orderingは、このrequirementでは定義しない。NullableとArrayのfield modifierは、
`TYPE-FIELD-004` に従い、field shapeを引き続きkey-incompatibleにする。

### SCHEMA-VO-008

Value Object declarationは、次の各方向のimplicit conversionを独立してenableまたはdisableできなければ
ならない（MUST）。

- underlying primitiveからValue Objectへ。
- Value Objectからunderlying primitiveへ。

これらのcapabilityに対するcanonical YAML propertyは次のとおりである。

```yaml
valueObject:
  underlying: int
  conversions:
    fromUnderlyingImplicit: true
    toUnderlyingImplicit: false
```

`fromUnderlyingImplicit` と `toUnderlyingImplicit` は `valueObject.conversions` 配下のboolean optionでなければ
ならない（MUST）。`conversions` mappingは省略してもよい（MAY）。省略した場合、またはmapping内のいずれかの
optionを省略した場合、対応するcapabilityはdisableしなければならない（MUST）。この2つの独立したcapabilityの
4つの組み合わせをすべて表現できなければならない（MUST）。

`fromUnderlyingImplicit` がenableの場合、generated C# APIはunderlying C# typeからValue Objectへのimplicit
conversionを提供しなければならない（MUST）。disableの場合、そのimplicit conversionを提供してはならない
（MUST NOT）。Value Objectからunderlying C# typeへのconversionについても、`toUnderlyingImplicit` に同じruleを
独立して適用する。generated source formattingとmember orderingは、このcontractの対象外である。

### SCHEMA-VO-009

directional implicit-conversion settingは、generated C# APIのconversion behaviorだけに影響しなければならない
（MUST）。key compatibility、comparison capability、equality、MessagePack wire identity、underlying primitive
identity、field modifier semanticsを変更してはならない（MUST NOT）。key compatibilityはconversion settingとは
独立して `TYPE-PRIMITIVE-005` と `TYPE-FIELD-004` によって決定する。

### SCHEMA-VO-010

generated Value Objectは `public override string ToString()` を公開しなければならない（MUST）。このmethodは
underlying valueのtextual representationを返さなければならない（MUST）。`ItemId(1001)` のようなtype-name-wrapped
debug representationをdefault contractとして使用してはならない（MUST NOT）。

- 現在 `SCHEMA-VO-002` が許可するnumeric underlying（`int`、`uint`、`long`、`ulong`）では、textual representationに
  invariant-culture formattingを使用し、runtime `CurrentCulture` によって変化してはならない（MUST）。
- `string` では、結果はunderlying string valueそのものでなければならない（MUST）。

このrequirementは、現行の `SCHEMA-VO-002` が許可するunderlying typeにだけ適用する。現行未許可のprimitiveを将来の
specification changeでunderlyingへ追加する場合、そのtypeの `ToString()` contractも同じchangeで別途定義する。

### SCHEMA-VO-011

各generated Value Objectは、generated Value Object typeを `N` として、`IEquatable<N>` を実装しなければならず
（MUST）、次のpublic APIをすべて公開しなければならない（MUST）。

- `public bool Equals(N other)`
- `public override bool Equals(object obj)`
- `public override int GetHashCode()`
- `public static bool operator ==(N left, N right)`
- `public static bool operator !=(N left, N right)`

Value Object `N<T>` では、typed equality `N(a).Equals(N(b))` はunderlying value `a` と `b` のequalityと同じ
logical resultを持たなければならない（MUST）。`Equals(object)` overrideは、`obj` が同じValue Object typeの場合は
typed equalityを使用しなければならない（MUST）。`obj` が `null` または異なるtypeの場合は `false` を
返さなければならない（MUST）。

`GetHashCode()` はunderlying valueのequality semanticsから導出し、equalなValue Objectが常にequalなhash codeを
持たなければならない（MUST）。`==` operatorはtyped equalityと同じlogical resultを持たなければならない
（MUST）。`!=` は `==` のlogical negationでなければならない（MUST）。

Equalityはkey compatibilityおよびordering/comparison capabilityから独立している。すべてのValue Objectは
equality-capableであり、かつ常にkey-compatibleである。Equalityは `SCHEMA-VO-008` のimplicit conversion settingの
4つの組み合わせすべてで同一でなければならない（MUST）。このrequirementは `IComparable<N>`、`CompareTo`、または
ordering operatorを定義しない。それらは該当するprimitive comparison capability specificationが引き続き管理する。

### SCHEMA-VO-012

Value Objectがlanguage-levelの`readonly struct`であることにより、`default(N)` はschema-validなValue Objectである
ことを保証されない（MUST NOT）。`default(N)` はinvalid stateになり得る。master-data build pathとgenerated public
constructor pathは、許可されたunderlying primitiveの直接のvalidity constraintを満たすvalueを扱わなければならない
（MUST）。特に、`string` をunderlyingとするValue Objectのpublic constructorは `null` をrejectしなければならない
（MUST）。Value Objectにはnested Value Objectまたはnested Custom Typeがないため、constructorがnested typeのrecursive
validityを再検証するcontractは定義しない。

## 検証ルール

このproposalの観測可能なvalidation outcomeは、`SCHEMA-VO-001` から `SCHEMA-VO-012` によって定義する。対象は、
key-compatible underlying restriction、wrapper restriction、one-document/one-declaration structure、scalar
representation、readonly-struct category、minimum public API、equality APIとsemantics、inherited capability、
directional conversion configuration、conversion isolation、supported-underlying valueの `ToString()` behavior、default/constructor
boundaryである。exact diagnostic codeとfinal MessagePack attribute shapeは未割り当てである。

## 受け入れ証拠

| Requirement（要件） | Success observation（成功時の観測） | Failure observation（失敗時の観測） | Suggested evidence（推奨する証拠） |
| --- | --- | --- | --- |
| `SCHEMA-VO-001` | Value Objectがimmutable、equality-capableであり、approved integrationを通じてserializableである。 | mutable、non-equality-capable、またはnon-serializableなrepresentationがrejectされる。 | Generated representation and serialization tests。 |
| `SCHEMA-VO-002` | `int`、`uint`、`long`、`ulong`、`string` のいずれか1つをunderlyingとするnominal scalar declarationが受け入れられる。 | `bool`、`float`、`double`、non-primitive、または複数underlyingを持つdeclarationがrejectされる。 | Underlying-type and capability validation tests。 |
| `SCHEMA-VO-003` | 許可されたprimitiveを直接wrapするValue Objectが受け入れられ、structural field typeはCustom Typeとして区別される。 | nested、enum、custom、array、nullable wrapper、またはfield数だけを根拠にしたcategory変更がrejectされる。 | Forbidden-wrapper and category-boundary tests。 |
| `SCHEMA-VO-004` | `kind: type`、top-level `name`、top-level `valueObject.underlying` を持つ1つのtype documentが、pathに依存せずnamed Value Object declarationを生成する。 | 複数declaration、canonical memberの欠落、またはpath-derived identityがrejectされる。 | Type document structure tests。 |
| `SCHEMA-VO-005` | Value Object fieldがunderlying primitiveと同じscalar representationを使用する。 | `value` propertyを含むwrapper objectが、定義されたrepresentationとして要求または受け入れられる。 | Data representation tests。 |
| `SCHEMA-VO-006` | generated outputが、C#命名仕様に従うtype identifierを持ち、public `Value` get-only propertyとpublic underlying-value constructorを持つpublic readonly structである。 | private type、setterを持つ `Value` property、public constructorの欠落、argumentを保存しないconstructor、class/record representation、または未定義のidentifier repair。 | C# generation golden/compile/API-surface and naming test。 |
| `SCHEMA-VO-007` | すべてのvalid Value Objectがkey-compatibleとして報告され、実際のindex使用なしでもそのcapabilityを持つ。approvedなunderlying comparison capabilityがあれば同じoutcomeが継承される。 | Value Objectごとのoverride、実際のkey使用を要する判定、またはunderlyingと異なるcomparison capability。 | Capability inheritance tests。 |
| `SCHEMA-VO-008` | canonical conversion syntaxが `false/false`、`true/false`、`false/true`、`true/true` を独立して表現し、省略されたmapping/optionがdisableとなり、enableされた方向だけimplicit C# operatorを持つ。 | 欠落した方向がenableになる、一方のsettingが他方を変える、またはdisableされた方向のoperatorが生成される。 | Conversion syntax, default, and generated-operator tests。 |
| `SCHEMA-VO-009` | conversion settingの変更がgenerated C# conversion surfaceだけを変更する。 | key compatibility、comparison、equality、wire identity、underlying identity、field modifier behaviorがconversion settingで変化する。 | Conversion-isolation tests。 |
| `SCHEMA-VO-010` | 現行valid underlyingの `ToString()` がunderlying textual valueを返し、numeric outputは `CurrentCulture` に依存せず、underlying stringは変更されない。 | resultが `CurrentCulture` で変化する、underlying stringを変更する、または `ItemId(1001)` のようなtype-name wrapperを追加する。 | Generated API behavior tests under multiple cultures and supported primitive categories。 |
| `SCHEMA-VO-011` | generated typeが `IEquatable<N>` を実装し、typed/object equality、`GetHashCode()`、`==`、`!=` を公開する。`N(a).Equals(N(a))` と `N(a) == N(a)` はtrue、`N(a) != N(a)` はfalse、異なるunderlying valueはunderlying equalityに従い、equal objectはequal hashを持ち、`Equals(object)` はnullまたは異なるtypeに対してfalseとなる。 | required API memberの欠落、typed/object equalityとunderlying equalityの不一致、equal valueで異なるhash、または `!=` が `==` のnegationでない。 | API-surface compile/reflection test plus typed equality, object equality, hash-consistency, and operator tests。 |
| `SCHEMA-VO-012` | validなunderlying valueを渡すpublic constructor pathがvalid stateを生成し、`default(N)` がinvalidになり得ることを許容する。 | `string` underlyingのconstructorがnullをvalid valueとして受理する、またはlanguage-level defaultを必ずschema-validと主張する。 | Constructor-boundary and default-state tests。 |

## 互換性

Value Objectのunderlying vocabularyをkey-compatible primitiveへ狭めること、scalar data representation、generated
C# API、implicit conversion surfaceは将来のschema/dataとgenerated artifactに影響し得る。このproposalは既存の
released schemaに対するimplicit migration、generated API changeのcompatibility classification、field-ID behaviorを
定義しない。これらのchoiceはOpen Questionであり、このproposalがApprovedになるまでimplementation authorityではない。
`float`、`double`、`bool` をValue Object underlyingへ許可することは、別のsemantic changeでなければならない。

## 例

次のexampleはnon-normativeである。

```yaml
# type declaration
kind: type
name: UserCode
valueObject:
  underlying: string
```

```yaml
# data value in a table record
userCode: "player-001"
```

次は、directional conversion settingを持つcanonical declarationのnon-normative exampleである。

```yaml
kind: type
name: ItemId
valueObject:
  underlying: int
  conversions:
    fromUnderlyingImplicit: true
    toUnderlyingImplicit: false
```

次のdeclarationは、underlying typeが現行のkey-compatible primitiveではないためinvalidである。

```yaml
kind: type
name: BoolState
valueObject:
  underlying: bool
```

```yaml
kind: type
name: Ratio
valueObject:
  underlying: double
```

```yaml
kind: type
name: NestedId
valueObject:
  underlying: ItemId
```

```yaml
kind: type
name: NullableId
valueObject:
  underlying: int?
```

## Open Questions（未解決事項）

- explicit conversion operatorまたはhelper APIも生成するか。また、それらにどのcompatibility guaranteeを与えるか。
- MasterMemoryとUnity-compatible C# projectに必要なMessagePack attributeとgenerated shapeは何か。
- 各primitive、特に `bool`、`float`、`double` にどのordering/comparison capabilityがあるか。finite floating-point valueのordering contractも含む。primitive specificationはnon-finite valueをrejectする。
- primitive wrapper contractを変更せずに、どのcustom validation constraintを追加できるか。
- Value Objectの追加、underlying typeの変更、renameをreleased schemaに対してどうclassificationするか。
- YAML parser dialectが各canonical scalarをどのように分類し、type validationへどのscalar valueを渡すか。

## 非目標

このproposalは、type registry、AST/IR resolver、Value Object parser、nullable/array validator、readonly-struct generator、
MessagePack generator、key generator、Enum、Flags Enum、Custom Type、Index、MasterReference、production binary
builderを実装しない。
