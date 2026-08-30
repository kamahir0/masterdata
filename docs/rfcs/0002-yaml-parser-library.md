# RFC: YAML parser/libraryの選定

Status: Proposed

## 背景（Context）

YAMLはprojectのSource of Truthである。current implementationはtyped deserializationと
`serde_yaml::Value` intermediate treeに `serde_yaml = 0.9` を使用している。したがってparserの選定は、
diagnostic、将来のGUI editing、source fileの解釈に影響する。parserがvalueをloadできるだけでは、
round-trip editingに適するとは限らない。本RFCは調査結果を記録するものであり、dependency migrationを
許可するものではない。

Masterdataのnormative YAML syntax reference baselineはYAML 1.2.2である。ただし、実際に受理するsource languageと
unsupported constructは[Masterdata YAML subset仕様](../specs/yaml-subset.md)が定める。このRFCはYAML 1.2.2の全機能を採用するものでも、
parser libraryを選択またはmigrationするものでもない。

## 課題（Problem）

current parserはSerde mappingには便利だが、upstream repositoryがarchivedであり、`0.9.34` releaseには
crateがno longer maintainedと記載されている。将来のreplacementはmachine validationと、人間が編集した
内容を保つeditor requirementの両方で評価しなければならない。valueをloadするだけのparserがround-trip
editingに適するとは限らない。

## 目標（Goals）

- productの実際のYAML needに対して現実的なRust optionを比較する。
- 未対応のYAML behaviorをproduct Open Questionとして明示する。
- human-approved decision前の大規模parser migrationを避ける。

## 非目標（Non-Goals）

このRFCは、YAML subset、`serde_yaml` の変更、round-trip editorの実装、anchor、tag、複数documentに関する
schema/domain semanticsを決めない。

## 選択肢（Options）

### 現行の `serde_yaml 0.9`（Current）

StrongなSerde integrationと小さなmigration surfaceを持つ。upstream GitHub repositoryはarchivedであり、
latestの `0.9.34` releaseはno longer maintainedと表示される。value-oriented APIはlosslessなsyntax tree
ではないため、current modelではcomment、whitespace、quote style、exact formattingを保持できない。
error objectはparser markを公開するが、repositoryにはそれをcore Diagnosticへ一貫してmapする作業が残る。

### `yaml_serde`

YAML organizationが公開するactively maintained forkであり、`serde_yaml` からのminimal migrationを意図する。
documentationではpackage renameをoptionとして説明している。Serde-compatibleな有望なcandidateだが、duplicate
key、tag、scalar resolution、error locationについて、compatibility test corpusなしにbehavior equivalenceを
仮定しない。

### `yaml-rust2` / `saphyr`

Pure-RustのYAML 1.2 parser/document APIである。`yaml-rust2` はstable/basic-maintenance postureを記載し、
`saphyr` はより新しく、よりactiveに開発されるAPI familyである。document/event accessと複数documentのloadを
提供するが、current typed modelのdrop-in Serde replacementではない。conversion、duplicate-key policy、source
span、serialization behaviorを明示的にengineeringする必要がある。

### `yaml-edit`

comment、whitespace、formattingを保持しながらeditすることに重点を置くlossless syntax-tree editorである。
将来のGUI write-back pathには魅力的だが、current Serde modelのdrop-in replacementではない。lossless syntaxと
typed domain ASTの間に、意図的なbridgeが必要になる。

### `serde-saphyr`

Saphyr ecosystemを基盤とするSerde-oriented YAML deserializerである。documented feature setにはerror
reportingとcomment-aware wrapperが含まれるが、maturity、duplicate-key behavior、round-trip model、このrepository
のtyped ASTとのcompatibilityにはrepository固有の評価が必要である。

## 比較マトリクス（Comparison matrix）

このmatrixでは、candidate projectが公表しているcapabilityと、local testがまだ必要なrequirementを区別する。
`Unknown` は「このRFCで確立されていない」という意味であり、「productとして許可する」という意味ではない。

