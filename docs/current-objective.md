# Current Objective

## Objective

ブラウザ版MasterDataに「プロジェクト新規作成」を導入する。ウェルカム画面から、利用者が選んだローカルフォルダ自体をプロジェクトルートとして初期化し、そのまま編集sessionを開始して直近プロジェクトとして永続化する。

## Completion boundary

[変更提案0027](spec-changes/0027-browser-project-create.md)の仕様策定・review・承認、[Browser Authoring](specs/browser.md)への適用、`src/browser/`と`src/App.tsx`の実装、focused unit / UI testsとworkspace check、通常commit / push。Desktop版の既存挙動とCLI `init`との互換を維持する。
Authorization: Humanの「ブラウザ版MasterData（Web版）に『プロジェクト新規作成』機能を導入してください」（2026-10-27）。

## Non-scope

Browserでのsample作成、Build / Publish、親フォルダへの権限要求、Desktop / CLIの挙動変更、release / tag、Edge実機検証、ユーザーの既存変更。
