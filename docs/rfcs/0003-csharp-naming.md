# RFC: 生成C#の命名方針（Generated C# naming policy）

Status: Proposed

## 背景（Context）

current C# generatorはscaffoldである。`table` からnameを導出し、任意の `csharpName` を受け取り、field nameを
C# propertyとconstructor parameterへ変換する。name normalizationによってcollisionが生じる可能性があり、
C#にはreserved keywordとnamespace ruleがある。これらはdomain semanticsではない。

## 課題（Problem）

invalidまたはcollisionしたgenerated nameがC# compile後までfailureにならないため、diagnosticが遅く、schemaとの
関連付けも難しい。completeなnaming policyでは、source field/table identityとgenerated presentation nameを区別
する必要もある。

## 目標（Goals）

- generated C# naming specificationが所有するobservable contractの背景とtrade-offを記録する。
- obvious invalid outputをcode-generation validation boundaryで止める。
- Unicode、escaping、rename compatibility policyを黙って選択しない。
- Value ObjectとCustom Typeの `implement-spec` 開始前に必要なpublic C# identifier contractを明示する。

## 非目標（Non-Goals）

このRFCは、generated C# naming contractのcanonical ownerではない。observable contractは
[C#命名仕様](../specs/type-system/csharp-naming.md)が所有する。rename migration policy、Unicode identifier policy、
namespaceおよびfilenameの詳細は、このRFCでも承認しない。
Type System semanticsも追加しない。

## 型システム実装前のhuman-confirmed contract

Value ObjectとCustom Typeの仕様は、generated public C# type、property、constructorを要求する。今回のhuman decisionにより、
これらのminimum public identifier surfaceに関する6つのquestionは解決された。採用されたobservable contractは
[C#命名仕様](../specs/type-system/csharp-naming.md)へ反映し、このRFCは背景、rationale、比較されたalternativeを記録する。
以下の旧Option A/B/Cはhistory上の比較材料であり、現在のcanonical contractは「決定」に記載した内容である。

### 1. Type declaration nameからgenerated type identifierへのmapping

**Decision**: YAML type declarationの `name` は、`TYPE-NAMING-002` が定義するtype-name ASCII C# identifierでなければならず、
generated C# type identifierはそのsource nameをそのまま使用する。追加normalizationは行わない。詳細は `TYPE-NAMING-001`、
`TYPE-NAMING-002` が所有する。

- **Option A**: YAML `name` が有効なC# identifierであることを要求し、そのidentifierをそのまま使用する。
- **Option B**: YAML `name` を決定的なnormalization ruleでC# identifierへ変換し、変換後のcollisionを検出する。
- **Option C**: YAML `name` をsource identityとして保持し、別のexplicit generated-name optionを必須または任意で提供する。

**Why it matters**: Value Object / Custom Typeのpublic type identifierと、そこから参照されるgenerated APIを実装者が独自に決めると、
compile結果と将来のcompatibility surfaceが変わる。

### 2. Field nameからgenerated public property identifierへのmapping

**Decision**: Custom Typeのsource field `name` はlowerCamelCase ASCII identifierでなければならない。generated public property
identifierは先頭ASCII characterだけをuppercaseへ変換し、残りをそのまま保持する。詳細は `TYPE-NAMING-003`、`TYPE-NAMING-004` が
所有する。

- **Option A**: YAML field `name` が有効なC# identifierであることを要求し、そのidentifierをpropertyにそのまま使用する。
- **Option B**: YAML field `name` を決定的にnormalizationし、property identifierへ変換する。変換後のcollisionはerrorとする。
- **Option C**: source field nameとは別に、generated property nameをfieldごとに明示する。

**Why it matters**: Custom Typeのget-only propertyの名前はpublic APIであり、実装者がcase変換やseparator処理を発明してはならない。

### 3. Field nameからconstructor parameter identifierへのmapping

**Decision**: Custom Typeのgenerated constructor parameter identifierは、対応するYAML field `name`をそのまま使用する。named
argumentから観測されるnameもstable public API surfaceである。parameter orderは `SCHEMA-CUSTOM-017` のYAML
`custom.fields` declaration order ruleに従う。詳細は `TYPE-NAMING-005` が所有する。

- **Option A**: property mappingから決定的に導出し、parameter mapping ruleもstable public APIとして扱う。
- **Option B**: parameter identifierをstable contractに含めず、compile可能でcollisionのないidentifierであることだけを要求する。
- **Option C**: property identifierとparameter identifierをfieldごとに独立指定する。

**Why it matters**: constructor parameter nameはproperty nameとは別のidentifier surfaceであり、named argumentを許す場合はpublic
source compatibilityに影響する。Option Bを選ぶ場合も、実装者がdomain semanticsを補完しないよう、stable contract外であることを
明記する必要がある。

### 4. Invalid C# identifierの扱い

**Decision**: lexical ruleを満たさないsource nameはschemaまたはcode-generation validation errorとしてrejectする。automatic
repairは行わない。詳細は `TYPE-NAMING-006` が所有する。

- **Option A**: schema/code-generation validation errorとしてrejectする。
- **Option B**: deterministicなescapeまたはprefix ruleでvalid identifierへ変換する。
- **Option C**: source nameを保持しつつ、explicit generated nameの指定を要求する。

**Why it matters**: invalid outputをcompile時まで遅延させず、実装者ごとに異なる修正を許さないためである。

### 5. C# reserved keywordの扱い

**Decision**: C# reserved keywordと一致するsource nameはvalidation errorとしてrejectする。C# `@` escapeは使用しない。詳細は
`TYPE-NAMING-006` が所有する。

- **Option A**: reserved keywordをinvalid inputとしてrejectする。
- **Option B**: C#の `@` escapeを使用してidentifierとして生成する。
- **Option C**: reserved keywordの場合だけexplicit generated nameを要求する。

**Why it matters**: keyword handlingを未定義のままにすると、同じschemaから生成するpublic APIがgeneratorの判断に依存する。

### 6. Normalization後のcollisionの扱い

**Decision**: 同じgenerated C# scopeでidentifierがcollisionする場合はvalidation errorとしてrejectする。automatic suffix、prefix、
escape、normalizationまたはその他のdisambiguationは行わない。generator-owned memberとのcollisionも含む。詳細は `TYPE-NAMING-007` が
所有する。

- **Option A**: schema/code-generation validation errorとしてrejectする。
- **Option B**: deterministicなsuffixまたはprefixを付けて別identifierへdisambiguateする。
- **Option C**: collisionしたfield/typeごとにexplicit generated nameを要求する。

**Why it matters**: `foo-bar` と `foo_bar` のような入力を同一APIへ黙ってmergeせず、生成結果を一意にする必要がある。

### Type-name lexical ruleのhuman-confirmed refinement

上記1のDecisionで使っていたstyle labelのうち、type nameのlexical boundaryは後続のhuman decisionによって明確化された。
canonical ruleは `^[A-Z][A-Za-z0-9]*$` であり、ASCII only、先頭のuppercase ASCII letter、および後続のASCII letterまたは
ASCII digitを要求する。`A`、`AB`、`ID`、`URL`、`HTTP`、`REWARD`、`ItemId`、`Reward`、`HTTPServer`、`XML2Data`、`A1`、
`Item2` はvalidである。ALL-CAPS acronymを含むstyle上の区別、word boundary、acronymの分割をvalidatorで追加要求しない。
このobservable contractは `docs/specs/type-system/csharp-naming.md` の `TYPE-NAMING-002` が所有する。

## 補助的な命名事項

generated namespaceのvalidity、generated filename、filesystemのcase collision、Unicode normalizationの高度な互換性、namespace/API
rename migrationは、将来のgenerated artifactとreleased compatibilityの検討事項である。これらは今回解決した6つのminimum
contractとは別であり、current implementationが要求する場合は、conservativeなvalidation guardを別途維持する。

## 選択肢（Options）

- invalid/reserved nameとnormalization collisionをrejectする。
- invalidまたはreserved nameをescapeまたはprefixする。
- explicit generated nameを必須にし、ambiguousなsource nameに対するnormalizationを避ける。

## トレードオフ（Trade-offs）

rejectionは予測可能でsource identityを保てるが、authorにinputのrenameを求める。escapingはより多くのinputを
保持できるが、generated API nameを変え、compatibilityを分かりにくくする。explicit nameの要求はuserにcontrolを
与えるが、schemaの記述量を増やし、namespace/file collisionは取り除かない。

## 提案（Proposal）

current scaffoldには、namespace、type、property、constructor-parameterの明らかに無効なgenerated outputを早期に止める
conservativeなvalidation boundaryがある。これはcurrent implementationのguardに関する観測であり、このRFCがcanonical
naming specificationの代替であることを意味しない。最終的なobservable contractは `docs/specs/type-system/csharp-naming.md` に
記録する。

## 互換性（Compatibility）

normalization、escaping、case sensitivity、file naming、Unicode handlingを変更すると、generated public APIと
filenameが変わる可能性がある。最終decisionにはgolden testと明示的なmigration policyが必要である。

## Open Questions（未解決事項）

- namespace、filename、Unicode normalization、case-folding、released API rename migrationにどのcompatibility guaranteeを与えるか。
- C# contextual keywordを将来どのように扱うか。

## 決定（Decision）

今回のhuman decisionは、6つのminimum naming questionについて、`docs/specs/type-system/csharp-naming.md` に記録されたpolicyを
示した。RFCはrationaleとalternative comparisonを保持し、`Status: Proposed` のままとする。RFCの `Accepted` transitionと
canonical specificationの `Approved` transitionは別のworkflow operationである。今回の明示的なhuman approvalによりcanonical
specificationは `Approved` となったが、このRFCの `Accepted` transitionは実行しない。RFCはrationaleとalternativeの記録であり、
canonical specificationの代替となるimplementation authorityではない。
