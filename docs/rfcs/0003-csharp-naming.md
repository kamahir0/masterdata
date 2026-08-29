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

- 将来のnaming specificationが回答すべきquestionを定義する。
- obvious invalid outputをcode-generation validation boundaryで止める。
- Unicode、escaping、rename compatibility policyを黙って選択しない。
- Value ObjectとCustom Typeの `implement-spec` 開始前に、人間が決めるべきpublic C# identifier contractを明示する。

## 非目標（Non-Goals）

このRFCは、final naming convention、rename migration policy、Unicode identifier policyを承認しない。
Type System semanticsも追加しない。

## 型システム実装前のblocking contract

Value ObjectとCustom Typeの仕様は、generated public C# type、property、constructorを要求する。しかし、YAML nameから
C# identifierへのmappingはこのRFCでまだ確定していない。したがって、以下の質問が人間によって解決されるまで、これらの
generated public APIに対する `implement-spec` は開始してはならない（MUST NOT）。このsectionのRecommendationはRFC上の
比較材料であり、final decisionではない。

### 1. Type declaration nameからgenerated type identifierへのmapping

**Question**: YAML type declarationの `name` をgenerated C# type identifierへどう変換するか。

- **Option A**: YAML `name` が有効なC# identifierであることを要求し、そのidentifierをそのまま使用する。
- **Option B**: YAML `name` を決定的なnormalization ruleでC# identifierへ変換し、変換後のcollisionを検出する。
- **Option C**: YAML `name` をsource identityとして保持し、別のexplicit generated-name optionを必須または任意で提供する。

**Recommendation**: 初期contractではOption Aを推奨する。暗黙のrenameを避け、generated public type identityを入力から追跡しやすく
できるためである。異なる入力表記を許可する必要がある場合は、Option BまたはCを人間が明示的に選ぶ。

**Why it matters**: Value Object / Custom Typeのpublic type identifierと、そこから参照されるgenerated APIを実装者が独自に決めると、
compile結果と将来のcompatibility surfaceが変わる。

### 2. Field nameからgenerated public property identifierへのmapping

**Question**: YAML field `name` をgenerated public property identifierへどう変換するか。

- **Option A**: YAML field `name` が有効なC# identifierであることを要求し、そのidentifierをpropertyにそのまま使用する。
- **Option B**: YAML field `name` を決定的にnormalizationし、property identifierへ変換する。変換後のcollisionはerrorとする。
- **Option C**: source field nameとは別に、generated property nameをfieldごとに明示する。

**Recommendation**: 初期contractではOption Aを推奨する。source field identityとpublic property identityの対応が明快になり、
normalization collisionを暗黙に解決せずに済むためである。人間向けYAML表記とC#表記を分ける要件がある場合は、Option BまたはCを選ぶ。

**Why it matters**: Custom Typeのget-only propertyの名前はpublic APIであり、実装者がcase変換やseparator処理を発明してはならない。

### 3. Field nameからconstructor parameter identifierへのmapping

**Question**: generated constructorのparameter identifierを、property identifierとどのように関係付けるか。

- **Option A**: property mappingから決定的に導出し、parameter mapping ruleもstable public APIとして扱う。
- **Option B**: parameter identifierをstable contractに含めず、compile可能でcollisionのないidentifierであることだけを要求する。
- **Option C**: property identifierとparameter identifierをfieldごとに独立指定する。

**Recommendation**: named argumentによるconstructor callをcompatibility surfaceに含めないならOption Bを推奨する。named argumentを
stable APIとして扱うならOption Aを選び、導出ruleを同時に決める。Option Cはschema記述量とmigration surfaceが増えるため、明示的な
要件がある場合に限る。

**Why it matters**: constructor parameter nameはproperty nameとは別のidentifier surfaceであり、named argumentを許す場合はpublic
source compatibilityに影響する。Option Bを選ぶ場合も、実装者がdomain semanticsを補完しないよう、stable contract外であることを
明記する必要がある。

### 4. Invalid C# identifierの扱い

**Question**: YAML nameからvalidなC# identifierを生成できない場合にどうするか。

- **Option A**: schema/code-generation validation errorとしてrejectする。
- **Option B**: deterministicなescapeまたはprefix ruleでvalid identifierへ変換する。
- **Option C**: source nameを保持しつつ、explicit generated nameの指定を要求する。

