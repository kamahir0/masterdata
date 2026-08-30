---
name: implement-spec
description: Implement and verify behavior from an explicitly Approved specification while reporting specification gaps instead of inventing semantics.
---

# implement-spec

## 目的とgate

このskillは、canonical documentが `Status: Approved` である1つ以上のspecification IDによって対象behaviorが識別
される場合だけ使用する。Approved specificationは、conversation、idea、Draft、Proposed document、implementation
convenienceではなく、input contractである。

targetが `Draft`、`Proposed`、または `Deprecated` の場合は、product codeを変更する前に停止し、人間のapprovalまたは
replacement specificationが必要だと報告する。targetがすでに `Implemented` の場合は、evidenceを検証し、semanticsを
黙って変更せず残りのgapを報告する。

このskillはspecificationをauto-approveせず、意味を変更しない。statusを `Implemented` にできるのは、acceptance
evidenceとrepository checksが完了した後だけである。

## 必須の準備

1. `AGENTS.md` を読む。
2. 各target specification ID、canonical file、exact status、predecessor/deprecation relationshipを確認する。
3. target spec全体、関連specification、relevant ADR、RFCのoutcome、`docs/product/terminology.md` を読む。
4. normative requirement、validation rule、compatibility note、non-goal、non-blocking Open Questionを抽出する。
5. 対象IDの影響を受けるcurrent implementation、tests、fixtures、codegen snapshot/golden file、GUI command、.NET bridge
   callをrepositoryから検索する。

`docs/spec-changes/` proposal、RFC、Draft、Proposed documentをimplementation inputにしてはならない。
canonical specが `Approved` でもproposed deltaが別にある場合、canonical meaningだけを実装し、そのdeltaはhuman-approved
atomic mergeが完了するまでscope外として報告する。

`Accepted` RFC、`Approved` だが `Applied` ではないspecification-change artifact、current code behaviorはcanonical
specificationの代替ではない。canonical fileがrequired atomic workflowなしにsemanticに変更されている場合は、新しい
behaviorを実装せずworkflow violationを報告する。

Approved specificationと一致しないcurrent implementation、fixture、generated artifactをauthorityとして扱わない。
Approvedでないfuture Type Systemや他のfeatureを、documentが存在するという理由で実装しない。

## 実装フロー（Implementation flow）

### 1. Acceptance matrixを作成する

各Requirement IDについて、次を記録する。

- observable behavior
- successとfailure condition
- compatibility expectation
- unit/integration/GUI test
- 必要に応じたfixtureまたはgenerated artifact evidence
- 所有すべきsource fileとcode boundary

traceabilityを明確にできる場合は、test nameまたは近接commentにRequirement IDを使う。testはApproved wordingを検証し、
Open Questionへの答えを黙って選んではならない。

runtime Diagnostic Codeは独自のnamespaceに保つ（例: `E-PROJECT-NOT-FOUND`）。`PROJECT-004` のようなRequirement IDを
再利用してはならない。related-requirement metadataは別に保持する。

### 2. Architecture impactを特定する

domain logicは `masterdata-core` に置き、CLIとGUIで共有する。GUI codeはfile discoveryやYAML semanticsを扱っては
ならず、coreを呼ぶTauri commandを使用する。.NET process invocationは `masterdata-dotnet` に置く。
MasterMemory internal、binary format、Source Generator behaviorをRustで再実装してはならない。semantic ruleを
便利なadapterへ移したり、複数layerで重複させたりしない。

### 3. Testsとfixturesをplanする

behaviorがtest可能なら、testを先に用意することを優先する。stableなend-to-end inputがruleを伝える場合は、
`fixtures/minimal`、`fixtures/full`、`fixtures/invalid` を追加または更新する。小さなruleにはfocused unitまたは
integration testで十分である。fixtureは固定inputであり、`cargo xtask` 経由でcopyする。通常のCLI/GUI executionにfixtureを
書き換えさせてはならない。

generated C#では、Approved behaviorの一部である場合だけsnapshot/golden evidenceを更新する。未承認のoutputを承認済み
に見せるためだけにsnapshotを追加しない。

### 4. 実装して検証する

acceptance matrixを満たす最小の変更を実装する。その後、関連するunit、integration、frontend、GUI、codegen、.NET bridge
checkを実行する。環境が対応していれば最後に `cargo xtask check-all` を実行する。checkを実行できない場合は正確な
理由を記録し、完全なverificationを主張してはならない。

