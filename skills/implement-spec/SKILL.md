---
name: implement-spec
description: Implement and verify behavior from an explicitly Approved specification while reporting specification gaps instead of inventing semantics.
---

# implement-spec

## 目的とgate

このskillは、canonical documentが `Status: Approved` である1つ以上のspecification IDによって対象behaviorが識別される場合だけ使用する。Approved specificationは、conversation、idea、Draft、Proposed document、implementation convenienceではなく、input contractである。

targetが `Draft`、`Proposed`、または `Deprecated` の場合は、product codeを変更する前に停止し、人間のapprovalまたはreplacement specificationが必要だと報告する。targetがすでに `Implemented` の場合は、evidenceを検証し、semanticsを黙って変更せず残りのgapを報告する。

このskillはspecificationをauto-approveせず、意味を変更しない。statusを `Implemented` にできるのは、acceptance evidenceとrepository checksが完了した後だけである。

implementation activityのcompletion targetは「実装した」ではなく、**final candidate ready for verification**である。agentはApproved semantic boundaryの内側でauthority recovery、implementation、regression evidence、self-review、self-fix、required validation、scope確認、commit / pushまでを一つのtask内で閉じる。verification activityへ渡すのは、原則としてself-review済みでBlocking findingを解消したfinal candidateであり、first draftではない。

このskillを実行するagent identityは固定しない。同じagentが後続のverification activityもfreshな別passとして行ってよく、別agentへdelegateしてもよい。repository上のDevelopment Stateはassigneeやsession roleをauthorityにしない。

## 必須の準備

1. `AGENTS.md` を読む。
2. `docs/current-objective.md` と `docs/execution-state.md` をfreshに確認し、Current Objectiveがimplementation-readyであることを確認する。
3. 各target specification ID、canonical file、exact status、predecessor/deprecation relationshipを確認する。
4. target spec全体、関連specification、relevant ADR、RFCのoutcome、`docs/product/terminology.md` を読む。
5. normative requirement、validation rule、compatibility note、non-goal、non-blocking Open Questionを抽出する。
6. 対象IDの影響を受けるcurrent implementation、tests、fixtures、codegen snapshot/golden file、GUI command、.NET bridge callをrepositoryから検索する。

`docs/spec-changes/` proposal、RFC、Draft、Proposed documentをimplementation inputにしてはならない。canonical specが `Approved` でもproposed deltaが別にある場合、canonical meaningだけを実装し、そのdeltaはhuman-approved atomic mergeが完了するまでscope外として報告する。

`Accepted` RFC、`Approved` だが `Applied` ではないspecification-change artifact、current code behaviorはcanonical specificationの代替ではない。canonical fileがrequired atomic workflowなしにsemanticに変更されている場合は、新しいbehaviorを実装せずworkflow violationを報告する。

Approved specificationと一致しないcurrent implementation、fixture、generated artifactをauthorityとして扱わない。Approvedでないfuture Type Systemや他のfeatureを、documentが存在するという理由で実装しない。

## 実装フロー（Implementation flow）

### 1. Work packageとacceptance mappingをrecoverする

task開始時のpromptが完全な形式でなくても、repositoryとtarget objectiveから次のwork package contractを内部的に整理する。

- Objective
- authority specification / Requirement ID
- completion boundary
- required invariantとfailure semantics
- explicit non-scope
- affected implementation boundary
- required regression evidence
- validation command

これはtask-localなworking noteであり、commitするdurable documentではない。既存specificationにstableなacceptance expectationがある場合は重複copyせず、small taskではcompact checklistまたはworking mappingで十分とする。巨大なpermanent acceptance matrixを毎回作成してはならない。

同じApproved objectiveを閉じるために必要なapplication/core implementation、CLI/adapter wiring、focused refactor、tests、fixture、local rationale、non-normative documentation correction、validationは一つのwork packageにまとめてよい。別のsemantic objective、別のHuman decision、unapproved future behavior、large unrelated refactor、optional cleanupは分ける。file数やruntime/testの違いだけを理由に機械的な分割を行わない。

各Requirement IDについて、次を記録する。

- observable behavior
- successとfailure condition
- compatibility expectation
- unit/integration/GUI test
- 必要に応じたfixtureまたはgenerated artifact evidence
- 所有すべきsource fileとcode boundary

