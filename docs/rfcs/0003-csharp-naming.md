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

## 非目標（Non-Goals）

このRFCは、final naming convention、rename migration policy、Unicode identifier policyを承認しない。
Type System semanticsも追加しない。

## 選択肢（Options）

- invalid/reserved nameとnormalization collisionをrejectする。
- invalidまたはreserved nameをescapeまたはprefixする。
- explicit generated nameを必須にし、ambiguousなsource nameに対するnormalizationを避ける。

## トレードオフ（Trade-offs）

rejectionは予測可能でsource identityを保てるが、authorにinputのrenameを求める。escapingはより多くのinputを
保持できるが、generated API nameを変え、compatibilityを分かりにくくする。explicit nameの要求はuserにcontrolを
与えるが、schemaの記述量を増やし、namespace/file collisionは取り除かない。

## 提案（Proposal）

current scaffoldはconservativeなvalidation boundaryを持つ。namespace、type、property、constructor-parameter
nameはvalidなASCII C# identifierでなければならず、reserved keyword、normalized type/property/parameter collision、
case-insensitiveなgenerated filename collisionはerrorとする。これはimplementation guardであり、Approved naming
specificationではない。generated API nameがcompatibility promiseになる前に、最終的なproduct policyをrefineし、
reviewしなければならない。

## 互換性（Compatibility）

normalization、escaping、case sensitivity、file naming、Unicode handlingを変更すると、generated public APIと
filenameが変わる可能性がある。最終decisionにはgolden testと明示的なmigration policyが必要である。

## Open Questions（未解決事項）

- reserved keywordをrejectするか、`@` でescapeするか。
- source nameとnamespaceに対するcurrent ASCII-only policyを受け入れるか。
- どのUnicode normalizationとcase-folding ruleを適用するか。
- `foo-bar` と `foo_bar` のcollisionをどう解決するか。
- type、property、constructor-parameter、namespace、filenameのcollisionをすべてerrorとするか。
- `table` identityが変わらない場合、C# type renameはcompatibleか。
- どのgenerated APIとfilename formをstable compatibility surfaceとするか。

## 決定（Decision）

明示的なhuman approval待ち。currentのconservative validation boundaryは、このRFCがProposedである間、obviously
invalidなoutputを防ぐ可能性がある。
