# C#命名仕様（C# naming）

Status: Proposed

Domain: Type System

## 概要

本仕様は、Value ObjectおよびCustom Typeのtype declarationと、Custom Typeのsource field nameから
generated C# public identifierへ至る、決定的な命名contractを定義する。source nameを自動修正せず、
invalid name、reserved keyword、generated member collisionは早期にvalidation errorとする。

この仕様は、採用理由と比較されたalternativeを記録する[C# naming RFC](../../rfcs/0003-csharp-naming.md)と
分担する。observableなgenerated identifier behaviorのcanonical ownerはこの仕様である。

Tableの `table` identityおよび `csharpName` によるgenerated presentation nameは、Table identityと
build/codegenの適用可能な仕様が所有する。この仕様はTableの命名・identityを変更しない。

## 用語

`Type Declaration`、`Value Object`、`Custom Type`、`Field`、`Generated C#`、`identifier`は
[product terminology（用語）](../../product/terminology.md)に従う。

この仕様でいうASCII letterは `A` から `Z` または `a` から `z`、ASCII digitは `0` から `9` を指す。
`C# reserved keyword`は、generated sourceが対象とするC# language versionのreserved keywordを指す。
contextual keywordをこの仕様のreserved keywordへ暗黙に追加しない。

## 規範要件

### TYPE-NAMING-001

Value ObjectまたはCustom Typeのtop-level `name`は、この仕様のPascalCase ASCII C# identifierでなければ
ならず（MUST）、generated C# type identifierはYAML `name`をそのまま使用しなければならない（MUST）。
`name`を別のidentifierへnormalization、split、re-case、transliterationしてはならない（MUST NOT）。

### TYPE-NAMING-002

Type declarationのPascalCase nameは、次のlexical ruleを満たさなければならない（MUST）。

- nameは空であってはならず、ASCII letterまたはASCII digitだけで構成する。
- 先頭characterはuppercase ASCII letterでなければならない。
- nameが1 characterより長い場合、先頭以外にlowercase ASCII letterまたはASCII digitを少なくとも1つ含めなければならない。

このruleでは、`A`、`A1`、`Item2`、`HTTPServer`、`XML2Data` はvalidであり、`REWARD` はinvalidである。
word boundary、acronymの分割、culture-sensitive case mapping、Unicode normalizationは判定に使用しない。

### TYPE-NAMING-003

Custom Typeのsource field `name` は、この仕様のlowerCamelCase ASCII C# identifierでなければならない
（MUST）。lowerCamelCase nameは、空でなく、先頭がlowercase ASCII letterであり、残りがASCII letterまたは
ASCII digitだけで構成されなければならない（MUST）。この仕様はfield nameをsplit、re-case、transliteration
またはUnicode normalizationしてはならない（MUST NOT）。

このruleでは、`x`、`item2`、`httpServer`、`xml2Data` はvalidであり、`ItemId`、`ITEM_ID`、`item-id` は
invalidである。Table source fieldにこのruleを適用するかどうかは、Tableのowner specificationが定義する。

### TYPE-NAMING-004

Custom Type fieldのgenerated public property identifierは、source field nameの先頭ASCII lowercase letter
だけを対応するuppercase ASCII letterへ変換し、それ以外のcharacter sequenceをそのまま保持しなければならない
（MUST）。generatorはseparatorの除去、word split、全体のre-case、suffixまたはprefix付与をしてはならない
（MUST NOT）。したがって、`fooBar` は `FooBar` となり、`Foobar` へ変換してはならない。

Value Objectの固定property `Value` はgenerator-owned memberであり、source field nameからこのruleで導出する
propertyではない。

### TYPE-NAMING-005

Custom Typeのgenerated public constructor parameter identifierは、対応するYAML field `name`をそのまま使用
しなければならない（MUST）。named argumentから観測されるparameter nameもこのexact source nameに従う。parameter
のorderは[Custom Type仕様](custom-types.md)のfield ID ascending ruleが所有し、この仕様はorderを変更しない。

### TYPE-NAMING-006

Value ObjectまたはCustom Typeのtype name、またはCustom Typeのfield nameが、この仕様のlexical ruleを満たさない、
ASCII-only scope外である、またはsource identifierとしてC# reserved keywordと一致する場合、schemaまたはcode-generation
validation errorとしなければならない（MUST）。generatorは `@` escape、underscore、prefix、suffix、transliteration、
normalizationまたはその他のautomatic repairを行ってはならない（MUST NOT）。source schemaはvalidなnameへ明示的に修正
しなければならない。

### TYPE-NAMING-007

同じgenerated C# declarationまたはoutput scopeで、複数のgenerated type、property、constructor parameter、または
generator-owned public memberが同じidentifierになる場合、validation errorとしなければならない（MUST）。source field
から導出されるproperty同士のcollisionも同じ扱いとする。collisionをsuffix、prefix、escape、normalizationまたは
その他の自動disambiguationで解決してはならない（MUST NOT）。

現在のValue Objectでreservedとなるgenerator-owned public member nameは `Value`、`Equals`、`GetHashCode`、
`ToString` である。現在のCustom Typeでreservedとなるvalue-type public member nameは `Equals`、`GetHashCode`、
`ToString` である。したがって、Custom Type field `equals`、`getHashCode`、`toString` は、それぞれ `Equals`、
`GetHashCode`、`ToString` とcollisionするためinvalidである。`Value` はValue Objectの固定propertyとのcollisionとして
扱い、Custom Typeに同名のgenerator-owned memberがない限り、この仕様だけでCustom Type fieldを禁止するものではない。

### TYPE-NAMING-008

この仕様のtype declaration naming ruleは、Value ObjectおよびCustom Typeに適用しなければならない（MUST）。この仕様を
根拠としてTableの `table` identityまたは `csharpName` presentation nameを変更してはならない（MUST NOT）。Enumおよび
Flags Enumのtype nameにこのruleを再利用する場合は、Enum member namingをこの仕様から暗黙に確定してはならない（MUST NOT）。

## 検証ルール

観測可能なvalidation outcomeは、`TYPE-NAMING-001` から `TYPE-NAMING-008` によって定義する。検証対象は、ASCII lexical
rule、type nameのidentity-preserving mapping、Custom Type fieldのproperty/constructor parameter mapping、reserved keyword、
automatic repairの禁止、generator-owned member collision、scopeの分離である。exact diagnostic code、source span、
generated filename、namespace derivationはこの仕様では定義しない。

同じC# declarationまたはoutput scopeというscopeは、生成対象が同じC# identifier namespaceまたはmember declarationへ
emissionされる範囲を指す。namespaceの導出方法自体はこの仕様の対象外だが、同一scope内のcollisionを黙って許可または修復
してはならない。

## 受け入れ証拠

| Requirement（要件） | Success observation（成功時の観測） | Failure observation（失敗時の観測） | Suggested evidence（推奨する証拠） |
| --- | --- | --- | --- |
| `TYPE-NAMING-001` | `name: Reward` が生成type identifier `Reward` になる。 | source nameが別のidentifierへ変換される、またはValue Object/Custom Typeのtype nameがTable namingから推測される。 | Generated type-name API test。 |
| `TYPE-NAMING-002` | `A`、`A1`、`Item2`、`HTTPServer`、`XML2Data` がvalidとして扱われる。 | `itemId`、`reward`、`reward_condition`、`reward-condition`、`REWARD`、非ASCII nameがrejectされる。 | PascalCase lexical validation tests。 |
| `TYPE-NAMING-003` | `x`、`item2`、`httpServer`、`xml2Data` がCustom Type field nameとして受け入れられる。 | `ItemId`、`ITEM_ID`、`item-id`、非ASCII field nameがrejectされる。 | lowerCamelCase lexical validation tests。 |
| `TYPE-NAMING-004` | `id -> Id`、`itemId -> ItemId`、`fooBar -> FooBar` が生成される。 | `fooBar -> Foobar`、separator除去、全体re-case、または自動suffixが行われる。 | Property-name mapping test。 |
| `TYPE-NAMING-005` | source field `itemId` のconstructor parameterが `itemId` となり、field ID順とは独立してexact nameが保持される。 | parameter nameがproperty nameへ変換される、またはgeneratorごとに変動する。 | Constructor reflection/named-argument API test。 |
| `TYPE-NAMING-006` | valid source identifierがそのまま受け入れられる。 | `class`、`struct`、`event`、`namespace`、`public` などのreserved keyword、invalid lexical name、または `@class` がrejectされる。 | Invalid-name and reserved-keyword validation tests。 |
| `TYPE-NAMING-007` | collisionのないgenerated APIが出力される。 | `equals -> Equals`、`getHashCode -> GetHashCode`、`toString -> ToString`、または同じscopeのduplicate identifierがrejectされる。 | Generated member collision tests。 |
| `TYPE-NAMING-008` | Value Object / Custom Typeのnaming ruleが適用され、Tableの `table` / `csharpName` の責務が変更されない。 | Type declaration ruleを根拠にTable identityまたはEnum member namingが暗黙に変更される。 | Cross-spec ownership review。 |

## 互換性

この仕様はgenerated C# type、property、constructor parameterのpublic API surfaceを固定するため、将来のgenerated artifact
とsource compatibilityに影響する。invalid nameを自動修正せずrejectするため、既存のinvalid inputを受け入れるmigrationは
定義しない。released APIのrename、namespace変更、filename変更、Unicode supportのcompatibility classificationはOpen Question
であり、この仕様のstatusを超えて解決してはならない。

Tableの `table` identityおよび `csharpName` presentation nameは別のowner specificationに属し、この仕様のtype declaration
naming contractと混在しない。

## 例

### Type declaration

```yaml
kind: type
name: RewardCondition
custom:
  fields:
    - id: 5
      name: note
      type: string
    - id: 1
      name: itemId
      type: ItemId
    - id: 3
      name: amount
      type: int
```

生成されるidentifierは、typeが `RewardCondition`、propertyが `Note`、`ItemId`、`Amount`、constructor parameterが
`note`、`itemId`、`amount` である。constructor parameterのorderはCustom Type仕様に従い、field ID ascendingで
`itemId`、`amount`、`note` となる。

### Collision

```yaml
kind: type
name: Reward
custom:
  fields:
    - id: 0
      name: equals
      type: string
```

`equals` は `Equals` へmappingされ、Custom Typeのgenerated equality memberとcollisionするためinvalidである。

## Open Questions（未解決事項）

以下は今回のtype-system naming contract外の事項であり、現行contractのapprovalまたは実装を黙って変更せず、必要になった時点で
別のspecification changeとして扱う。

- generated namespaceの導出と、複数typeのoutput scopeをどの単位で分けるか。
- generated filenameとfilesystemのcase collisionをどう扱うか。
- Unicode identifier、Unicode normalization、culture-sensitive case mappingを将来サポートするか。
- released generated APIのrename、namespace変更、またはsource name変更をcompatibility上どう分類するか。
- 将来、explicit generated-name overrideまたはescapingを追加するか。
- C# contextual keywordを将来reject、許可、または別のruleで扱うか。

## 非目標

この仕様は、Tableのidentityまたはnaming、Enum member naming、Flags Enum member naming、namespace migration、filename policy、
Unicode support、released API migration、C# parser、Rust parser、validator、C# code generator、またはfeature implementationを
定義・実装しない。
