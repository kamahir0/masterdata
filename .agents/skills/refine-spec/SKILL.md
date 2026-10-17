---
name: refine-spec
description: ユーザーの目的と作業シナリオから、根拠とagent判断を区別したDraft/Proposed仕様を作成する。
---

# refine-spec

[Specification Workflow](../../../docs/contributing/specification-workflow.md)に従い、選択されたObjectiveをreview可能な仕様へ具体化する。Human gateは[Execution Workflow](../../../docs/execution-workflow.md#human-gate)が所有する。

1. Current Objective、Development State、関係する仕様・理由・証拠だけを確認する。
2. 要望の表面的な実装案に留まらず、達成したい作業、困り方、類似ケースを整理する。
3. HumanのDecision / Requirement / Preference等とAgent Decisionを区別する。旧資料やcurrent codeをauthorityにしない。
4. canonical ownerを選び、状態・操作・結果、失敗・回復、acceptance、non-scope、未決定事項を必要な範囲で書く。
5. Objective内で判断可能な事項は理由を示して解決し、Human gateだけを残す。
6. 既存Approved仕様へのsemantic changeは別提案に置く。review可能なら`Proposed`とする。

このpass自身は承認や製品実装をしない。出力は提案、source / Agent Decision、検証方法、残る判断、autonomous approval eligibility。内容が小さければ一つの短い文書でよい。
