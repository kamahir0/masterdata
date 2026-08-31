# Schema言語（Schema language）

Status: Draft

## Document envelope（ドキュメントの外枠）

すべてのYAML source documentは、自身の意味を宣言しなければならない（MUST）。

```yaml
kind: schema
table: item
```

または:

```yaml
kind: data
table: item
```

`kind` が未知または欠落している場合はparse errorとする。1つのtableは、1つ以上のdata
documentからdataを受け取ってもよい（MAY）。Data fileはfilenameやdirectoryではなく、
宣言されたtable identityによってmergeする。

## Current scaffoldとcanonical persisted field shape

```yaml
kind: schema
table: item
csharpName: ItemMaster
fields:
  - key: 0
    name: id
    type: ItemId
```

Rust ASTはschema declarationをtyped（`SchemaDocument`、`FieldDefinition`）に保つ。Type declarationも
`TypeDocument`として同じdocument boundaryでtypedに扱い、将来のindex/reference featureがunstructured mapへ
崩れることを防ぐ。

上記の`key`は、specification change 0003のApplied deltaを反映した、現在のpersisted fieldのcanonical surfaceである。`key`の
serialization-only semanticsは[Table / Primary Key / Secondary Key仕様](table-and-keys.md)の`SCHEMA-KEY-001`が所有する。
current scaffoldの実装・ASTが旧`id`や`reservedFields`を保持している場合、それはimplementation gapを示すevidenceであり、
現行canonical contractの代替ではない。

## Approved Table / Key shape（Approved Table / Keyのshape）

Table schemaのpersisted field surfaceは、[Table / Primary Key / Secondary Key仕様](table-and-keys.md)が所有する。persisted fieldでは、
Field IDではなくMessagePack専用の`key`を使用する。

```yaml
kind: schema
table: item-category
csharpName: ItemCategoryMaster
fields:
  - key: 0
    name: id
    type: ItemId
primaryKey:
  fields: [id]
secondaryKeys:
  - fields: [category]
    nonUnique: true
```

このshapeはApproved Table/Key specificationとApplied Field Identity changeの内容を示すcanonical contractである。current
implementationはMessagePack keyをASTとresolved Table modelへ保持し、selection後のrecord validation、Primary/Secondary Keyのvalidation、
uniqueness、canonical ordering、およびC# loweringまで行う。Referenceとproduction binary orchestrationは別sliceのscopeである。
`key`はMessagePack `[Key(n)]`へ対応するが、logical field identity、rename、deletion、addition、secondary-key identity、reference identity、または
schema migration identityを表さない。Custom Typeのpersisted fieldも同じ`key` modelを使用する。

現在のscaffoldが認識するdocumentは `schema`、`data`、`type` である。Approvedの
[Value Objects仕様（Value Objects specification）](type-system/value-objects.md)および[Custom Type仕様](type-system/custom-types.md)は、
unified type-declaration documentとして `kind: type` を定義する。これらのtype declarationはcurrent parserがtyped documentとして
受け付け、Type System validationとC# generationのimplementation contractを構成する。type documentのpathまたはfilenameはtype identityを
決めず、1つのYAML documentに複数のtype declarationを入れる形式は使用しない。

current scaffoldは `table` をproject-localなlogical table identityとして使用する。存在する場合の
`csharpName` はgenerated C# type-name overrideであり、2つ目のtable identityではない。以前に示した
`tableId` fieldはRust modelやgeneratorでconsumeされておらず、current scaffoldには意図的に存在しない。
この方向のcompatibility implicationは、current-scaffold directionについてAcceptedとなった
[`docs/rfcs/0001-table-identity.md`](../rfcs/0001-table-identity.md) に記録する。global identity、
rename migration、released-schema compatibility、legacy `tableId` migration、cross-project identityは
引き続きOpen Questionである。

## Type declaration（type declaration）

Type Systemは、Value Object、Enum、Flags Enum、Custom Typeを同じunified type-declaration boundaryで扱う。
Value Objectのcanonical surfaceは[Value Objects仕様](type-system/value-objects.md)、Enum/Flagsのcanonical surfaceは
[EnumとFlags Enum仕様](type-system/enums.md)、Custom Typeのcanonical surfaceは[Custom Type仕様](type-system/custom-types.md)が
それぞれ所有する。Type Systemの初回implementation sliceでは、これらのdeclarationをtyped AST、symbol table、resolved modelへ変換する。

```yaml
kind: type
name: ItemId
valueObject:
  underlying: int
```

```yaml
kind: type
name: Reward
custom:
  fields:
    - key: 0
      name: itemId
      type: ItemId
```

これらのtype declarationのcanonical surfaceは、それぞれのowner specificationで定義される。type documentのpathまたはfilenameはtype identityを
決めず、1つのYAML documentに複数のtype declarationを入れる形式は使用しない。Value ObjectとCustom Typeのtype name、およびCustom Type field nameの
ASCII lexical ruleとgenerated C# identifier mappingは、[C#命名仕様](type-system/csharp-naming.md)が所有する。これはTableの `table` identityや
`csharpName` presentation nameへ適用されない。

上記Custom Type例の`key`は、Applied specification change 0003とApproved Custom Type / Table / Key specificationが所有する現在の
canonical surfaceである。current Type System resolverはこのshapeを受け付けるが、`key`はMessagePack serialization metadataとしてのみ扱い、
constructor orderやlogical field identityへ流用しない。実装状態を理由に旧Field ID modelをcurrent authorityとして扱ってはならない。

## Masterdata YAML subsetとの関係

Masterdataが受理するYAMLの構造・collection・scalar semanticsは、[Masterdata YAML subset仕様](yaml-subset.md)がcanonical ownerである。
このspecificationは `Status: Draft` であり、YAML subsetは `Status: Approved` である。current parserのdefault behaviorからsubsetの
意味を導出してはならない（MUST NOT）。parser/libraryの選択は[`docs/rfcs/0002-yaml-parser-library.md`](../rfcs/0002-yaml-parser-library.md)で
別途追跡する。

timestamp-looking plain scalarの扱い、GUI saveでのcomment・formatting・quote保持、およびparser/libraryの選択・migration・maintenanceの未決定事項は、YAML subset仕様の
`Open Questions`に残る。Custom Type data mapping内のschema未定義memberは、このgeneric routingの対象外であり、`SCHEMA-CUSTOM-007`が
所有するvalidation ruleに従ってvalidation errorとする。
