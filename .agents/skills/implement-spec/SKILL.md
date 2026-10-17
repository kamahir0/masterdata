---
name: implement-spec
description: Approvedな振る舞いを実装し、focused validation、self-review、実際の作業検証、Git deliveryまで進める。
---

# implement-spec

[Execution Workflow](../../../docs/execution-workflow.md)と[Specification Workflow](../../../docs/contributing/specification-workflow.md)に従う。

1. Objective、Approved仕様、必要な設計理由、Git差分、変更対象を確認する。
2. acceptance、失敗・回復、data safety、対象外を確認し、製品の要求を満たす明瞭な実装を行う。
3. behaviorに合わせたfocused validationを行う。GUIでは代表作業、状態の可視性、入力と操作の一貫性も確認する。
4. 仕様から期待結果を決められない場合はSpecification Gapとして戻る。Human gateがなければ同じrunで整理・review・承認後に再開する。
5. 必要な箇所だけ[実装理由](../../../docs/contributing/implementation-rationale.md)を残し、変更時は鮮度を確認する。
6. `review-code`でself-reviewし、安全に直せるBlockingを修正して影響する検証を再実行する。
7. local checks、scope確認、commit / push、Candidate記録、fresh verification、必要なCIまで進める。

新たな機能やbenchmarkを完了直前に要求しない。チェックやruntimeがまだない場合は、その事実を示し、旧製品のtoolingを復元しない。検証結果、Candidate、delivery、残る制約を報告する。
