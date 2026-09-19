# 仕様ワークフロー

Workflow status: Active

この文書は、Human-selected Objectiveをreview可能・実装可能なbehaviorへ変換しつつ、AI agentが不要な同期停止なしに自走するためのrepository processを定義する。product/domain semanticsには実装前に`Approved` specificationが必要だが、`Approved`への到達方法はHuman approvalだけに限定しない。

## 正規の経路（canonical path）

```text
Human-selected Objective / request
        |
        v
extract intent + repository evidence
        |
        v
refine-spec
  - source decisions
  - agent-resolvable decisions
  - Human-gated questions
        |
        v
Draft / Proposed change
        |
        v
review-spec (independent challenge pass)
        |
        +--> Human gate present ------> decision-required -> Human decision
        |
        +--> autonomous approval eligible
                    |
                    v
             Approved / Applied
                    |
                    v
implement-spec -> tests -> review-code -> correction -> verification
                    |
                    v
             Implemented / objective-complete
```

Stageやstatusはrecovery / authorityのために残すが、Human gateがなければ各段階の間にHuman promptを挟む必要はない。

## Evidenceとdecisionの区別

Conversationはrefinementのevidenceであり恒久的なspecificationではない。重要なbehaviorだけをcanonical ownerへ正規化し、長大なconversation logを保存しない。

normative textを書く前に、statementを次のように分類する。

| Class | Meaning | Normative basis |
| --- | --- | --- |
| `Decision` | Human / durable authorityが明示確定したchoice | proposed ruleの強い根拠 |
| `Requirement` | desired capability / outcome | wordingとcontextから強度を導く |
| `Constraint` | boundary / prohibition / condition | 明確ならMUST NOT等を支持 |
| `Preference` | bindingでないfavored choice | 単独ではnormativeでない |
| `Proposal` | candidate solution | 単独ではnormativeでない |
| `Idea` | exploratory possibility | 単独ではnormativeでない |
| `Question` | clarification request | 単独ではnormativeでない |
| `Open Question` | unresolved behavior | Human gateかagent-resolvableかを分類 |
| `Rejected` | 明示的に退けられたchoice | 再導入禁止のconstraintになり得る |
| `Agent Decision` | Current Objectiveを実現するためagentが選択したlow-risk decision | autonomous approval条件を満たすproposalの根拠になり得る |

`Agent Decision`をHuman発言として記録してはならない（MUST NOT）。選択したalternative、理由、compatibility / reversibility、evidenceをdurable proposalへ明示する。

## Agent-resolvable decision

HumanがObjectiveを選択した後、次をすべて満たすdecisionはagentが解決してよい（MAY）。

- selected Objectiveを実現するために必要、または自然なscope内である。
- reversibleまたは低blast-radiusである。
- persisted/source format、stable identity、public API/protocol/CLI/config、migration policyをbreakingに変更しない。
- destructive data loss、security/permission boundary、external irreversible effectを導入しない。
- existing Approved authority、Product Direction、architecture constraintsと矛盾しない。
- alternativesのtrade-offを説明でき、選択したbehaviorをtestできる。
- future maintainerが意図を誤って壊し得る場合、spec / ADR / rationale / testへ理由を残せる。

単なるimplementation convenienceだけを理由にobservable behaviorを選んではならない（MUST NOT）。choiceがmaterial product forkである場合はHuman gateへ送る。

## Human-gated semantic decision

次はagent approval対象外であり、`decision-required`へ送る。

- Current Objective / priorityのmaterial expansion。
- intentional data loss、destructive / irreversible semantics。
- breaking compatibility、stable identity、persisted/source format、public contractの変更。
- security / trust / permission / authentication boundary。
- release / publish / deployment等のexternal irreversible effect。
- materially異なる複数product optionが残り、repository authorityから合理的に一意化できない。
- source evidenceまたはrationaleが不足し、安全なchoiceを説明できない。

「新しいrequirementだから」「Approved specを変えるから」という理由だけではHuman gateにならない。

## Normative strength

- `MUST` / `MUST NOT`: explicit requirement / constraint、またはObjective内で正当化されたAgent Decision。
- `SHOULD` / `SHOULD NOT`: 例外理由を持つstrong default。
- `MAY`: permission / capability。

Human evidenceの強度を勝手に強めない。Agent Decisionを使う場合はHuman intentの翻訳ではなく、Objective内でのagent choiceとして明示する。

## 役割と境界

### refine-spec

intent/evidenceを分類し、canonical owner、affected ID、compatibility、acceptance、Open Questionを整理する。Human gateでない不足decisionはalternativesを比較してAgent Decisionとして解決してよい。

`refine-spec` pass自身はstatusを`Approved`へ変更しない。review可能な`Proposed`まで作り、approval eligibilityを明示する。これはauthorとreviewのchallenge passを分離するためであり、Human promptを要求するためではない。

Approved / Implemented canonical documentへのsemantic deltaは`docs/spec-changes/` artifactへ置く（MUST）。canonicalへ直接未reviewed semanticsを混ぜない。

### review-spec

authoring assumptionsから離れたchallenge passとして、intent fidelity、cross-spec consistency、normative strength、testability、compatibility、Human gate有無を確認する。

reviewerはOpen Questionをその場で黙って解決しない。ただしrefinementで記録されたAgent Decisionがautonomy criteriaを満たすかを検証する。

結果として次を明示する。

- Blocking Issues
- Non-blocking Issues
- Questions
- Approved as Proposed
- Autonomous approval eligible
- Human gate required

