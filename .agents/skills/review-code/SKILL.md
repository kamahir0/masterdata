---
name: review-code
description: 製品の正しさ、安全性、UX、仕様適合と証拠を優先して差分をreviewし、具体的なBlockingを特定する。
---

# review-code

[Execution Workflow](../../../docs/execution-workflow.md#git-delivery-and-completion)に従い、対象diff、Objective、Approved仕様、code / testsの実際を確認する。

第一passは製品へ集中する。correctness、data safety、状態遷移、入力保持、race / failure / recovery、UX、architecture、scopeを確認する。

第二passではsupporting evidenceを確認する。focused tests、実際の作業確認、CI、参照、必要な[実装理由の鮮度](../../../docs/contributing/implementation-rationale.md)を調べる。test passだけでUXや理由の正しさを証明したことにしない。

仕様・実装・旧資料の三者を混同しない。新しい期待結果が必要ならSpecification Gap、実装がApproved仕様に反するならbugとして分類する。

出力はscope、仕様適合、検証と限界、rationale freshness、findings、verdictを短く示す。実質的なcorrectness / safety / contractの問題はBlocking、好みや任意改善はNon-blockingとする。

Blockingがなければ完了を妨げない。ある場合は具体的な修正と影響する検証へ絞る。authoringから離れた最終reviewでも同じ基準を使い、別agentの起動を必須にしない。
