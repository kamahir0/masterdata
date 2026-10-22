# Browserのプロジェクトハンドル永続化と権限再開

Status: Applied

Basis: 2026-10-23 Human request「ブラウザ版は、開くたびに毎回権限許可が必要になってしまい、これは技術的にどうしようもないことか」「実装して」に基づく仕様変更提案。

## 課題

現状のブラウザ版（`docs/specs/browser.md` WEB-005）では「handleは初期版では永続化しない。再開はフォルダの再選択と明示操作から行う」と規定されていたため、ページをリロードしたり閉じたりするたびに、ユーザーは毎回フォルダ選択ピッカー（`showDirectoryPicker`）から対象フォルダを探して選び直す必要があった。

Chromium系ブラウザの File System Access API では、`FileSystemDirectoryHandle` を IndexedDB に保存でき、Chrome 122+ の Persistent Permissions（永続権限）により権限が維持されている場合は起動時にサイレントに再開可能である。また権限確認が必要な場合でも、保存されたハンドルに対してユーザー操作から `requestPermission` を呼ぶことで、フォルダピッカーを開き直すことなくワンクリックで再開できる。

## 提案

1. **IndexedDB へのフォルダハンドル永続化**:
   - ブラウザ版でプロジェクトのロード（`open()` または再開）に成功した際、ルートの `FileSystemDirectoryHandle` を IndexedDB（DB `masterdata`, object store `handles`, key `recent-root`）に保存する。
   - 別のプロジェクトを明示的に開いた場合は、保存されたハンドルを新しいもので上書き更新する。

2. **起動時（`initial()`）の自動再開と権限判定**:
   - アプリ起動時の `window.desktop.initial()` において、IndexedDB から保存済みハンドルを取得する。
   - 保存済みハンドルが存在し、かつ `queryPermission({ mode: "readwrite" }) === "granted"` の場合:
     - ユーザー操作なしでそのまま `load(handle)` を実行し、直近プロジェクトを自動復元する。
   - 保存済みハンドルが存在するが、権限が `"granted"` でない場合（`"prompt"` または `"denied"`）:
     - バックグラウンドでの `requestPermission()` 呼び出しはブラウザのセキュリティ要件（User Gesture）に違反するため行わない。
     - `initial()` は `null` を返すが、前回のプロジェクト情報（ハンドルおよび `handle.name`）を `browserStatus` 経由で保持・通知する。
   - 保存済みハンドルからの読込時に `NotFoundError` や破損を検出した場合は、保存済みハンドルを安全に消去して通常スタート画面へフォールバックする。

3. **スタート画面（Welcome画面）の再開UX**:
   - 前回開いたプロジェクト情報がある場合、スタート画面に「前回のプロジェクトを開く（フォルダ名）」ボタンをプライマリアクションとして提示する。
   - このボタンをクリックした際、ユーザー操作を起点として `handle.requestPermission({ mode: "readwrite" })` を要求する。
   - 許可されれば即座に `load(handle)` を実行してプロジェクトを開く。
   - 拒否・取消された場合は入力を妨げずスタート画面に留まる。
   - 併設の「別のプロジェクトを開く」ボタンから、いつでも別のローカルフォルダを選択できる。

4. **セキュリティと互換性の維持**:
   - ユーザーが選択したフォルダの境界（`sources/` と `masterdata.toml`）のみを操作する既存の安全性規則（WEB-001 / WEB-005）を完全に維持する。
   - Desktop（Tauri）版および CLI には一切の影響を与えない。

## Acceptance / validation

1. **自動再開**: `queryPermission` が `"granted"` を返す環境（永続権限付与済み）でブラウザ版を起動した際、`initial()` により前回のプロジェクトが自動でロードされること。
2. **ワンクリック再開**: 権限が `"prompt"` の環境で起動した際、スタート画面に「前回のプロジェクトを開く（プロジェクト名）」が表示され、ボタン押下により `requestPermission` が呼ばれてプロジェクトがロードされること。
3. **新規選択による更新**: 「別のプロジェクトを開く」から新しいフォルダを選択した場合、IndexedDB のハンドルが新しいフォルダに更新されること。
4. **破損・削除への耐性**: 保存されたフォルダが削除または移動されていた場合、クラッシュせず保存ハンドルがクリアされ、通常のスタート画面へフォールバックすること。
5. **テストと検証**: 単体テスト・結合テスト（IndexedDB および FileSystemHandle の mock / focused tests）が全件パスし、コード規約・workspace check を満たすこと。

## Canonical owner

- [Browser Authoring](../specs/browser.md)（WEB-005 を更新）

## Review

別passでchallenge。
- Human intentへの適合: ユーザーの「開くたびの権限許可」に対する課題認識に直接応え、永続権限環境でのサイレント復帰と、通常環境でのワンクリック再開UXを提供している。
- 状態遷移と安全性: `queryPermission` / `requestPermission` のライフサイクル、User Gesture要件の遵守、ハンドル破損・NotFound時のIndexedDBクリアとフォールバック、別フォルダ選択時の更新処理が漏れなく網羅されている。
- 境界と互換性: 権限範囲はユーザー明示選択フォルダ内に閉じており、Desktop / CLIの挙動には影響しない。
- Blocking Issues: None
- Non-blocking Issues: None
- Questions: None
- Approved as Proposed: Yes
- Autonomous approval eligible: Yes（選択済みObjective内、境界破壊なし、安全性の向上）
- Human gate: None

Approval mode: Agent-autonomous
Objective / request basis: 2026-10-23 Human requestに基づく直近プロジェクトハンドルの永続化と権限再開機能の実装委任。review passの判定後、orchestrationが承認。
