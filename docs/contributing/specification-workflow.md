# 仕様ワークフロー

Workflow status: Active

この文書は、設計会話をレビュー可能で実装可能なbehaviorへ変換するための、
repositoryのプロセスを定義する。会議議事録ではないため、product
specificationのlifecycleとは異なり `Workflow status` を使用する。productと
domainのsemanticsには、引き続き実装前に `Approved` specificationが必要となる。

## 正規の経路（canonical path）

```text
conversation or request
        |
        v
extract intent and evidence
        |
        v
read existing specs, ADRs, RFCs, and terminology
        |
        v
classify statements and preserve their strength
        |
        v
Draft/Proposed specification change
        |
        v
review-spec
        |
        v
explicit human approval
        |
        v
Approved Specification
        |
        v
implement-spec -> tests / implementation / fixtures -> verification
        |
        v
Implemented (only when evidence is complete)
```

Conversationはrefinementのevidenceであり、恒久的なspecificationではない。AI agentは、
明示的なevidenceなしにProposal、Preference、Question、または記憶していたcontextを
Approved behaviorへ昇格させてはならない（MUST NOT）。長大なconversation logを
specificationへ入れてはならない。重要なdecision rationaleだけを、必要に応じて
ADRへ残す。

## 発言の分類（Statement classification）

`refine-spec` はnormative textを書く前に、関連する各statementを分類する。1つの
conversationに複数の分類が含まれていてよい。

| Class（分類） | Meaning（意味） | Normative by itself?（それ自体が規範か） |
| --- | --- | --- |
| `Decision` | 発言者が「これにする」と確定した明示的な選択。 | 仕様案の根拠にはなるが、reviewと人間による承認はなお必要。 |
| `Requirement` | productに提供させたいcapabilityまたはoutcome。 | 固定された強度は持たない。発言の表現とcontextからMUST/SHOULD/MAYを導く。 |
| `Constraint` | 境界、禁止事項、または条件。「これは望まない」など。 | 明確なconstraintは、提案内のMUST NOTまたは別の明示的ruleになり得る。 |
| `Preference` | 強制的なcommitmentを伴わない、好み・優先順位・選択。 | そのままではnormativeではない。明示的に昇格されない限りnon-normativeに保つ。 |
| `Proposal` | 検討対象として示されたcandidate solution。 | そのままではnormativeではない。比較するか、未決定の選択として記録する。 |
| `Idea` | 探索的な可能性またはbrainstormingの発想。 | そのままではnormativeではない。requirementとして実装してはならない。 |
| `Question` | 情報またはclarificationを求める発言。 | そのままではnormativeではない。evidenceがなければ追跡対象にする。 |
| `Open Question` | follow-upのために残す未解決のdecisionまたは曖昧さ。 | そのままではnormativeではない。答えがbehaviorを変える場合はapprovalをblockする。 |
| `Rejected` | 明示的に退けられたoptionまたはbehavior。 | 暗黙に再導入してはならないというconstraintになる。 |

「これでもよいかもしれない」「こちらの方がよさそう」「Xはどうか？」という表現は、
通常は `Preference`、`Proposal`、または `Question` であり、`Decision` ではない。
「Xにする」「Yは望まない」「Xは必須である」は、scopeが明確なら `Decision` または
`Constraint` のevidenceになり得る。

## 意図の強度を保つ

Normative wordは互換ではない。

- `MUST` / `MUST NOT` は、明示的なrequirementまたはconstraintに限って使用する。
- `SHOULD` / `SHOULD NOT` は、例外を設ける理由が文書化された強いrecommendationを
  表す。単なるcapabilityや可能性には使わない。
- `MAY` はpermissionまたはcapabilityを表し、behaviorの利用をrecommendするものではない。

例えば「すべてのtable fileを同じdirectoryに置けるようにしたい」は、data-file locationが
table identityを決めてはならない（MUST NOT）、また複数のtable data fileが1つのdirectoryに
共存してもよい（MAY）というfaithfulなnormalizationを根拠づける。しかし、明示的な
recommendationがない限り、「すべてのtable data fileをそのdirectoryに置くべきである」
（SHOULD）とはできない。

strength、default behavior、error severity、ordering、nullability、edge caseが未指定なら、
`Open Questions` に曖昧さを残す。implementation convenienceのためにdefaultを発明しては
ならない。

## 役割と境界

### refine-spec

`refine-spec` は「何をspecificationにすべきか」を整理する。conversationとrepository
contextを読み、canonicalな対象specを特定し、statementを分類し、conflictを検出し、
stable IDを割り当て、Draft/Proposed changeを作成する。product behaviorを実装せず、
specificationを `Approved` に自動変更しない。

