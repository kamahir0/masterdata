# Type System仕様（Type-system）

Type-system ruleは、独立してreviewできるcanonical documentへ分割している。これにより、
Primitive、Field Modifier、Value Object、Enum/Flags、Custom Typeのbehaviorを、1つの
document-wide statusに縛られずDraft、Proposed、Approved、Implementedへ進められる。この
directory indexはnon-normativeであり、要件を所有するのはlink先のdocumentである。

現行のtype categoryの境界は次のとおりである。

```text
Primitive
├─ key-compatible: int, uint, long, ulong, string
│  └─ nominal scalarとしてValue Objectへ宣言できる
└─ key-incompatible: bool, float, double

Value Object
├─ nominal scalar
└─ always key-compatible

Custom Type
├─ structural mapping
├─ one or more fields
└─ always key-incompatible
```

field数はValue ObjectとCustom Typeを分類しない。1-field Custom TypeもCustom Typeであり、dataはmappingとして扱う。
Value Objectのdataはscalarである。Array shapeとgenerated C#の`ImmutableArray<T>` representationは、
[Field Modifiers仕様](field-modifiers.md)が全field categoryに共通して定義する。
Value ObjectおよびCustom Typeのtype name、Custom Type fieldのsource name、generated property、constructor
parameterのmappingは[C#命名仕様](csharp-naming.md)が所有する。Tableの`table` identityと`csharpName` presentation nameは
この仕様の対象外である。

- [Primitive Types仕様](primitives.md) — `Status: Approved`
- [Field Modifiers仕様](field-modifiers.md) — `Status: Approved`
- [Value Objects仕様](value-objects.md) — `Status: Approved`
- [C#命名仕様](csharp-naming.md) — `Status: Approved`
- [Enums and Flags仕様](enums.md) — `Status: Approved`
- [Custom Types仕様](custom-types.md) — `Status: Approved`

これらのfileでは既存のRequirement IDを保持する。documentを分割してもIDをrenameまたは
reassignしない。
