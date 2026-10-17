---
name: review-spec
description: 確定した提案を別passで吟味し、意図への忠実性、状態遷移、回復、検証可能性と承認適格性を評価する。
---

# review-spec

[Specification Workflow](../../../docs/contributing/specification-workflow.md)に従い、proposal確定後のfresh challenge passを行う。別agentの起動は必須ではない。

確認するのは、Human intentのstrength、Agent Decisionの明示、Objectiveとの整合性、canonical owner、状態・失敗・回復、acceptance、compatibility / data safety、Human gate。GUIでは代表作業を通し、見た目と実際の操作が一致するかを問う。

「旧仕様・旧テストと同じ」「実装しやすい」だけを採用理由にしない。細部の好みだけをBlockingへ格上げしない。

出力を簡潔に示す。

- Blocking Issues / Non-blocking Issues / Questions（なければ`None`）。
- Approved as Proposed: Yes / No。
- Autonomous approval eligible: Yes / Noと理由。
- Human gate: Noneまたは具体的な判断。

このreview pass自体はstatusを変更しない。Blockingはrefinementへ返す。承認適格ならorchestrationが承認・適用し、選択済みObjective内で継続できる。
