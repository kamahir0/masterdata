# 仕様（Specifications）

Specificationはversion-controlledなproduct/domain contractである。人間がreviewでき、
testで検証できる言葉で、観測可能なbehaviorとcompatibility ruleを記述する。仕様が
Draftの間はcodeが一部だけを実装していてもよいが、current `Approved` または
`Implemented` specificationと黙って矛盾してはならない（MUST NOT）。

このdirectoryはproductおよびdomain specificationのcanonicalな場所である。conversationを
proposed changeへ変換するrepository workflowは、[仕様ワークフロー（Specification workflow）](../contributing/specification-workflow.md) に記載する。

## Normativeとnon-normativeの内容

Normative contentは、implementationが公開する必要のある、または公開を許可されたbehaviorを定義する。主に
`Normative Requirements`、`Validation Rules`、明示的に
compatibility ruleと記した箇所に置く。各normative requirementはstable IDを持つ。

Non-normative contentには、summary、rationale、examples、implementation notes、references、
design discussionを含む。人がspecificationを理解する助けにはなるが、behaviorを追加しない。
exampleだけをproduct ruleの唯一の記述場所にしてはならない（MUST NOT）。

Specificationはmeeting minutesではない。conversation historyはchangeまたはreview contextへ、
decision rationaleは重要な場合にADRへ置き、specificationにはそこから得られたbehaviorだけを記載する。

## Statusのlifecycle

`Status:` headerは、次の値のいずれか1つだけを使用する。

- `Draft`: まだ整理中の状態。不明点を含んでもよく、implementation authorityではない。
- `Proposed`: implementation candidateとしてreviewできる程度に整理済みだが、人間のmaintainerによる最終承認は受けていない。
- `Approved`: current normative contract。新しいimplementation workは原則としてこのstatusをSource of Truthにするべきである（SHOULD）。
- `Implemented`: acceptance criteriaに対応するimplementation、tests、適切なfixture evidenceを備えた `Approved` contract。適用してもrequirementの意味は変わらない。
- `Deprecated`: historyまたはcompatibility contextのために残す旧contract。新しいimplementationの対象ではない。replacementまたは理由をlinkするべきである（SHOULD）。

通常の進行は `Draft` -> `Proposed` -> `Approved` -> `Implemented` である。人間のmaintainerは
specificationを `Deprecated` へ移してもよい。`Approved` へ移せるのは、明示的なhuman approval
（review済みrepository changeまたは同等のmaintainer operationなど）がある場合だけである。
AI agentはこのtransitionを自動で行ってはならない（MUST NOT）。「Approved as Proposed」という
review recommendationはhuman reviewerへのevidenceであり、approval operationそのものではない。

ApprovedまたはImplemented documentへのsemantic changeは、新しいproposed change artifactとする。
そのartifactがreview中はcanonical documentを変更せず、引き続きauthorityとする。明示的なhuman
approval後、承認済みchangeをcanonical documentへatomicにmergeする。変更されないRequirement IDは
stableに保ち、置換または分割されたrequirementには新しいIDを付け、predecessorへのreferenceを
残す。implementationが不完全な場合、canonical documentは `Approved` のままとし、contractを
codeに合わせて変更せずgapを報告する。

## 承認の粒度

`Status:` はcanonical specification file全体に適用される。したがってcanonical fileには、
lifecycleを一緒に進められるrequirementを含めるべきである（SHOULD）。広いtopicが共通するという
理由だけで、すでに承認済みのcontractと無関係なDraft requirementを同じfileへ置いてはならない。

1つの広いdocument内でrequirementのmaturityが分かれた場合、statusを変更する前に小さなcanonical
fileへ分割する。分割はorganization上の変更に限る。既存Requirement IDは変更せず（MUST）、再割り当ても
してはならない（MUST NOT）。関連するspec familyのnon-canonical indexとしてdirectoryの
`README.md`を使用する。

requirementごとのstatus metadataよりも、このfile-level granularityを優先する。`Status: Approved`
または `Status: Implemented` がfile内のすべてのnormative requirementに対して曖昧さなく同じ意味を
持つことが目的である。

## Normative language（規範語）

normative requirementでは、次の語を文字どおり一貫して使用する。

