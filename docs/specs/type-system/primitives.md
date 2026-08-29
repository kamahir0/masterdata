# Primitive Types仕様（Primitive Types）

Status: Proposed

Domain: Type System

## 概要

本proposalは、初期Primitive Type vocabulary、直接のkey compatibility、strict scalar validation、
finiteなfloating-point value、初期のstring ruleを定義する。YAML parser dialectやcapabilityを
表現するimplementation data structureは定義しない。

## 用語

Primitive Type、Field、Value Objectというtermは[product terminology（用語）](../../product/terminology.md)
に従う。Field-levelのpresenceとmodifier behaviorは[Field Modifiers](field-modifiers.md)に属する。

## 規範要件

### TYPE-PRIMITIVE-001

初期Primitive Type vocabularyは、次のcanonical type nameだけをサポートしなければならない（MUST）。

| Type name（型名） | Value domain（値域） |
| --- | --- |
| `bool` | 真偽値 |
| `int` | 符号付き32-bit整数値 |
| `uint` | 符号なし32-bit整数値 |
| `long` | 符号付き64-bit整数値 |
| `ulong` | 符号なし64-bit整数値 |
| `float` | 単精度floating-point値 |
| `double` | 倍精度floating-point値 |
| `string` | string値 |

この表のcanonical nameは、fieldがPrimitive Typeを宣言するときに使用するnameである。

### TYPE-PRIMITIVE-002

初期Primitive Type vocabularyは `byte`、`sbyte`、`short`、`ushort`、`decimal`、`char`、
`Guid`、`DateTime` をサポートしてはならない（MUST NOT）。将来のspecificationは、初期vocabularyの
意味を変更せずに、追加のPrimitive Typeを定義してもよい（MAY）。

### TYPE-PRIMITIVE-003

supported Primitive Typeを宣言するfieldのdata scalarは、宣言されたprimitiveのscalar categoryと
representable valueに一致しなければならない（MUST）。validationはprimitive type間でscalarを
implicit coerceしてはならない（MUST NOT）。特に、`int` に対する `1.0` は受け入れてはならない
（MUST NOT）。`uint` に対するnegative valueも受け入れてはならず（MUST NOT）、宣言されたinteger
range外のvalueも受け入れてはならない（MUST NOT）。

YAML parserのscalar classificationとこのvalidation ruleの正確な境界はOpen Questionのままである。
このruleは、type systemがparser classificationを黙って再解釈することを許可しない。

### TYPE-PRIMITIVE-004

初期vocabularyのinteger rangeは、C#/.NETのfixed-widthなsigned/unsigned domainに一致しなければ
ならない（MUST）。`int` は -2^31 から 2^31-1、`uint` は 0 から 2^32-1、`long` は -2^63 から
2^63-1、`ulong` は 0 から 2^64-1 である。これらのrange外のvalueはrejectしなければならない（MUST）。
validationはrange外のvalueをnarrow、wrap、saturate、またはimplicit convertしてはならない（MUST NOT）。

### TYPE-PRIMITIVE-005

PrimitiveをMasterMemoryのPrimary KeyまたはSecondary Keyとして直接利用できるか評価する場合、
`int`、`uint`、`long`、`ulong`、`string` はkey-compatibleに分類しなければならない（MUST）。
`bool`、`float`、`double` はkey-incompatibleに分類しなければならない（MUST）。

このrequirementはprimitive capabilityだけを定義する。indexの宣言とvalidationは将来のindex
specificationに属する。

### TYPE-PRIMITIVE-006

空文字列 `""` は有効な `string` valueでなければならない（MUST）。Primitive string validationは、
valueが空であることだけを理由にrejectしてはならない（MUST NOT）。fieldが `null` を受け入れるかは、
`string` primitiveではなくField Modifier ruleが所有する。

### TYPE-PRIMITIVE-007

`float` または `double` として宣言されたfieldのfinal valueはfiniteでなければならない（MUST）。
`NaN`、positive infinity、negative infinityは、いずれのprimitiveのvalueとしても受け入れてはならない
（MUST NOT）。これはtype-system ruleであり、parserがnon-finite valueを公開する場合にどのYAML scalar
syntaxを使うかは選択しない。