canonical documentがすでに `Approved` または `Implemented` の場合、semantic deltaは
`docs/spec-changes/` の別artifactへ記録しなければならない（MUST）。alternativeをまだ
比較中ならRFCを使用する。canonical documentへ未承認behaviorを混在させてはならない（MUST NOT）。
Requirement IDとruntime diagnostic codeは別namespaceに保ち、source evidenceなしに
current code behaviorをrequirementへ昇格させてはならない。RFCまたはADRをrecommendしても
よいが、product featureは実装せず、`Approved` を自動設定してはならない。

### review-spec

`review-spec` は独立したchallenge passである。source requestがある場合はそれと提案文を
比較し、関連specとADRを確認し、normative strengthとtestabilityを検査する。blocking
issues、non-blocking issues、questions、および人間によるapprovalに適しているかを報告する。
Approved as Proposed はrecommendationに過ぎず、specification statusを変更しない。

さらに、既存implementationに未承認semanticsが含まれていないか、Requirement IDとtest nameが
実際に対応しているか、unapproved changeがApproved canonical documentへ混入していないかを
確認する。

### implement-spec

`implement-spec` は明示的にApprovedとなったspecificationから開始する。Requirement IDを
acceptance criteria、tests、fixtures、affected crate、GUI boundary、.NET adapterへ対応付け、
Approved behaviorを実装して検証する。Diagnostic codeはRequirement IDとは別namespaceとして
扱い、acceptance/test traceabilityを確認する。誤ったcurrent behaviorをauthorityとして
扱わない。

`docs/spec-changes/` proposal、Draft、Proposed documentはimplementation inputにしてはならない。
conversationから不足するsemanticsを設計してはならない。不足するruleは `Specification Gap`
として報告し、refinementへ戻す。

## 承認とlifecycle

lifecycleとstatusの正確な意味は、[仕様index（specification index）](../specs/README.md)で定義する。特に、

- `Draft` と `Proposed` はimplementation authorityではない。
- `Approved` へ進めることができるのは、人間のmaintainerによる明示的なapprovalだけである。
- AI-generated draftを自動昇格させない。
- `Implemented` は、Approvedの意味が実装されevidenceで裏付けられた状態であり、仕様を
  弱めたり再解釈したりする許可ではない。
- specification-change artifactは `Draft` -> `Proposed` -> `Approved` -> `Applied` または
  `Rejected` を使用する。`Applied` はatomicなcanonical mergeを記録するもので、
  implementation statusではない。
- semantic changeは新しいproposed change artifactから始める。typo、formatting-only change、
  public/domain semantic changeを伴わないinternal refactorは通常のworkflowで扱ってよい。

## Approved specificationの変更

Approved canonical documentへ未承認semanticsを混ぜてはならない。refinementでsemantic changeが
見つかった場合は、影響するRequirement IDをlinkしたdurable artifactを
[`docs/spec-changes`](../spec-changes/README.md) 配下に作成し、canonicalの `Approved` /
`Implemented` documentは変更しない。artifactに対して `review-spec` を実行する。

明示的な人間の承認後、承認済みdeltaをcanonical documentへatomicに適用し、proposalは
`Status: Applied` のaudit recordとして残す。この時点で初めてimplementationを開始できる。
proposal自体はimplementation contractではない。canonical documentが `Implemented` だった場合、
semantic deltaの適用後は新しいacceptance evidenceが揃うまで `Approved` に戻す。これにより、
1つの `Approved` documentが、旧requirementはApprovedで新ruleは未承認という状態を同時に
主張することを防ぐ。

## 正規化とownership（Normalization and ownership）

requirementを追加する前に、既存の全IDと関連wordingを検索し、ownerのspecを読む。1つの
semantic ruleには1つのcanonical homeを置く。関連documentは2つ目のnormative versionを
copyするのではなく、そのrequirementへlinkするか関係を要約する。conflictはreviewerへ
提示し、黙って上書きしてはならない。

[product terminology（用語）](../product/terminology.md) をvocabularyの基準として使用する。termが
複数の意味を持つ場合は、Open Questionとして曖昧さを記録するか、terminology documentを
refineしてから使用する。

## Testsとfixtures

すべてのApproved behaviorには、規模に応じたverification planを用意する。traceabilityが
明確になる場合は、test nameまたは近接するcommentにRequirement IDを含める。安定した
end-to-end caseにfixtureが有用なら `fixtures/minimal`、`fixtures/full`、`fixtures/invalid`
を使用する。小さなruleにfixtureを必須とせず、focused unit testで十分な場合はそれを使う。
fixtureは固定入力として保持し、repository toolingによって `target/dev-project` にcopyする。

`implement-spec` は関連testを実行し、環境が対応していれば最後に `cargo xtask check-all` を
実行する。最終報告には、実行できなかったcheckと、明示的に未実装としたboundaryを記載する。

