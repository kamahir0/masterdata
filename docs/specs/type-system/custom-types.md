# Custom Type仕様（Custom Type）

Status: Proposed

Domain: Type System

## 概要

本proposalは、Custom Typeを1つ以上のnamed fieldから構成されるstructural value typeとして定義する。Custom Typeは
Value Objectとは異なり、field数ではなくtype categoryとdata shapeによって識別される。Custom Typeのdata representationは
常にmappingであり、generated C# representationはpublic `readonly struct`に固定する。

このdocumentは、canonical declaration、field type、field modifier、mapping validation、dependency cycle、stable field IDと
`reservedFields`、generated public API、structural equality、およびconstructor validation boundaryを定義する。Enum、Flags Enum、
MessagePackのexact shapeはowner specificationへ委譲する。generated C# identifierのobservable contractは、
[C#命名仕様](csharp-naming.md)が所有する。

## 用語

Custom Type、Value Object、Primitive Type、Field Modifier、Field ID、Type Declarationというtermは
[product terminology（用語）](../../product/terminology.md)に従う。Value Objectのnominal scalar semanticsは[Value Objects仕様](value-objects.md)、
field shapeとArray semanticsは[Field Modifiers仕様](field-modifiers.md)、Field IDの共通identity ruleは
[Field identity仕様](../compatibility/field-identity.md)が所有する。

## 規範要件

### SCHEMA-CUSTOM-001

Generated Custom Typeはimmutableでなければならない（MUST）。生成後にpublic APIを通じてfield valueを置換または変更できる
mutation pathを公開してはならない（MUST NOT）。

### SCHEMA-CUSTOM-002

Custom Typeは、1つ以上のnamed fieldから構成されるstructural value typeでなければならない（MUST）。`custom.fields` の
field数は1以上でなければならず（MUST）、zero-field Custom Typeはinvalidでなければならない（MUST）。

type categoryはfield数で決定してはならない（MUST NOT）。1-field Custom Typeは正式に許可され、Value Objectへ分類または
scalar representationへcollapseしてはならない（MUST NOT）。Value Objectはkey-compatible primitive scalarにnominal type
identityを与える別categoryであり、Value Objectのunderlying restrictionは[Value Objects仕様](value-objects.md)が所有する。

### SCHEMA-CUSTOM-003

Custom TypeはValue Objectと同じunified type-declaration modelを使用しなければならない（MUST）。1つのYAML documentには
正確に1つのtype declarationだけを含めなければならず（MUST）、type documentのpathまたはfilenameがCustom Type identityを
決定してはならない（MUST NOT）。

Custom Typeのcanonical declaration surfaceは次のとおりである。

```yaml
kind: type
name: Reward
custom:
  fields:
    - id: 0
      name: itemId
      type: ItemId
    - id: 1
      name: amount
      type: int
```

`kind` memberは `type`、declarationを識別するtop-level memberは `name`、category mappingはtop-levelの `custom`、field
collectionは `custom.fields` でなければならない（MUST）。各 `custom.fields` entryは、stable field identity、field name、base
typeをそれぞれ宣言する `id`、`name`、`type` memberを含まなければならない（MUST）。このdocumentは、exactly one type
declarationというunified boundaryをCustom Typeにも適用する。

### SCHEMA-CUSTOM-004

Custom Type fieldのbase typeは、Primitive Type、Value Object、Custom Type、Enum、またはFlags Enumのいずれかでなければ
ならない（MUST）。Table、Index、MasterReferenceその他のdomain relationshipをvalue typeとしてfieldへ追加してはならない
（MUST NOT）。EnumおよびFlags Enumをfield base typeとして使用する際のnumeric、capability、compatibility、generated API
semanticsは、それぞれの仕様が利用可能になった後にその仕様へ従う。このrequirementはEnumまたはFlags Enumの詳細仕様を
承認または実装するものではない。

### SCHEMA-CUSTOM-005

Custom Typeの各fieldには、[Field Modifiers仕様](field-modifiers.md)のRequired、Nullable、またはArray shapeを適用しなければ
ならない（MUST）。Custom Type fieldでも `T`、`T?`、`T[]` を使用し、`T?[]` と `T[]?` を許可してはならない（MUST NOT）。
modifierはfield-level optionで表現し、type-name stringへ埋め込んではならない（MUST NOT）。NullableとArrayの同時適用、
field entryのpresence、`null`、empty Array、explicit `false`、nullable/Arrayのgenerated representationは
`TYPE-FIELD-001` から `TYPE-FIELD-008` に従う。

### SCHEMA-CUSTOM-006

Custom Typeのdata representationは常にmappingでなければならない（MUST）。1-field Custom Typeであってもscalarへcollapseして
はならず（MUST NOT）、schemaで宣言されたfield nameをmapping memberとして使用しなければならない（MUST）。field entryの
presenceと各field valueのRequired、Nullable、Array semanticsは `TYPE-FIELD-003` に従う。

### SCHEMA-CUSTOM-007

Custom Type data mappingにschemaで宣言されていないmemberが存在する場合、validation errorとしなければならない（MUST）。
unknown memberをsilent ignore、discard、または既知fieldとして解釈してはならない（MUST NOT）。

### SCHEMA-CUSTOM-008

Custom Type自身は常にkey-incompatibleでなければならない（MUST）。field数、contained typeのcapability、field modifier、または
個別のdeclaration optionを理由に、Custom TypeをMasterMemoryのPrimary KeyまたはSecondary Keyへ直接利用可能なtypeへ昇格
させてはならない（MUST NOT）。Custom Type fieldにkey-compatibleなValue ObjectまたはPrimitiveが含まれていても、Custom Type
全体のkey compatibilityは変化しない。

### SCHEMA-CUSTOM-009

Custom Type dependency graphはacyclicでなければならない（MUST）。各Custom Type fieldが参照するCustom Typeをdependency
edgeとし、direct recursion（`A -> A`）とindirect recursion（`A -> B -> A`）をともにinvalidとしなければならない（MUST）。
NullableまたはArray modifierをedgeの間に適用してもcycleを許可してはならない（MUST）。`A -> B?` と `A -> B[]` は、
dependency graph上ではそれぞれ `A -> B` edgeとして扱う。

### SCHEMA-CUSTOM-010

Custom Typeの各fieldはstableなnumeric Field IDを必須としなければならない（MUST）。Field IDはCustom Type内でuniqueで
なければならず（MUST）、field nameとは独立したidentityでなければならない（MUST）。Custom TypeのField ID namespaceは、
他のCustom Typeおよびtableのnamespaceから独立していなければならない（MUST）。active/reserved IDのshared used-ID namespace、
collision、削除後の再利用禁止、およびrename時のID維持は、[Field identity仕様](../compatibility/field-identity.md)の
`COMPAT-FIELD-001`、`COMPAT-FIELD-002`、`COMPAT-FIELD-003`、および `COMPAT-FIELD-004` に従わなければならない（MUST）。

このstable Field IDはfuture MessagePack field identityの基礎となるが、exact attribute、numeric keyの配置、formatter、
wire shapeをこのproposalで決定するものではない。

### SCHEMA-CUSTOM-011

Generated Custom Typeは、次のcategoryに固定されたpublic `readonly struct`でなければならない（MUST）。各declared fieldは
public get-only propertyとして公開しなければならない（MUST）。Custom Typeごとに `class`、`record`、または `record struct`
を選択可能にしてはならない（MUST NOT）。property identifier、type identifier、namespace、collision、reserved keyword policy
は、Custom Typeのstructural semanticsとは別に[C#命名仕様](csharp-naming.md)が所有する。`implement-spec` はこのmappingを
独自に補完してはならない（MUST NOT）。

```csharp
public readonly struct Reward
{
    public ItemId ItemId { get; }
    public int Amount { get; }
    public Reward(ItemId itemId, int amount)
    {
        ItemId = itemId;
        Amount = amount;
    }
}
```

### SCHEMA-CUSTOM-012

Generated Custom Typeのpublic constructorは、すべてのfieldを引数として受け取らなければならない（MUST）。constructor
parameterの順序は、YAML declaration orderではなく、field IDのascending orderで決定しなければならない（MUST）。例えば
field IDが `5`、`1`、`3` の場合、constructorのorderは `1`、`3`、`5` でなければならない（MUST）。parameter identifierは
[C#命名仕様](csharp-naming.md)に従い、generated source formattingはこのrequirementの対象外である。

### SCHEMA-CUSTOM-013

Generated Custom Typeは、generated typeを `N` として、次のminimum equality public APIを公開しなければならない（MUST）。

- `IEquatable<N>`
- `public bool Equals(N other)`
- `public override bool Equals(object obj)`
- `public override int GetHashCode()`
- `public static bool operator ==(N left, N right)`
- `public static bool operator !=(N left, N right)`

Custom Typeのequalityはstructural equalityでなければならない（MUST）。すべてのfieldがequalである場合に限りCustom Typeを
equalとしなければならない（MUST）。`Equals(object)` は同じCustom Typeならtyped equalityを使用し、nullまたは異なるtypeには
`false`を返さなければならない（MUST）。`==` はtyped equalityと同じlogical resultを返し、`!=` は `==` のlogical negationで
なければならない（MUST）。このrequirementはordering、`IComparable<N>`、`CompareTo`、またはordering operatorを定義しない。

### SCHEMA-CUSTOM-014

Custom Typeの `GetHashCode()` はstructural equalityと整合しなければならない（MUST）。`a.Equals(b) == true` ならば
`a.GetHashCode() == b.GetHashCode()` でなければならない（MUST）。Array fieldについては `TYPE-FIELD-009` のsequence equality、
contents、およびorderに基づくhash semanticsを使用しなければならず（MUST）、Array reference identityだけに基づいては
ならない（MUST NOT）。具体的なhash algorithmはこのproposalで固定しない。

### SCHEMA-CUSTOM-015

Custom Typeのgenerated public constructorは、直接観測可能なfield input constraintを検証しなければならない（MUST）。最低限、
Required `string` fieldの `null` はrejectし、Nullable fieldの `null` は許可し、Array fieldの
`default(ImmutableArray<T>)` または `ImmutableArray<T>.IsDefault == true` はrejectしなければならない（MUST）。
`ImmutableArray<T>.Empty` はvalidである。language-levelの `default(CustomType)` はschema-validであることを保証されず、
invalid stateになり得る。

constructorは、nested Value Objectまたはnested Custom Typeのrecursive validityを再検証する必要はない（MAY）。このことは
nested typeのdefault stateをschema-validとする意味ではない。完全なschema/data validityはmaster-data build時のvalidationで
保証する。

### SCHEMA-CUSTOM-016

削除したCustom Type fieldのField IDは、同じCustom Typeの `custom.reservedFields` にreserved identityとして保持しなければ
ならない（MUST）。Custom Typeのcanonical reserved entryは、次のminimum shapeを使用しなければならない（MUST）。

```yaml
custom:
  reservedFields:
    - id: 1
      formerName: legacyAmount
      formerType: long
```

各entryは `id`、`formerName`、`formerType` を含まなければならない（MUST）。reserved IDは新しいactive fieldへ再利用しては
ならず（MUST NOT）、field renameではactive fieldのIDを維持し、`reservedFields` へ移してはならない（MUST NOT）。
`reservedFields` はCustom Typeごとのnamespaceに属し、別のCustom Typeの同じnumeric IDまたはtable field IDとはcollision
しない。このrequirementは、minimum shapeを超えるdeletion timestamp、reason、replacement、migration、compatibility
version、serialization metadataを定義しない。

## 検証ルール

このproposalの観測可能なvalidation outcomeは、`SCHEMA-CUSTOM-001` から `SCHEMA-CUSTOM-016` によって定義する。対象は、
Custom Type categoryとfield数の境界、unified declaration surface、許可されるfield base type、field modifier、mapping shape、
unknown member、key incompatibility、direct/indirect cycle、stable Field IDとreserved identity、readonly-struct public API、constructor order、
structural equality/hash、およびconstructor/default stateである。Enum/Flags Enumの詳細、C# naming policy、MessagePack exact shape、
released-schema migrationはこのproposalでは割り当てない。

## 受け入れ証拠

| Requirement（要件） | Success observation（成功時の観測） | Failure observation（失敗時の観測） | Suggested evidence（推奨する証拠） |
| --- | --- | --- | --- |
| `SCHEMA-CUSTOM-001` | generated Custom Typeのfieldがpublic mutation pathなしに保持される。 | public setterまたはその他のpublic mutation pathが公開される。 | Generated API immutability test。 |
| `SCHEMA-CUSTOM-002` | 1-fieldおよびmulti-field Custom Typeがstructural categoryとして受け入れられる。 | zero-fieldが受理される、1-fieldがValue Objectへ分類される、またはfield数だけでcategoryが決まる。 | Category-boundary and field-count tests。 |
| `SCHEMA-CUSTOM-003` | canonicalな `kind: type`、top-level `name`、top-level `custom.fields` を持つ1 document/1 declarationがpath非依存でresolveする。 | canonical key欠落、複数declaration、またはpath-derived identityが受理される。 | Type declaration structure tests。 |
| `SCHEMA-CUSTOM-004` | Primitive、Value Object、Custom Type、および各仕様が利用可能になった後のEnum/Flags Enumをfield base typeとして表現できる。 | Table、Index、MasterReferenceがvalue typeとして受理される、またはEnum/Flags詳細がこのspecから暗黙に追加される。 | Base-type resolution boundary tests。 |
| `SCHEMA-CUSTOM-005` | Custom Type fieldでRequired、Nullable、Arrayとexplicit `false` がField Modifiers仕様どおりに扱われる。 | suffix syntax、Nullable+Array、またはfield modifierのcategory-specificな別解釈が受理される。 | Custom field modifier tests。 |
| `SCHEMA-CUSTOM-006` | 1-fieldを含むすべてのCustom Type dataがmappingとして表現され、宣言fieldのentry presenceが検証される。 | 1-field Custom Typeのscalar shorthandが受理される、またはfield entryが省略可能になる。 | Mapping-shape and presence tests。 |
| `SCHEMA-CUSTOM-007` | schemaで宣言されたmemberだけを持つmappingが受け入れられる。 | `amuont` のようなunknown memberがsilent ignoreされずvalidation errorになる。 | Unknown-member validation test。 |
| `SCHEMA-CUSTOM-008` | field数やcontained typeにかかわらずCustom Typeがkey-incompatibleとして分類される。 | 1-fieldまたはkey-compatible fieldだけのCustom TypeがPrimary/Secondary Keyとして許可される。 | Key-capability tests。 |
| `SCHEMA-CUSTOM-009` | acyclicな `A -> B -> C` が受け入れられる。 | `A -> A`、`A -> B -> A`、またはNullable/Arrayを経由したcycleがrejectされる。 | Dependency graph validation tests。 |
| `SCHEMA-CUSTOM-010` | 各fieldがrequiredでuniqueなnumeric IDを持ち、name rename後もIDが維持され、Custom Typeごとのnamespaceが独立する。 | ID欠落、重複、nameとの同一視、削除IDの再利用、またはtable/別Custom Typeとのnamespace共有。 | Field-ID and evolution tests。 |
| `SCHEMA-CUSTOM-011` | generated typeがpublic `readonly struct`で、各fieldにpublic get-only propertyがあり、type/property identifierがC#命名仕様どおりである。 | class、record、record struct、setter付きproperty、fieldの欠落、または未定義のidentifier repair。 | C# compile/reflection/API-surface and naming tests。 |
| `SCHEMA-CUSTOM-012` | public constructorが全fieldをfield ID ascending orderで受け取り、各parameter identifierが対応するsource field nameと一致する。 | declaration order依存、field欠落、ID順と異なるparameter order、またはparameter nameの暗黙変換。 | Constructor-order and named-argument API tests。 |
| `SCHEMA-CUSTOM-013` | same structural valuesが `Equals(N)`、`Equals(object)`、`==` でequal、異なるfield valueまたはtypeがnot equal、`!=` がnegationとなり、`IEquatable<N>` と全required APIが存在する。 | reference identityだけの比較、required API欠落、またはordering APIをこのspecのcontractへ混入。 | Equality API and structural-equality tests。 |
| `SCHEMA-CUSTOM-014` | equalなCustom Typeと同じcontents/orderのArray fieldがequal hashを持つ。 | equal valuesのhash不一致、またはArray reference identityだけに依存。 | Structural hash-consistency and sequence tests。 |
| `SCHEMA-CUSTOM-015` | Required stringのnullがconstructorでrejectされ、Nullable nullと `ImmutableArray<T>.Empty` が受け入れられ、default Arrayがrejectされる。`default(CustomType)` はinvalidになり得る。nested typeのrecursive validityはconstructorの必須検査ではない。 | null/default Arrayが受理される、またはconstructorがnested default stateを再帰的に必須検証すると主張する。 | Constructor validation-boundary and default-state tests。 |
| `SCHEMA-CUSTOM-016` | `custom.reservedFields` が `id`、`formerName`、`formerType` を保持し、active IDとのcollisionなしに削除IDを保持する。renameはactive IDを維持する。 | active/reserved ID collision、reserved IDの再利用、minimum memberの欠落、またはrename時のreservedへの移動。 | Reserved-field-ID schema validation and evolution tests。 |

`SCHEMA-CUSTOM-016` のminimum shapeとactive/reserved namespaceは、次のsuccess caseで観測できる。

```yaml
kind: type
name: Reward
custom:
  fields:
    - id: 0
      name: itemId
      type: ItemId
  reservedFields:
    - id: 1
      formerName: oldAmount
      formerType: int
```

次はactive/reserved collisionのfailure caseである。

```yaml
kind: type
name: Reward
custom:
  fields:
    - id: 1
      name: amount
      type: int
  reservedFields:
    - id: 1
      formerName: oldAmount
      formerType: long
```

あるfieldをID `1` で削除してreserved identityへ移した後、そのID `1` を新しいactive fieldへ割り当てる変更も、同じ
used-ID namespaceにおける再利用としてfailureになる。

## 互換性

Custom Typeのcategory、mapping representation、stable Field ID、reserved identity、field ID順のconstructor、generated public API、
structural equality、Array immutabilityは、将来のschema/dataとgenerated artifactに影響する。Field IDの共通identity ruleと削除後の
再利用禁止は[Field identity仕様](../compatibility/field-identity.md)を参照し、Custom Typeのdeleted Field IDは
`custom.reservedFields` に保持する。このproposalは、minimum shapeを超えるtombstone metadata、MessagePack attribute/wire shape、
released schema evolutionのclassificationを決定しない。generated API namingは[C#命名仕様](csharp-naming.md)が所有する。

## 例

### 1-field Custom Type

1-fieldであることはValue Objectの条件ではない。`custom.fields` を使用するため、dataはmappingでなければならない。

```yaml
kind: type
name: EnabledState
custom:
  fields:
    - id: 0
      name: value
      type: bool
```

```yaml
enabledState:
  value: true
```

次のscalar shorthandはinvalidである。

```yaml
enabledState: true
```

### Multi-field Custom Type

```yaml
kind: type
name: Reward
custom:
  fields:
    - id: 0
      name: itemId
      type: ItemId
    - id: 1
      name: amount
      type: int
    - id: 2
      name: note
      type: string
      nullable: true
    - id: 3
      name: tags
      type: string
      array: true
```

```yaml
reward:
  itemId: 1001
  amount: 3
  note: null
  tags: []
```

### Invalid dependency cycles

```text
A -> A
A -> B -> A
A -> B? -> A
A -> B[] -> A
```

## Open Questions（未解決事項）

- Custom Type field IDのallocation policyは何か。
- `custom.reservedFields` のminimum shapeを超えて、deletion timestamp、reason、replacement、migration、compatibility version、serialization metadataを保持するか。
- EnumおよびFlags Enumをfield base typeとして使用する際の、各仕様とのdependency boundaryと実装順序。
- MasterMemory/MessagePack integrationで必要なexact attribute、wire shape、formatter、serialization constructor、およびresolver behavior。
- Custom Typeの追加、field rename、field deletion、field ID変更、field type変更、modifier変更をreleased schemaに対してどうclassificationするか。
- Generated public constructorでdirect constraint違反を通知するexact exceptionまたはDiagnostic mappingを定義するか。

## 非目標

このproposalは、Rust type registry、AST/IR resolver、Custom Type parser、recursive cycle detector、nullable/Array validator、C#
codegen、Enum、Flags Enum、Index、MasterReference、MessagePack generator、MasterMemory integration、production binary builder、
released-schema migration policyを実装しない。
