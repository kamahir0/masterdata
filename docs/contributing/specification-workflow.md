# Specification Workflow

Status: Active

仕様のlifecycleとimplementation authorityを所有する。自律実行・Human gateは[Execution Workflow](../execution-workflow.md)、文書の配置・量は[Documentation Policy](documentation-policy.md)に従う。

## 流れ

```text
Human request / Current Objective
  → 意図・作業シナリオ・根拠の整理
  → Draft / Proposed
  → 別passでのchallenge review
  → Human gateなら判断取得、それ以外は条件付き自律承認
  → Approved仕様へ適用
  → 実装 / focused tests / self-review / 実際の作業で検証
  → evidenceが揃った範囲をImplementedへ
```

仕様駆動は、最初に製品全体を凍結することを意味しない。選択されたObjectiveを実装・検証できる範囲で具体化し、実際の操作から問題が分かれば同じworkflowで修正する。

## 根拠と判断

必要なstatementについて、HumanのDecision / Requirement / Constraint、Preference、Proposal / Idea / Question、Rejected、Agent Decisionを区別する。Agent DecisionをHuman発言として記録しない。

agentはObjective内の選択肢を比較し、理由と観察可能な結果を説明でき、Human gateに該当しない判断を解決してよい。実装の都合だけでユーザーに見える振る舞いを決めない。

## 最小の仕様内容

- 解く問題と対象作業、対象外。
- source evidenceとAgent Decisionの区別。
- 状態、操作、結果。必要なエラー・取消・失敗・回復の振る舞い。
- 観察可能なacceptanceと、変更に合った検証方法。
- 未決定事項、compatibility / data safetyへの影響。

大きなGUI機能では見た目だけでなく、選択・フォーカス・編集中・保存状態の遷移を具体的な作業で確認する。細かい内部構造をbehaviorの要件へ固定しない。

Requirement IDは必要な単位でstableに付け、追加前に既存definitionを検索する。Diagnostic Codeやtest nameと混同しない。MUST / MUST NOTは根拠のあるhard invariantに限る。

## Statusと承認

canonical仕様: `Draft → Proposed → Approved → Implemented`。
既存仕様への変更提案: `Draft → Proposed → Approved → Applied`、または`Rejected`。

初めての仕様は`docs/specs/`へDraftとして作成してよい。Approved / Implementedな仕様のsemantic changeは`docs/spec-changes/`の提案でreviewし、承認後にcanonicalへ適用する。誤字・link等の意味を変えない修正には提案を要求しない。

refinement passは承認しない。proposal確定後に別passでreviewし、review passは判定を返す。別agentの起動は必須ではない。reviewではintent fidelity、整合性、規範の強さ、状態と失敗の具体性、検証可能性、scope、Human gateを確認する。

次をすべて満たす場合、orchestrationを行うagentが自律承認し、同じrunで実装へ進めてよい。

- 選択済みObjective内で必要または自然なscope。
- reviewにBlockingなし、materialな未解決事項なし。
- Human gateなし、互換性とdata safetyの影響が明確。
- 観察可能なacceptanceと検証方法があり、Human evidenceとAgent Decisionが区別されている。

承認対象の文書または変更提案に、`Approval mode: Agent-autonomous`、Objective / request basis、review結果を簡潔に記録する。Human判断なら`Approval mode: Human`とdecision basisを残す。

承認済み仕様を更新した場合は、新behaviorのevidenceが揃うまで`Approved`として扱う。コードに合わせて未承認のbehaviorを追認したり、通るように要件を弱めたりしない。

## 実装・検証との関係

code / testsは実際のbehaviorの証拠であり、仕様やユーザー意図を自動で決めるauthorityではない。仕様と実装が違うだけならbug fix、仕様から期待結果を選べないならSpecification Gapとしてこのworkflowへ戻る。

仕様への適合、回帰テスト、実際の作業の使いやすさをそれぞれ確認する。テストのためだけに無関係なlarge fixtureやbenchmarkを要求しない。必要に応じてRequirement IDからtest / codeへtraceできるようにする。
