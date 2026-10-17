# 編集対象・一時操作・表示の整理

Status: Applied

Approval mode: Agent-autonomous
Basis: 2026-10-17 Humanの具体的なサンプル操作レビューと既存の自律実装委任。別passでintent、対象identity、一度だけのjump、保存の値保持、reduced motion、scopeをreview。Blocking / Non-blocking / Questions: None。Approved as Proposed: Yes。Autonomous approval eligible: Yes。Human gate: None。

## 判断と適用先

[Desktop Authoring](../specs/desktop.md)のEDIT-002 / EDIT-004 / UI-001へ適用した。Humanの表示・操作要求を採用し、YAMLは以前のflow書式の保守を優先せず、編集して保存する有効sourceをblock形式へ統一するAgent Decisionとした。

navigationによる対象変更と値操作、scrollと構造並替え、一回の問題箇所へのjumpと継続する検索を区別する。見た目の変更で入力やデータを失わせない。詳細behaviorはcanonical仕様、回帰証拠はtests、変更履歴はGitが所有する。