review pass自身はstatus transitionを行わない。orchestration layerがreview結果に基づきautonomous approvalまたはHuman gateへ進める。

### implement-spec

`Approved` canonical specificationだけをimplementation authorityとして使用する。`Draft` / `Proposed` / 未Applied artifactを直接実装しない。

implementation中にSpecification Gapを見つけたら、同一autonomous runでrefine/review/approvalへ戻ってよい。Human gateでなければ再度implementationへ復帰する。

## Approval lifecycle

canonical specification:

`Draft -> Proposed -> Approved -> Implemented`

specification-change artifact:

`Draft -> Proposed -> Approved -> Applied` または `Rejected`

`Approved`は「implementation authorityとして採用済み」を意味し、approverのidentityそのものをstatusへ埋め込まない。Approval Recordへprovenanceを残す。

### Autonomous approval

Proposed changeは、次をすべて満たす場合agentがapprove/applyしてよい（MAY）。

1. Human-selected Current Objectiveのscope内。
2. Human-gated semantic decisionを含まない。
3. `review-spec`でBlockingなし。
4. materialなunresolved ambiguityなし。
5. compatibility impactがnon-breaking、または既存Approved compatibility policy内で明確。
6. success / failure / regression evidenceがtestable。
7. source evidenceとAgent Decisionが区別されている。
8. Approval Recordへ少なくとも次を残す。
   - `Approval mode: Agent-autonomous`
   - Current Objective / request basis
   - review result
   - Human gateがない理由の短い記録

同じagentがrefinementとreviewを行ってもよいが、reviewはproposal確定後の別passとして行い、authoring中の私的意図をreview evidenceにしない。

### Human approval

Human gateに該当するchangeはHumanの明示decisionが必要。`Approval mode: Human`とdecision basisをApproval Recordへ残す。

Humanが特定proposalをapproveした後は、canonical mergeのためだけに追加確認を要求しない。atomicにapplyし、そのままObjective内のimplementationへ進めてよい。

## Approved specificationの変更

Approved / Implemented canonical documentへsemantic changeを直接混ぜない。change artifactをProposedとしてreviewし、approval後にcanonicalへatomic mergeする。

Implemented documentへsemantic deltaを適用した場合、新behaviorのimplementation evidenceが揃うまでcanonical statusを`Approved`へ戻す。

Requirement IDはstableに保つ。意味をmaterially置換・分割する場合はhistory / predecessorを残して新IDを使うべきかをreviewする。

## Normalization and ownership

1 semantic rule = 1 canonical owner。関連documentはcopyせずlinkする。

Requirement IDを追加する前に既存definition / wordingを検索する。runtime Diagnostic Code、test name、current implementation behaviorをrequirement authorityとして流用しない。

Product terminologyを基準にし、term ambiguityがmaterial behaviorを変える場合はrefineする。

## Durability Review

repositoryへ残す基準:

> この情報を知らないfuture developer / AI agentが、合理的だが意図に反する変更をしてregressionを起こし得るか。

Yesなら次へrouteする。

| Knowledge | Owner |
| --- | --- |
| observable Requirement / Constraint / semantic Decision | specification |
| architecture trade-off / cross-cutting WHY | ADR |
| substantial alternative comparison | RFC |
| long-term product direction | product docs |
| current priority | Current Objective |
| regression / known bug | focused test |
| unusual implementation WHY | nearby rationale + evidence |
| rejected/deferred boundary whose再導入が危険 | canonical spec / RFC / ADR as appropriate |

巨大なacceptance matrixやconversation archiveを恒久的single source of truthにしない。

## Compatibility

stable identity、serialized shape、generated API、file interpretation、protocol、config、CLI contract等へ影響するchangeではcompatibility impactを必ず記載する。

- backward compatible
- migration required
- intentionally breaking
- not applicable

のどれかを明確にする。breakingなら原則Human gate。

## Testsとfixtures

Approved behaviorには規模に応じたverification evidenceを用意する。stable end-to-end inputがcontract理解に有用ならfixture、小さなruleならfocused unit/integration testで十分。

traceabilityに有用ならRequirement IDをtest nameまたは近接commentへ置く。fixtureは固定inputであり通常executionから書き換えない。

## Reverse Traceability

forward traceability `Spec -> Test -> Implementation` に加えて `Implementation -> Why -> Evidence` を維持する。

straightforwardな実装から意図的に外れたcode、platform workaround、ordering/concurrency、unusual filesystem/error handling、optimization等は、future maintainerがsimplifyする前に理由を復元できるようnearby rationaleとevidenceを持つ。

詳細は[実装理由ガイド](implementation-rationale.md)と`review-code`。

## Current Objective

Current Objectiveはpriority / completion boundary / explicit non-scopeのownerであり、進捗logではない。原則として次の場合だけ更新する。

1. Humanがpriorityを変更した。
2. Objectiveが完了した。
3. completion boundaryがHuman decisionで変わった。
4. repository realityとの明確な矛盾を修正する。
5. Humanから次priority選択までのautonomous delegationが明示された。

Agent Decisionの細部やtest inventoryをCurrent Objectiveへ複製しない。

## Workflowを健全に保つ

workflowの目的はHuman synchronization回数を最大化することではなく、high-value model reasoningを安全に成果へ変えること。

incidentで繰り返すfailure modeが判明した場合、最小のhard invariant、focused test、rationale、skill guidanceを追加する。単発事故を理由に全taskへ新しいmanual checkpointを増やすことをdefaultにしない。
