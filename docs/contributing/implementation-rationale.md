# 実装理由とReverse Traceability

Workflow status: Active

このガイドは、実装から、その実装が守っているinvariantと根拠を復元するための
repository ruleを定義する。仕様から実装へ進むforward traceability
（`Spec -> Test -> Implementation`）を補完し、将来のdeveloperまたはAI agentが
コードを簡略化・削除・置換する前に、意図された制約を確認できるようにする。

## 基本原則

straightforwardな実装から意図的に外れたnon-obvious codeは、将来その理由を
復元できるだけのrationaleを保持しなければならない（MUST）。保存するのは
「何をしているか」ではなく、次のうち該当する理由である。

- なぜstraightforwardな実装にしないのか（`WHY`）
- 削除または簡略化すると何が壊れるのか（`WHAT BREAKS IF REMOVED OR SIMPLIFIED`）
- どのtest、specification、ADR、benchmark、external constraintが根拠か（`EVIDENCE / REFERENCE`）
- いつ安全に削除できるか（必要な場合の`REMOVAL CONDITION`）

すべてのbranch、clone、allocation、cache、またはhelperにcommentを追加してはならない
（MUST NOT）。コードから明らかなprivate implementation detailには、通常rationaleは
不要である。対象は、regression回避、platform/library/toolchain差異、performance、
compatibility、ordering、timing/concurrency、意図的なstate duplicationまたはcopy、
unusual error/filesystem handling、および一見削除・簡略化できる構造である。

## 理由を置く場所

| 理由の種類 | durable evidenceのowner |
| --- | --- |
| observable product/domain behavior | `Approved` / `Implemented` Specification |
| cross-cutting architecture decision | ADR |
| regressionまたはknown bug | focused regression test。必要ならnearby rationale comment |
| platform/library/toolchain workaround | nearby rationale comment + regression testまたはdurable reference |
| performance optimization | benchmark、measured profile、allocation evidence、またはknown hot path + nearby rationale comment |
| temporary workaround | nearby rationale comment + `REMOVAL CONDITION`。必要ならdurable reference |
| obvious private implementation detail | documentation不要 |

comment、test、issue、ADR、benchmarkは、observable product behaviorの仕様authorityを
置き換えない。local rationaleは実装上の制約を説明する証拠であり、それだけで新しい
product requirementへ昇格してはならない。
すべてのlocal decisionにIssueまたはADRを作成する必要はなく、scopeとdurabilityに応じて
focused testとnearby commentだけを使用してもよい（MAY）。

## Rationale comment

non-obviousなlocal constraintをcommentで保持する場合、repositoryで意味が分かる
自然な形式を使う。次のlabelは例であり、文字列そのものを強制するものではない。

```rust
// WHY: keep the previous output until the new build is complete.
// IF REMOVED: a failed build can destroy the last valid artifact.
// EVIDENCE: failed_build_preserves_previous_output.
// REMOVE WHEN: the filesystem adapter provides the same atomic guarantee.
```

referenceだけ（例えば`See issue #123.`）を書いてはならない（MUST NOT）。referenceを
読めなくても、protected behaviorとfailure modeが最低限分かる説明を残す。dependencyや
platformに依存する場合は、可能な範囲でaffected version/platform、protected behavior、
removal conditionを記録する。

## 回帰修正と仕様変更の境界

`Approved` specificationがすでにobservable behaviorを定義していて、implementationだけが
違反している場合、specification changeは要求しない。通常はbug fix、focused regression
test、必要に応じたlocal rationaleで修正する。

`Approved` specificationからbehaviorを選択できず、修正にdomain/public behaviorの判断が
必要な場合は、`Specification Gap`として`refine-spec`へ戻す。bug workaroundを自動的に
新しいspecification requirementへ変換してはならない（MUST NOT）。

regression testはcoverageだけでなく、「この一見不自然なコードがなぜ必要か」を示す
evidenceでもある。test nameはbehaviorを表す名前にする。例えば、
`failed_build_preserves_previous_output`、`custom_type_indirect_cycle_is_rejected`、
`normalization_does_not_mutate_builder_input` は有用だが、`test_bug_1`、`test_edge_case`、
`test_clone` は理由を復元できない。

## 変更・refactor時のルール

unusual codeをsimplify、delete、deduplicate、replaceする前に、nearby rationale、
Requirement ID、regression test、ADR、issue/reference、benchmark、platform/library/toolchain
constraintを検索しなければならない（MUST）。`looks unnecessary -> delete`と進めず、
protected invariantを確認し、そのinvariantが変更後もtestまたは同等のevidenceで守られる
場合だけ変更する。

refactorで実装の場所が移動する場合、rationaleもprotected invariantとともに移動しなければ
ならない（MUST）。rationaleが不要になった場合は、なぜ不要になったかをtest、commit、ADR、
または変更後のcode structureから確認できる状態にしてから削除する。

acceptance matrixはimplement-spec中の作業用mappingとして有用だが、巨大な手動matrixを
恒久的なsingle source of truthにしない。必要なreverse traceabilityは、behaviorを説明する
test name、正確なRequirement ID comment/metadata、必要な場所のnearby rationale comment、
およびdurable evidenceによってrepository自身に保持する。

performance optimizationでは、単に「faster」とだけ記録してはならない（MUST NOT）。可能な
場合はbenchmark、profile、allocation evidence、またはknown hot pathを示す。optimizationが
observable semanticsを変更しない限り、specificationへ追加しない。observable product contract
を変更するoptimizationだけがspecificationの対象となる。