- `MUST` / `MUST NOT`: 必須または禁止されるbehavior。
- `SHOULD` / `SHOULD NOT`: 例外を設ける理由を文書化した強いdefault。
- `MAY`: 許可されるbehaviorまたはcapability。behaviorの利用をrecommendするものではない。

wordingはevidenceの強度を保つ必要がある。「置くことができる」または「可能である
べき」は、それだけで全callerに `SHOULD` または `MUST` を課す根拠にはならない。Question、Idea、
Preference、Proposalは、別の明示的decisionが解決するまでnormativeではない。source conversationが
defaultやedge caseを確定していない場合は、`Open Question` に残す。

## Requirement IDの規約

Requirement IDは、uppercase ASCII segmentをhyphenで区切り、末尾に3桁のnumberを置く。
形式は `<DOMAIN>-NNN` または `<DOMAIN>-<TOPIC>-NNN` である（例: `PROJECT-001`、
`SCHEMA-VO-001`、`INDEX-PRIMARY-001`、`REF-001`）。GUI requirementは `GUI-` domain prefixを
持つ（例: `GUI-TABLE-EDIT-001`）。domainはcanonical owner areaを示すべきである。

IDを追加する前に、既存のすべてのspecification definitionを検索して割り当てる。一度公開したIDは、
rename、reassign、削除後の再利用をしてはならない（MUST NOT）。意味を変更する場合は新しいIDを
割り当て、predecessor/deprecation noteを付ける。同じnormative ruleは1つのcanonical specificationに
置き、他のdocumentからはそのIDへlinkし、内容をcopyしない。軽量な `cargo xtask check-specs`
commandは、明示的なrequirement definitionとreference、duplicate definition、malformed ID、
status/header metadata、duplicate ADR number、RFC/proposal numberingとmetadata、broken relative linkを
検査する。`See PROJECT-001` のようなrequirement referenceはownerではない。

## Requirement IDとDiagnostic

Requirement IDはnormative ruleを識別し、specification namespaceから割り当てる（例:
`PROJECT-004`、`SCHEMA-VO-001`）。runtime Diagnostic Codeは観測されたfailureを識別し、
`E-PROJECT-NOT-FOUND` や `E-YAML-PARSE` のように明確に分離されたnamespaceを使用する。
`E-` prefixはDiagnostic専用であり、Requirement IDは `E-` で始めてはならない（MUST NOT）。
Diagnosticはreferenceとして `related_requirements` metadataを持ってもよいが、Diagnostic Codeは
requirement definitionではなく、definitionとして使用してはならない。

## Open Questions（未解決事項）

`Open Questions` sectionには、未解決の選択、曖昧さ、acceptance detailの不足を記録する。これは
non-normativeである。答えによってbehaviorが変わるQuestionは、approvalをblockするか、current
proposalの対象外として明示しなければならない（MUST）。agentはimplementation convenienceのために
これを黙って解決してはならない。回答が得られたら、適切なproposed specification changeへdecision
を記録し、必要ならADRにも残す。

## 互換性とtraceability

stable identity、serialized shape、generated API、file interpretation、その他のpublic behaviorを
変更するたびにbackward compatibilityを検討する。互換、migrationが必要、意図的なbreaking、または
「not applicable」のいずれかをspecificationに明記する。

Testsとfixturesはrequirementのevidenceであり、requirementの代替ではない。1対1の対応が有用な場合は、
test nameまたは近接するcommentにRequirement IDを含める。例えば次のようにする。

```rust
#[test]
fn schema_vo_001_rejects_invalid_underlying_type() {
    // Covers SCHEMA-VO-001.
}
```

stableなend-to-end inputによってruleが明確になる場合は `fixtures/minimal`、`fixtures/full`、
`fixtures/invalid` にcaseを追加する。fixtureが不要なruleではfocused unitまたはintegration testで
十分である。fixture fileは固定されたtest inputであり、CLIまたはGUI executionで書き換えてはならない
（MUST NOT）。

## Specification change procedure（仕様変更手順）

