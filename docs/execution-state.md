# Development State

Stage: objective-complete
Candidate: none
Work base: 00b71886b3266cc0c831c1113ca0d0df9e194cc0

## Active work

None.

## Blocking findings

None.

## Completion evidence

ブラウザ版におけるプロジェクトフォルダハンドルのIndexedDB永続化と権限再開機能を実装・検証完了。
1. `src/browser/storage.ts` を新設し、`FileSystemDirectoryHandle` を IndexedDB に安全に保存・読出・クリアする永続層を実装。
2. `src/browser/runtime.ts` の `initial()` において、保存済みハンドルが存在し `queryPermission` が `"granted"`（Chrome 122+ 永続権限）であれば直近プロジェクトを自動ロードし、権限確認が必要な場合は `browserStatus` に前回のプロジェクト情報を保持。
3. `src/App.tsx` のスタート画面に「前回のプロジェクトを開く（フォルダ名）」ボタンを配置し、ユーザー操作からワンクリックで `requestPermission` を呼び出して再開できる導線を提供。起動チェック中のレースコンディション防止スピナーを追加。
4. 仕様変更提案 `0023-browser-persistent-project-handle` を作成・承認し、`docs/specs/browser.md`（WEB-005）を更新。
5. 検証:
   - `npm run check` (TypeScript): OK
   - `npm test`: 56件全パス
   - `npx playwright test tests/browser.e2e.ts`: 11件全パス
   - `npx playwright test tests/desktop.e2e.ts`: 70件全パス
   - `python3 tools/check_workspace.py`: OK
   - `git diff --check`: OK