traceabilityを明確にできる場合は、test nameまたは近接commentにRequirement IDを使う。testはApproved wordingを検証し、Open Questionへの答えを黙って選んではならない。

runtime Diagnostic Codeは独自のnamespaceに保つ（例: `E-PROJECT-NOT-FOUND`）。`PROJECT-004` のようなRequirement IDを再利用してはならない。related-requirement metadataは別に保持する。

### 2. Architecture impactを特定する

domain logicは `masterdata-core` に置き、CLIとGUIで共有する。GUI codeはfile discoveryやYAML semanticsを扱ってはならず、coreを呼ぶTauri commandを使用する。.NET process invocationは `masterdata-dotnet` に置く。MasterMemory internal、binary format、Source Generator behaviorをRustで再実装してはならない。semantic ruleを便利なadapterへ移したり、複数layerで重複させたりしない。

### 3. Testsとfixturesをplanする

behaviorがtest可能なら、testを先に用意することを優先する。stableなend-to-end inputがruleを伝える場合は、`fixtures/minimal`、`fixtures/full`、`fixtures/invalid` を追加または更新する。小さなruleにはfocused unitまたはintegration testで十分である。fixtureは固定inputであり、`cargo xtask` 経由でcopyする。通常のCLI/GUI executionにfixtureを書き換えさせてはならない。

generated C#では、Approved behaviorの一部である場合だけsnapshot/golden evidenceを更新する。未承認のoutputを承認済みに見せるためだけにsnapshotを追加しない。

### 4. 実装して検証する

acceptance mappingを満たす最小の変更を実装する。その後、関連するunit、integration、frontend、GUI、codegen、.NET bridge checkを実行する。環境が対応していれば最後に `cargo xtask check-all` を実行する。checkを実行できない場合は正確な理由を記録し、完全なverificationを主張してはならない。

### Reverse traceabilityとlocal rationale

implementation中に、non-obviousなworkaround、optimization、ordering constraint、platform-specific path、timing/concurrency rule、intentionalなredundancy、clone/copy/cache/allocation、または unusual error/filesystem operationを導入する場合は、future developerまたはAIが理由を復元できるlocal rationaleをprotected invariantの近くに保持しなければならない（MUST）。必要に応じて、`WHY`、削除・簡略化した場合のfailure mode、`EVIDENCE / REFERENCE`、`REMOVAL CONDITION`を記録する。

regressionでは、behaviorを説明するfocused test nameを優先し、必要ならRequirement IDをnearby commentまたはmetadataで対応付ける。performance optimizationは、可能な範囲でbenchmark、profile、allocation evidence、またはknown hot pathへtraceする。issueやURLは理由の代わりにならない。

refactorで実装位置を移動する場合、rationaleもprotected invariantとともに移動する。rationaleが不要になった場合は、なぜ不要になったかをtest、commit、ADR、または変更後のcode structureから確認してから削除する。

local implementation rationaleを新しいproduct requirementへ変換してはならない（MUST NOT）。observable behaviorが変更される場合だけ、既存のApproved specと照合し、必要なら`refine-spec`へ戻す。Approved specが既にbehaviorを定義していて実装だけが違反する場合は、specを変更せずbug fixとregression evidenceを行う。

### Rationale Freshnessのcompletion gate

実装変更後、次を同じchangeのcompletion flowとして実行する。

1. 関連testを実行する。
2. touched implementationの近くにあるrationaleを再検索する。
3. 影響する各rationaleについて、current implementation、protected invariant、failure mode、evidenceがまだ一致するかを確認する。
4. 正確なら保持し、invariantまたは理由が変わったら更新し、理由が不要になったら削除する。
5. `Requirement ID`、ADR/RFC、`Regression:` test name、repository-relative documentation pathなど、commentに明示された構造参照を`cargo xtask check-rationale`で検証する。
6. `review-code`をexecutor自身のself-reviewとして実行し、必要な修正後に`cargo xtask check-all`を実行する。

code compiles、tests passだけではrationale-sensitiveな変更の完了とはみなさない。referenceが存在してもstaleな理由を残してはならない。逆に、semantic freshnessを機械checkが証明したと主張してはならない。理由が不明な場合は新しいreasonを発明せず、`Rationale Gap`または`Specification Gap`として報告する。

## Final candidate completion protocol

通常のApproved implementation taskでは、次のflowを同じwork package内で完了する。