| 観点 | serde_yaml 0.9 | yaml_serde | yaml-rust2 / saphyr | yaml-edit | serde-saphyr |
| --- | --- | --- | --- | --- | --- |
| Maintenance status（保守状況） | upstreamはarchived/deprecated | 保守されたforkのcandidate | Pure-Rust family。yaml-rust2はstabilityを重視し、saphyrはより速く進化する | editorに特化したcandidate | Active candidate。local maturity checkが必要 |
| Serde integration（Serde統合） | Native | 最小migrationを想定 | drop-in native mappingなし | current modelではなし | Native Serde path |
| Error line/column（エラーの行/列） | Parser markは利用可能。local mappingが必要 | local compatibility testが必要 | event/parser spanにlocal mappingが必要 | position trackingをdocumented | error reportingをdocumented |
| Duplicate mapping keys（重複mapping key） | local behavior testが必要 | local behavior testが必要 | local behavior testが必要 | syntaxは保持し、semantic policyはlocal | local behavior testが必要 |
| Anchors / aliases（anchor / alias） | local behavior testが必要 | local behavior testが必要 | YAML document/eventをsupport。local semantic testが必要 | syntaxを保持でき、semantic policyはlocal | supportをdocumented。product policyはlocal |
| Merge keys（merge key） | local behavior testが必要 | local behavior testが必要 | product policyはlocal | syntaxを保持でき、semantic policyはlocal | supportをdocumented。product policyはlocal |
| Multiple `---` documents（複数document） | API behaviorのtestが必要 | API behaviorのtestが必要 | document loadingをdocumented | document model behaviorのtestが必要 | API behaviorのtestが必要 |
| Custom tags（custom tag） | local behavior testが必要 | local behavior testが必要 | event/document support。product policyはlocal | syntax preservationは可能。policyはlocal | support pathをdocumented。policyはlocal |
| Numeric interpretation（numericの解釈） | compatibility corpusが必要 | compatibility corpusが必要 | YAML 1.2-oriented。corpusが必要 | syntaxは保持し、conversionはlocal | compatibility corpusが必要 |
| Timestamp interpretation（timestampの解釈） | compatibility corpusが必要 | compatibility corpusが必要 | conversion policyはlocal | syntaxは保持し、conversionはlocal | compatibility corpusが必要 |
| Unknown fields（未知のfield） | Serde attribute/local policy | Serde attribute/local policy | conversion layer policy | preserve可能 | Serde attribute/local policy |
| Round-trip editing（round-trip編集） | current modelではlosslessでない | 未確立 | emitterであり、lossless editorではない | lossless editingがprimary goal | comment-aware valueだが、完全なformat guaranteeではない |
| Comment preservation（comment保持） | current typed modelでは対象外 | 未確立 | current typed modelのguaranteeではない | 明示的にsupport | comment wrapperをdocumented |
| Format/quote preservation（format/quote保持） | なし | 未確立 | emitterによるguaranteeなし | 明示的にsupport | 完全なformat preservationとしては未確立 |
| Performance（性能） | 既存baseline | baselineが必要 | candidate benchmarkが必要 | editor-oriented benchmarkが必要 | candidate benchmarkが必要 |
| Cross-platform（cross-platform対応） | Rust/native dependency behaviorのtestが必要 | testが必要 | Pure Rust | Pure Rust | Pure Rust ecosystem |
| Ecosystem maturity（ecosystemの成熟度） | 歴史的にはmatureだが、現在はunmaintained | newer fork | established lineageだがmaturityは異なる | より狭く新しいfocus | newer candidate |
| License（license） | MIT/Apache-2.0 lineage。package metadataを確認する | package metadataを確認する | MIT/Apache-2.0 family | package metadataを確認する | package metadataを確認する |

## トレードオフ（Trade-offs）

`serde_yaml` を維持すればimmediate migration riskを避け、current typed implementationを保てるが、maintenance
riskは残り、将来のlossless editingも解決しない。Serde-compatible forkはcode churnを減らすが、やはりcorpusが
必要である。YAML 1.2 document APIはsyntaxとdocumentを制御しやすくする一方、明示的なtyped conversion layerが
必要になる。lossless editorはGUI write-backに最も適するが、semantic parserを黙って置き換えるのではなく、補完
する構成が望ましい可能性が高い。

## 提案（Proposal）

human-approved decisionがなされるまで、current `serde_yaml 0.9` dependencyを変更しない。replacementを選ぶ前に、
上記matrixを網羅するcompatibility corpusを作り、architectureがsemantic parserとlossless syntax representationの
2層を必要とするか決める。candidateのうち最も便利なものを理由にproduct YAML subsetを推論しない。

## 互換性（Compatibility）

parser libraryの変更は、scalar type、duplicate-key acceptance、anchor/alias、tag、error location、serialization
outputを変える可能性がある。migrationにはfixtureとgolden-outputの比較、および明示的なcompatibility decisionが必要
である。このRFCによるmigrationは行わない。

## Open Questions（未解決事項）

- anchor、alias、merge key、複数document、custom tagを許可するか。
- duplicate mapping keyはerror、first-value rule、last-value rule、または後続diagnosticのために保持するものか。
- どのnumericとtimestamp formをtyped valueにするか。
- unknown fieldをreject、ignore、またはpreserveするか。
- GUI editでcomment、quote style、whitespace、orderingを保持しなければならないか。
- semantic/lossless representationの2層構成は正当化できるか。
- 選択するparser stackに対して、どのlicenseの組み合わせとmaintenance policyを受け入れるか。

## 決定（Decision）

YAMLのnormative syntax reference baselineはYAML 1.2.2とする。このdecisionは、YAML 1.2.2が許可するすべてのconstructを
Masterdataが受理すること、または特定のparser libraryを選択することを意味しない。実際のaccepted source languageとsubset restrictionは
[Masterdata YAML subset仕様](../specs/yaml-subset.md)が所有する。

RFC自体は引き続き明示的なhuman approval待ちであり、current recommendationはmigrationを延期し、dependencyを `serde_yaml 0.9` にpinした
まま、review済みcompatibility corpusを通じてparser stackとeditor requirementを解決することである。

## 参照（References）

- [`serde-yaml`](https://github.com/dtolnay/serde-yaml) と [`0.9.34` release](https://github.com/dtolnay/serde-yaml/releases/tag/0.9.34)
- [`yaml_serde`](https://github.com/yaml/yaml-serde)
- [`yaml-rust2`](https://github.com/Ethiraric/yaml-rust2) と [`saphyr`](https://github.com/saphyr-rs/saphyr)
- [`yaml-edit`](https://github.com/jelmer/yaml-edit)
- [`serde-saphyr`](https://github.com/bourumir-wyngs/serde-saphyr)
