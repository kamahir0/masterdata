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

## Planned schema shape（予定するschemaの形）

```yaml
kind: schema
table: item
csharpName: ItemMaster
fields:
  - id: 0
    name: id
    type: ItemId
reservedFields:
  - id: 1
    formerName: oldName
    formerType: string
```

Rust ASTはschema declarationをtyped（`SchemaDocument`、`FieldDefinition`、`ReservedField`）に
保つ。将来のtype/index/reference featureがunstructured mapへ崩れることを防ぐためである。

現在のscaffoldが認識するdocumentは `schema` と `data` だけである。Proposedの
[Value Objects仕様（Value Objects specification）](type-system/value-objects.md)は、将来のunified type-declaration
documentの方向として `kind: type` を定義する。このproposed kindはcurrent parserでは受け付けず、
specificationがApprovedかつImplementedになるまでimplementation contractではない。

current scaffoldは `table` をproject-localなlogical table identityとして使用する。存在する場合の
`csharpName` はgenerated C# type-name overrideであり、2つ目のtable identityではない。以前に示した
`tableId` fieldはRust modelやgeneratorでconsumeされておらず、current scaffoldには意図的に存在しない。
この方向のcompatibility implicationは、current-scaffold directionについてAcceptedとなった
[`docs/rfcs/0001-table-identity.md`](../rfcs/0001-table-identity.md) に記録する。global identity、
rename migration、released-schema compatibility、legacy `tableId` migration、cross-project identityは
引き続きOpen Questionである。

## Planned type declaration（予定するtype declaration）

Type Systemは、Value Object、Enum、Flags Enum、Custom Typeを同じunified type-declaration boundaryで扱う方向である。
Value Objectのcanonical surfaceは[Value Objects仕様](type-system/value-objects.md)、Custom Typeのcanonical surfaceは
[Custom Type仕様](type-system/custom-types.md)が所有する。

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
    - id: 0
      name: itemId
      type: ItemId
```

これらのtype declarationはProposed specificationのdirectionであり、current parserは `kind: type`、`valueObject`、または
`custom` をまだ受け付けない。specificationがApprovedかつImplementedになるまで、current parserのimplementation contract
ではない。type documentのpathまたはfilenameはtype identityを決めない。1つのYAML documentに複数のtype declarationを入れる
形式は使用しない。Value ObjectとCustom Typeのtype name、およびCustom Type field nameのASCII lexical ruleとgenerated
C# identifier mappingは、[C#命名仕様](type-system/csharp-naming.md)が所有する。これはTableの `table` identityや
`csharpName` presentation nameへ適用されない。

## YAML subsetのOpen Questions

productが承認したYAML subsetはまだない。refinementでは、current parserからこれらの選択を導出せず、
見える状態に残さなければならない（MUST）。

- anchorとaliasを許可するか。
- merge keyを許可するか。
- 1つのfileで `---` によって区切られた複数documentを許可するか。
- custom tagを許可するか。
- duplicate mapping keyをどのようにdiagnoseするか。
- numericおよびtimestamp-looking scalarをどのように解釈するか。
- schema/document-levelでcanonical specificationが所有していないunknown YAML memberをreject、ignore、またはround-trip editingのためにpreserveするか。

Open Questionsには、GUIがYAMLを書き戻す際にcomment、formatting、quoteを保持する必要があるかも含まれる。
parser/library selectionは、[`docs/rfcs/0002-yaml-parser-library.md`](../rfcs/0002-yaml-parser-library.md)
で別途追跡する。

このOpen Questionはschema/document envelopeなどのgenericなmemberの扱いだけを対象とする。Custom Type data mapping内の
schema未定義memberは、この一般的なquestionの対象外であり、`SCHEMA-CUSTOM-007` が所有するvalidation ruleに従って
validation errorとする。
