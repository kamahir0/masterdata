---
name: implement-spec
description: Implement and verify behavior from an explicitly Approved specification while reporting specification gaps instead of inventing semantics.
---

# implement-spec

## 目的とgate

Approved specificationをfinal candidateへ実装するactivity-specific procedure。authority / safety / architecture / Git deliveryの共通ruleは`AGENTS.md`と`docs/execution-workflow.md`をownerとし、このskillでは繰り返さない。

targetが`Approved`でない場合はproduct codeを変更しない。`Implemented`ならcurrent evidenceを確認して残gapを報告する。implementation activityの完了は「first draft」ではなく、self-reviewとrequired validationを終えた**final candidate ready for verification**である。

## Context loading

freshness epochはworkflowに従う。その上でimplementationに必要なものだけ読む。

1. Current Objective / Development Stateとtarget canonical specのstatus。
2. target spec全体と、observable behavior / compatibility / architectureに直接関係するspec・ADR・accepted outcome。
3. affected code / tests / fixtures / adaptersとrelevant CI reality。

taskに無関係なspec / RFC / terminology / fixtureを固定儀式として読む必要はない。Draft / Proposed、未Applied change artifact、Accepted RFC、current codeはApproved semanticsの代替ではない。

## Work packageとacceptance mapping

promptが不完全でも、実装前にtask-localに次をrecoverする。

- Objective / authority
- completion boundary / explicit non-scope
- required invariant / failure semantics / compatibility expectation
- affected implementation boundary
- regression evidence / validation command

stable expectationが既にcanonical specにあるなら複製しない。small taskはcompact working mappingで十分で、permanent acceptance matrixを作らない。

各Requirement IDは、必要な範囲でobservable behavior、success/failure、test/fixture、owner code boundaryへ対応付ける。traceabilityに有用ならtest nameやnearby commentへRequirement IDを置く。runtime Diagnostic CodeをRequirement IDとして再利用しない。

同じApproved objectiveを閉じるためのcore/application、adapter wiring、focused refactor、tests、fixtures、local rationale、non-normative docs、validationは同一work packageに含めてよい。別semantic objective、別Human decision、unapproved future behavior、large unrelated refactor、optional cleanupは分ける。

## Implementation

Approved acceptanceを満たす最小で明瞭な変更を行う。architecture boundaryは`AGENTS.md` / ADRを優先し、adapter都合でdomain semanticsを複製しない。

test可能なbehaviorにはfocused regression evidenceを置く。stable end-to-end input自体がcontract理解に有用ならfixtureを使い、小さなruleにはunit/integration testで十分。generated snapshot/goldenはApproved outputのevidenceである場合だけ更新する。

public behavior、compatibility、diagnostics、ordering、serialization、user-visible stateをApproved authorityから安全に決められない場合は実装で補完せずSpecification Gapへ戻す。workflowのHuman gate条件に該当しないGapは、同じautonomous runで`refine-spec -> review-spec -> autonomous approval/application`を経てからimplementationへ復帰してよい。Human gateなら`decision-required`へ停止する。private helper、internal decomposition、non-observable allocation等はagentが決めてよい。

## Local rationale

non-obviousなworkaround、ordering/concurrency constraint、platform-specific behavior、intentional redundancy、optimization、unusual filesystem/error handling等を導入・変更した場合だけ、future agentがprotected invariantを復元できるlocal rationaleを近接させる。

rationale-sensitiveな変更では、実装後にaffected rationaleを再確認し、current invariant / failure mode / evidenceと一致しなければ更新または削除する。構造参照は`cargo xtask check-rationale`で検証する。test passだけでrationale freshnessを証明したことにしない。

## Final candidate completion protocol

通常flow:

```text
authority / acceptance recovery
        -> implementation
        -> focused regression evidence
        -> rationale freshness when relevant
        -> review-code self-review
        -> safe self-fix of Blocking findings
        -> affected validation
        -> cargo xtask check-rationale
        -> cargo xtask check-all
        -> diff / scope review
        -> commit / push
        -> verification-ready transition
```

`review-code`でApproved authority内のBlockingを安全に修正できるなら、first draftをverificationへ渡さず同じactivityで修正し、affected validationとself-reviewを更新する。

環境が対応する場合は`cargo xtask check-all`を最終repository checkに使う。実行不能なcheckは理由を報告し、完全なverificationを主張しない。

## Specification status

`Approved -> Implemented`へ変更してよいのは、そのspecのscope内acceptance criteriaにevidenceがあり、必要なtests/fixturesとcompatibility evidenceが同期し、required checksが成功または実行不能理由が明示され、主張するbehaviorに未解決Specification Gapがない場合だけ。

implementationに合わせてnormative languageを弱めたり、unapproved behaviorをauthorityへ昇格させたりしない。

## Specification Gap

Approved authorityがimplementationに必要なobservable behaviorを決めていない場合:

```text
Specification Gap
- Spec ID / file:
- Missing decision:
- Why implementation cannot proceed safely:
- Non-semantic implementation work that can proceed:
- Proposed route: refine-spec (and review-spec before approval)
```

Gap解消をimplementation code内で黙って行わない。Objective内のlow-risk / reversible / non-breaking decisionはspecification workflowへ戻してAgent Decisionとしてdurably定義できる。destructive、breaking compatibility、security/authority、material product fork等のHuman gateだけHumanへ戻す。

## 完了報告

final candidate reportでは、Humanがverificationへ進めるのに必要な事実だけを報告する。

- Objective / authority / completion boundary / non-scope
- implementation boundaryとcompatibility impact
- acceptance evidence、self-review、required checks
- final candidate readiness、commit SHA、push結果
- 未解決Gapまたは実行不能check

詳細なacceptance matrixや既知のcanonical semanticsをreportへ再複製しない。