```text
authority recovery
        -> acceptance / Requirement mapping
        -> implementation plan
        -> implementation
        -> focused tests / regression evidence
        -> rationale freshness scan
        -> review-code self-review
        -> Blocking findingを安全に自己修正
        -> affected tests再実行
        -> review-code再確認
        -> cargo xtask check-rationale
        -> cargo xtask check-all
        -> diff / scope self-review
        -> commit / push
        -> final candidate completion report
        -> Development Stateをverification-readyへ遷移
```

`review-code`でBlocking findingが見つかり、Approved authorityの範囲内で安全に修正できる場合は、verification activityへfirst draftを渡さず、同じtask内で修正する。修正後は影響するtest、必要なrationale確認、`review-code`を再実行し、その後にrequired checkとscope確認を行う。

次の場合は、Blockingを解消するためにsemantic decisionを発明してはならない。

- Specification Gap
- Approved authorityのconflict
- Human Approvalが必要な変更
- destructive semanticsが不明な操作
- safetyまたはcompatibilityを守るrationaleをrecoverできない変更

この場合はunfinished implementationを無理にfinal candidateとせず、既存のSpecification Gap protocolまたは明確なHuman decision reportへ戻す。private helper name、internal decomposition、test helper、private error plumbing、non-observable allocationなどは、observable behaviorを変えない限りagent自身で決定してよい。

### 5. Specificationとimplementationを照合する

statusを変更する前に、すべてのRequirement IDをimplementationとtest evidenceに照合する。specificationを更新できるのは別途approvedされたsemantic changeだけであり、implementation workによってcontractを変更して未完成implementationを正しく見せてはならない。

既存implementationにunapproved domain assumptionがないかも確認する。例として、approved index specificationがない限り、`id` fieldはprimary keyではない。このようなassumptionはevidenceにせず、removeまたはreportする。Diagnosticの`related_requirements` entryがsemantically正確であること、test name/commentが主張するRequirement IDと実際に対応することも確認する。

次の条件をすべて満たした場合だけ、`Status: Approved` を `Status: Implemented` に変更する。

- scope内の全acceptance criteriaにevidenceがある。
- testsと適切なfixturesが同期している。
- compatibility behaviorが検証済みまたは明示的に文書化されている。
- repository checksが成功している、または実行不能なcheckを明示的に報告している。
- 主張するbehaviorに影響する未解決のSpecification Gapがない。

## Specification Gapの扱い（Specification Gap protocol）

Approved specificationがimplementationに必要なbehaviorを未定義のまま残している場合、domain ruleを黙って選択してはならない。次の形式で報告する。

```text
Specification Gap
- Spec ID / file:
- Missing decision:
- Why implementation cannot proceed safely:
- Non-semantic implementation work that can proceed:
- Proposed route: refine-spec (and review-spec before approval)
```

private helper nameやallocation strategyなど、observable behaviorに影響しないinternal choiceは通常どおり決めてもよい。public behavior、compatibility、diagnostics、ordering、serialization、user-visible GUI stateに影響し得るchoiceはspecification gapである。

## 完了報告（Required completion report）

次を報告する。

- work package contract（Objective、authority、completion boundary、invariant、failure semantics、non-scope）
- target specification IDとbefore/after status
- acceptance criteriaとtest/fixture mapping
- 変更したimplementation boundary
- compatibility impact
- self-reviewの結果、Blocking findingの自己修正有無、およびfinal candidateとしてverification-readyな状態か
- 実行したcommandと結果（`cargo xtask check-all` を含む）
- commit SHAとpush結果
- 未実装boundaryまたはSpecification Gap

## 絶対に外せない安全策

- Approved specが必要な場合、conversationから直接実装しない。
- DraftまたはProposed specを実装中に昇格させない。
- Open Questionをcodeでdefaultへ解決しない。
- codeに合わせるためnormative languageを弱めたり強めたりしない。
- unapproved semanticsを含むcurrent implementation behaviorをauthorityとして扱わない。remove、report、またはrefinementへ戻す。
- canonical specificationがatomicに更新される前に、proposed Approved-spec changeを実装しない。
- Diagnostic CodeをRequirement IDとして扱わず、test name/commentが主張するrequirementを説明していることを確認する。
- CLI、GUI、.NET adapterでcore domain semanticsを重複させない。
- testとverification evidenceなしに `Implemented` と主張しない。