**Recommendation**: Option Aを推奨する。入力の誤りを早期に検出し、暗黙のpublic API renameを避けられるためである。Option BまたはCを
選ぶ場合は、変換結果とcompatibility ruleを同時に定義する。

**Why it matters**: invalid outputをcompile時まで遅延させず、実装者ごとに異なる修正を許さないためである。

### 5. C# reserved keywordの扱い

**Question**: `class` や `event` のようなreserved keywordに一致するsource nameをどう扱うか。

- **Option A**: reserved keywordをinvalid inputとしてrejectする。
- **Option B**: C#の `@` escapeを使用してidentifierとして生成する。
- **Option C**: reserved keywordの場合だけexplicit generated nameを要求する。

**Recommendation**: Option Aを推奨する。source nameとgenerated APIの対応を予測しやすく、language-specific escapeをdomain入力へ
持ち込まないためである。Option BまたはCを選ぶ場合は、type、property、parameterの各surfaceへ同じruleを適用するかを明示する。

**Why it matters**: keyword handlingを未定義のままにすると、同じschemaから生成するpublic APIがgeneratorの判断に依存する。

### 6. Normalization後のcollisionの扱い

**Question**: 異なるsource nameが同じgenerated identifierへ変換される場合にどうするか。

- **Option A**: schema/code-generation validation errorとしてrejectする。
- **Option B**: deterministicなsuffixまたはprefixを付けて別identifierへdisambiguateする。
- **Option C**: collisionしたfield/typeごとにexplicit generated nameを要求する。

**Recommendation**: Option Aを推奨する。source identityを黙って変更せず、schema authorが意図したrenameを明示できるためである。
Option BまたはCを選ぶ場合は、disambiguationの安定性とcompatibilityを同時に定義する。

**Why it matters**: `foo-bar` と `foo_bar` のような入力を同一APIへ黙ってmergeせず、生成結果を一意にする必要がある。

## 補助的な命名事項

generated namespaceのvalidity、generated filename、filesystemのcase collision、Unicode normalizationの高度な互換性、namespace/API
rename migrationは、将来のgenerated artifactとreleased compatibilityの検討事項である。これらは、このRFCの6つのminimum
blocking questionを解決するまでのType System semanticを直接決めるものではない。ただし、current implementationが要求する場合は、
conservativeなvalidation guardを別途維持する。

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
conservativeなvalidation boundaryがある。これはcurrent implementationのguardに関する観測であり、このRFCがそのままapproved
naming specificationであることを意味しない。generated API nameがcompatibility promiseになる前に、上記のminimum blocking
questionについて最終的なproduct policyをrefineし、reviewしなければならない。

## 互換性（Compatibility）

normalization、escaping、case sensitivity、file naming、Unicode handlingを変更すると、generated public APIと
filenameが変わる可能性がある。最終decisionにはgolden testと明示的なmigration policyが必要である。

## Open Questions（未解決事項）

上記のminimum blocking questionに対するhuman decisionが必要である。

- Type declaration `name` をgenerated C# type identifierへexactに写すか、normalizationするか、explicit generated nameを設けるか。
- Field `name` をgenerated public property identifierへexactに写すか、normalizationするか、explicit generated nameを設けるか。
- Constructor parameter identifierをstable public contractに含めるか、compile可能なnon-contract detailとして扱うか。
- invalid C# identifierをreject、escape/prefix、またはexplicit generated name要求のいずれで扱うか。
- reserved keywordをreject、`@` escape、またはexplicit generated name要求のいずれで扱うか。
- normalization collisionをreject、deterministic disambiguation、またはexplicit generated name要求のいずれで扱うか。
- namespace、filename、Unicode normalization、case-folding、released API rename migrationにどのcompatibility guaranteeを与えるか。

## 決定（Decision）

明示的なhuman decision待ち。RFCは `Status: Proposed` のままであり、上記のblocking questionをfinal naming contractとして
採用していない。currentのconservative validation boundaryはobviously invalidなoutputを防ぐための実装guardに過ぎず、
このRFCがProposedである間もgenerated public APIの最終命名規則を承認したものではない。
