# Development State

Stage: implementation-ready
Candidate: none
Work base: 79781b66aa43f83315ed2f614c7596d398b75324

## Active work

In progress: Human-selected [開発者用語の見直し](spec-changes/0057-desktop-developer-terminology.md)の実装とfocused validationが完了。actual Desktop表示と新Candidateのrequired CIを確認する。既存external evidence gatesは維持する。

Completed: GUI-SHELL-PRESENTATION-001 / 002のHuman-selected磨き込み。actual macOS review / source hashes / fresh review / production app更新、固定Candidateの全17 required CI jobs / integrityがPASS。結果は[GUI磨き込みevidence](evidence/clean-room-candidate.md#gui磨き込み追加scope2026-10-10)へ記録。今回scopeのactive implementationなし、既存external evidence gatesは維持する。

Completed: 2026-10-10 Human-selected Desktop操作feedback（0056）の実装 / macOS actual review / source hashes / fresh review。固定Candidate全17 required CI jobs / integrityがPASS。結果は[追加scope evidence](evidence/clean-room-candidate.md#desktop操作feedback追加scope2026-10-10)へ記録。今回scopeのactive implementationなし、既存external evidence gatesは維持する。

Completed: `GUI-SHELL-LANGUAGE-001`の日本語化、focused検証、macOS actual日本語UI確認、日本語Candidateの全17 required CI jobs / integrity reconciliation。既存external evidence gateは維持する。

Completed: canonical authoring / write / structural / delivery / CLI / portable Unity implementation、全portable oracle adapters、React / Ant Design代表surface review、100k capacity、macOS OS入力2run、frozen-boundary fresh / adversarial review、Candidate required CI reconciliation。
Completed: 許可済みmacOS実IMEで発見した`GUI-GRID-006`違反をcell / nested inputで修正。actual IME再検証 / 一Undo / source hashes、Reduced Motion ON / OFF復元、修正Candidate全14 CI jobs / integrityがPASS。
Remaining: [Candidate evidence / finite ledger](evidence/clean-room-candidate.md#human-gates--verdict)のrequired external evidence。許可済みCGEvent helperはzero-input permission preflightで停止し、入力・product変更なし。未取得の証拠を追加実装や新しいacceptanceで置換しない。

## Blocking findings

Evidence gaps: macOS actual first-accepted <150ms証明 / held-drag Escape（CGEvent permission不足、設定変更禁止）、Windows x64 actual OS interaction / performance、Unity actual Editor / runtime。実IMEのconcrete Blockingは修正・再検証済みで他の既知製品Blockingは未検出。Cutover-ready / objective-complete未達。

## Human decision needed

[exact gaps / evidence / choices](evidence/clean-room-candidate.md#human-gates--verdict)。Windows / Unity実行環境は未提供。macOS CGEvent使用は明示authorization済みだが、`CGPreflightPostEventAccess()`がfalseで送信前に停止した。Human条件どおりpermission設定を変更しない。残scopeは有限ledgerの既存evidenceと実際に発見したcontract defectだけ。main merge / cutoverはscope外。