## 実装から理由を復元する（Reverse Traceability）

このworkflowは `Spec -> Test -> Implementation` のforward traceabilityだけでなく、
`Implementation -> Why -> Evidence` のreverse traceabilityも維持する。straightforwardな
実装から意図的に外れたnon-obvious codeは、将来その理由を復元できるだけのrationaleを
保持しなければならない（MUST）。詳細な分類とcomment方針は[実装理由ガイド](implementation-rationale.md)
を参照する。

理由のownerは次のように分ける。

- observable product/domain behaviorはSpecification。
- cross-cutting architectureはADR。
- regressionまたはknown bugはfocused regression testと、必要な場合のnearby rationale comment。
- platform/library/toolchain workaroundは、protected behaviorとremoval conditionを含むnearby rationale commentに、testまたはdurable referenceを補う。
- performance optimizationはbenchmark、profile、allocation evidence、またはknown hot pathにtraceする。
- obvious private implementation detailにはdocumentationを要求しない。

Approved specificationがすでにbehaviorを定義しているのにimplementationだけが違反している
場合、specificationを変更してはならない（MUST NOT）。bug fix、focused regression test、
必要に応じたlocal rationaleで修正する。Approved specからbehaviorを選択できない場合だけ、
`Specification Gap`として`refine-spec`へ戻す。local rationaleを新しいproduct requirementへ
昇格させてはならない。

unusual codeをsimplify、delete、deduplicate、replaceする前に、nearby rationale、Requirement ID、
regression test、ADR、issue/reference、benchmark、platform/library/toolchain constraintを検索
しなければならない（MUST）。refactorで実装位置が移動する場合、rationaleもprotected invariantと
ともに移動する。acceptance matrixは作業用mappingに留め、恒久的な巨大traceability表は作成しない。

## Rationale Freshnessと実装diffのreview

実装が変更されたとき、テストが通ることだけをcompletion evidenceとしては扱わない。nearby rationale、
protected invariant、関連test、Requirement ID、ADR/RFC、benchmark、またはexternal constraintに影響する
変更なら、同じ変更内でrationaleを再検証する。基本経路は次の通りである。

```text
change implementation
        -> detect affected rationale
        -> revalidate rationale
        -> keep / update / remove
        -> verify evidence references
        -> review-code
```

対象は、rationale commentの近接code変更、rationaleが守るfunction/moduleのrefactor、protected invariantの
変更、参照testの変更・削除、Requirement IDやADR/RFCの変更、依存関係・toolchain・platform assumptionの
変更、optimizationまたはworkaroundの変更、simplify・deduplicate・replace、およびordering・timing・
filesystem・concurrencyの変更である。whitespaceやmechanical renameなど意味的に無関係な変更を過剰に
blockしてはならない（MUST NOT）。

再検証の結果は次のいずれかである。

- `Still accurate`: current implementation、failure mode、evidenceに一致するため保持する。
- `Invariant/reason changed`: 保護対象または理由を更新し、参照evidenceも再確認する。
- `Reason no longer applies`: invariantが消えた、または別のevidenceで十分に保護されていることを確認して
  obsolete rationaleを削除する。

`review-code`は実装diffのspec conformance、regression safety、rationale freshness、reverse traceability、
evidence integrity、architecture boundaryを担当する。`review-spec`の代替ではなく、specificationの
semanticを変更したりOpen Questionを解決したりしない。明示されたreferenceの存在は、可能な範囲で
`cargo xtask check-rationale`が検証するが、WHYがcurrent implementationに正しいか、failure modeがまだ
存在するか、workaroundがまだ必要かはAI/code reviewが判断する。

reviewでは、少なくとも`Blocking`、`Non-blocking`、`Rationale Gap`、`Stale Rationale`、`Evidence Gap`、
`Specification Gap`を区別する。Approved spec違反はspecを後付けで変更せずbug fixとして扱い、Approved specから
behaviorを選べない場合だけ`Specification Gap`として`refine-spec`へ戻す。

## Workflowを健全に保つ

`skills/` 配下のrepository skillsはcodeと同様にreview・version controlする。agentが `MAY` を `SHOULD` に
変更するなど、incidentによって反復するfailure modeが
判明した場合は、該当skillと、必要に応じてこの文書へfocusedなruleまたはregression exampleを
追加する。skillの改善でもrole boundaryと人間によるapproval gateを保たなければならない。

## RFCとADRへの振り分け

大きなdesignについてalternativeを比較中ならRFCを使用する。choiceを採用した後の実際の
product/domain behaviorはspecificationへ記録する。crate boundary、schema representation、
identity、compatibility、external bridgeに影響するなど、architectural optionを選んだ理由を
残す必要がある場合はADRを使用する。RFCもADRもApproved specificationを暗黙に上書きしない。