### Reverse traceabilityとlocal rationale

implementation中に、non-obviousなworkaround、optimization、ordering constraint、platform-specific
path、timing/concurrency rule、intentionalなredundancy、clone/copy/cache/allocation、または unusual
error/filesystem operationを導入する場合は、future developerまたはAIが理由を復元できるlocal
rationaleをprotected invariantの近くに保持しなければならない（MUST）。必要に応じて、`WHY`、
削除・簡略化した場合のfailure mode、`EVIDENCE / REFERENCE`、`REMOVAL CONDITION`を記録する。

regressionでは、behaviorを説明するfocused test nameを優先し、必要ならRequirement IDをnearby
commentまたはmetadataで対応付ける。performance optimizationは、可能な範囲でbenchmark、profile、
allocation evidence、またはknown hot pathへtraceする。issueやURLは理由の代わりにならない。

refactorで実装位置を移動する場合、rationaleもprotected invariantとともに移動する。rationaleが
不要になった場合は、なぜ不要になったかをtest、commit、ADR、または変更後のcode structureから
確認してから削除する。

local implementation rationaleを新しいproduct requirementへ変換してはならない（MUST NOT）。
observable behaviorが変更される場合だけ、既存のApproved specと照合し、必要なら`refine-spec`
へ戻す。Approved specが既にbehaviorを定義していて実装だけが違反する場合は、specを変更せず
bug fixとregression evidenceを行う。

### Rationale Freshnessのcompletion gate

実装変更後、次を同じchangeのcompletion flowとして実行する。

1. 関連testを実行する。
2. touched implementationの近くにあるrationaleを再検索する。
3. 影響する各rationaleについて、current implementation、protected invariant、failure mode、evidenceが
   まだ一致するかを確認する。
4. 正確なら保持し、invariantまたは理由が変わったら更新し、理由が不要になったら削除する。
5. `Requirement ID`、ADR/RFC、`Regression:` test name、repository-relative documentation pathなど、
   commentに明示された構造参照を`cargo xtask check-rationale`で検証する。
6. `review-code`をfinal implementation reviewとして実行し、必要な修正後に`cargo xtask check-all`を実行する。

code compiles、tests passだけではrationale-sensitiveな変更の完了とはみなさない。referenceが存在しても、
staleな理由を残してはならない。逆に、semantic freshnessを機械checkが証明したと主張してはならない。
理由が不明な場合は新しいreasonを発明せず、`Rationale Gap`または`Specification Gap`として報告する。

### 5. Specificationとimplementationを照合する

statusを変更する前に、すべてのRequirement IDをimplementationとtest evidenceに照合する。specificationを更新できるのは
別途approvedされたsemantic changeだけであり、implementation workによってcontractを変更して未完成implementationを
正しく見せてはならない。

既存implementationにunapproved domain assumptionがないかも確認する。例として、approved index specificationがない限り、
`id` fieldはprimary keyではない。このようなassumptionはevidenceにせず、removeまたはreportする。Diagnosticの
`related_requirements` entryがsemantically正確であること、test name/commentが主張するRequirement IDと実際に対応する
ことも確認する。

次の条件をすべて満たした場合だけ、`Status: Approved` を `Status: Implemented` に変更する。

- scope内の全acceptance criteriaにevidenceがある。
- testsと適切なfixturesが同期している。
- compatibility behaviorが検証済みまたは明示的に文書化されている。
- repository checksが成功している、または実行不能なcheckを明示的に報告している。
- 主張するbehaviorに影響する未解決のSpecification Gapがない。

## Specification Gapの扱い（Specification Gap protocol）

Approved specificationがimplementationに必要なbehaviorを未定義のまま残している場合、domain ruleを黙って選択しては
ならない。次の形式で報告する。

```text
Specification Gap
- Spec ID / file:
- Missing decision:
- Why implementation cannot proceed safely:
- Non-semantic implementation work that can proceed:
- Proposed route: refine-spec (and review-spec before approval)
```

private helper nameやallocation strategyなど、observable behaviorに影響しないinternal choiceは通常どおり決めてもよい。
public behavior、compatibility、diagnostics、ordering、serialization、user-visible GUI stateに影響し得るchoiceは
specification gapである。

## 完了報告（Required completion report）

次を報告する。

- target specification IDとbefore/after status
- acceptance criteriaとtest/fixture mapping
- 変更したimplementation boundary
- compatibility impact
- 実行したcommandと結果（`cargo xtask check-all` を含む）
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
