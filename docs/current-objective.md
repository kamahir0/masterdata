# Current Objective

## Objective

ブラウザ版において、選択したプロジェクトフォルダのハンドル（`FileSystemDirectoryHandle`）を IndexedDB に永続化し、次回アクセス時に直近プロジェクトを再開・権限復旧できるようにする。永続権限（Persistent Permissions）が付与されている場合は起動時に自動復元し、権限確認が必要な場合もフォルダピッカーを開き直すことなくワンクリックで再開できる導線を提供する。

## Completion boundary

仕様変更提案（0023-browser-persistent-project-handle）の作成・別pass review・自律承認、`docs/specs/browser.md`（WEB-005）の更新、IndexedDB 永続化および `initial()` / スタート画面 UI の実装、focused validation（テストの追加・全テスト通過）、`python3 tools/check_workspace.py`、`git diff --check`、self-review、通常 commit / push。

## Non-scope

Safari / Firefox 向けの代替ファイルアクセス実装、OPFS（サンドボックス）へのプロジェクト複製、Desktop / CLI の既存挙動変更。