## 検証ルール

このproposalの観測可能なvalidation outcomeは、`TYPE-PRIMITIVE-001` から `TYPE-PRIMITIVE-007` に
よって定義する。対象は、unsupported name、scalar-category mismatch、fixed-width integer range
violation、finite floating-point value、invalid direct key capability、empty-string acceptanceである。
Exact diagnostic codeとsource-location mappingは、このproposalでは割り当てない。

## 受け入れ証拠

| Requirement（要件） | Success observation（成功時の観測） | Failure observation（失敗時の観測） | Suggested evidence（推奨する証拠） |
| --- | --- | --- | --- |
| `TYPE-PRIMITIVE-001` | 各initial canonical nameが宣言されたdomainへresolveする。 | 初期vocabulary外のnameが、これらのprimitiveの1つとして扱われない。 | Type vocabulary table test。 |
| `TYPE-PRIMITIVE-002` | future-only nameがinitial profileの外に保たれる。 | 列挙された各excluded nameがinitial primitiveとしてrejectされる。 | Unsupported-name validation test。 |
| `TYPE-PRIMITIVE-003` | 宣言されたcategoryとrepresentable valueを持つscalarが受け入れられる。 | `int` に対する `1.0`、negative `uint`、またはimplicit type conversionがrejectされる。 | Strict scalar validation tests。 |
| `TYPE-PRIMITIVE-004` | 各integer domainのboundary valueが受け入れられる。 | 各rangeの直外valueが、narrowing、wrapping、saturation、implicit conversionなしにrejectされる。 | Integer boundary tests。 |
| `TYPE-PRIMITIVE-005` | listedされた5つのkey-compatible primitiveがcompatibleに分類される。 | `bool`、`float`、`double` がincompatibleに分類される。 | Capability classification test。 |
| `TYPE-PRIMITIVE-006` | `""` がstring valueとして受け入れられる。 | stringが空であることだけを理由にfailureが報告されない。 | Empty-string validation test。 |
| `TYPE-PRIMITIVE-007` | scalar categoryとprecisionがvalidな代表的finite `float` / `double` valueが受け入れられる。 | `NaN`、positive infinity、negative infinityがいずれのfloating-point primitiveのfinal valueとしてもrejectされる。 | Finite/non-finite boundary validation tests。 |

## 互換性

このproposalはimplementationまたはreleased-schema migrationを追加しない。Primitive nameとscalar
representationは、実装後にgenerated C#とserialized dataへ影響するため、released-schema compatibility
はOpen Questionである。implementationがfull scalar compatibilityを主張する前に、正確なYAML numeric
grammarとparser behaviorを決定しなければならない。

## 例

次は、意図するvalidation boundaryのnon-normativeな例である。

```yaml
count: 1
ratio: 1.0
name: ""
```

`count: 1` は `int`、`ratio: 1.0` はfloating-point value、空の `name` はvalidなstring valueに
なり得る。どのvalueがvalidかはfieldのdeclared typeが決める。これらの例はYAML parser dialectを定義しない。

## Open Questions（未解決事項）

- parser library間で意見が分かれる場合、どのYAML scalar classificationをauthorityとするか。
- hexadecimal、octal、binary、exponent、およびその他のnon-decimal numeric formをintegerとfloating-point primitiveで受け入れるか。
- 選択したYAML parserが `NaN` またはinfinity tokenを公開する場合、type-systemのfinite-value check前にそのparser-level scalarをどう表現するか。
- timestamp-looking scalarを `string` またはnumeric primitiveとして宣言した場合、どのように扱うか。
- 将来、Primitive Type nameのcompatibility aliasを許可するか。
- 各scalar validation failureにどのdiagnostic codeとsource spanを割り当てるか。

## 非目標

このproposalは、Rust type registry、scalar parser、nullable/array validator、enum、flags enum、
custom type、index、reference、MessagePack generator、production binary builderを実装しない。