1. conversationまたはrequestからintentを抽出し、各statementを分類する。
2. affected specification、ADR、RFC、および [用語（terminology）](../product/terminology.md) を読む。
3. 新しいcontractでは、stable ID、明示的なOpen Questions、compatibility impact、test impactを備えた
   `Draft` / `Proposed` specificationを更新または作成する。requirementsが同じlifecycle statusを
   共有できるcanonical fileを選ぶ。approval前に広いtopicを分割する必要がある場合は分割する。
   `Approved` / `Implemented` canonical documentへのsemantic changeでは、
   [`docs/spec-changes`](../spec-changes/README.md) 配下に別artifactを作成する（alternative比較中ならRFC）。
   canonical documentへ未承認semanticsを入れてはならない。
4. Draftまたはchange artifactに対して `review-spec` を実行する。blocking issueを解消するか、
   human reviewerが受け入れる理由を記録する。
5. human maintainerがrepositoryのreview operationで明示的に承認する。artifactは `Approved` となり、
   その後初めてdeltaをcanonical specificationへatomicに適用する。適用後、artifactを `Applied` とする。
   `Implemented` specificationへsemantic deltaを適用した場合は、新しいevidenceが揃うまでcanonical
   documentを `Approved` に戻す。
6. `implement-spec` でcanonicalなapproved behaviorだけを実装し、testsとfixturesを同期し、repository
   verificationを実行する。`Applied` でないchange artifactはimplementation inputにしてはならない。
7. evidenceが揃った後にのみcanonical statusを `Implemented` に変更する。

typo fix、formatting-only edit、public/domain semantic changeを伴わないinternal refactorは通常の
code/document workflowで扱ってよい。semantic boundaryが不明な場合はspecification workflowを使う。

## RFC・specification・ADRの役割

- **RFC**: adoptionとalternativeの検討中に使う、比較的大きなdesign proposal。implementation authorityではない。
- **Specification**: 採用されたproduct/domain behavior。normative requirementとcompatibility contractを含む。
- **ADR**: 重要なarchitectural choiceを選んだ理由を記録する。affected requirementへpointするべきだが、
  semanticsの2つ目のcopyになってはならない。

通常の関係はRFC -> approved specificationであり、rationaleまたはtrade-offを残す必要がある場合にADRを追加する。

## 文書一覧（Documents）

- [Projectの構成と探索仕様](project-layout.md)
- [Schema言語仕様](schema-language.md)
- [Masterdata YAML subset仕様](yaml-subset.md) — `Status: Approved`
- [Type System仕様](type-system/README.md)
  - [Primitive Types仕様](type-system/primitives.md)
  - [Field Modifiers仕様](type-system/field-modifiers.md)
  - [Value Objects仕様](type-system/value-objects.md)
  - [C#命名仕様](type-system/csharp-naming.md)
  - [Enum / Flags仕様](type-system/enums.md) — `Status: Approved`
  - [Custom Types仕様](type-system/custom-types.md)
- [Table / Primary Key / Secondary Key仕様](table-and-keys.md) — `Status: Approved`
- [Index / reference仕様](index-and-reference.md) — `Status: Draft`（Reference中心。Table/Keyのsemantic ownerは別document）
- [Build Selection仕様](build-selection.md) — `Status: Approved`
- [Build pipeline仕様](build-pipeline.md)
- [Compatibility仕様](compatibility/README.md)
  - [Table identity仕様](compatibility/table-identity.md)
  - [Field identity仕様](compatibility/field-identity.md) — `Status: Deprecated`（旧Field ID modelのhistory）
  - [Enum identity仕様](compatibility/enum-identity.md)
  - [Index identity仕様](compatibility/index-identity.md)

現在のRust implementationは、project contract、YAML document envelope、旧scaffoldのfield shape、source-content hash、明確に命名された
schema source-content hashを含むbuild-plan formationを扱うが、Applied後のMessagePack `key` modelを実装していない。
`id` という名前のfieldにはimplicit primary-key meaningがない。今回Approvedとなったtype-system contractはimplementation
authorityだが、current parser、validator、generatorはそれをまだ実装していない。Enum/FlagsとTable / Primary Key / Secondary Keyは
Approvedであり、specification change 0003もAppliedだが、type resolution、indexes、references、MasterMemory binary generation、full GUIは
意図的に未完了である。
