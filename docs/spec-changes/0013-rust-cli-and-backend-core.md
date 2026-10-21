# Rust CLIとネイティブバックエンドコアの採用

Status: Applied

Approval mode: Agent-autonomous

## Basis

2026-10-21 Human Question / Decision: 「コアがrustなので、やっぱりCLIはrustで作った？」「そもそもGUI側的にもそれって一応アリなのか？バックエンドとフロントエンドの境界を曖昧にするようなのもアリなのか」「パフォーマンスやGUIの操作感はどっちがよい？」「rust採用がよさそう。」

## Behavior / Agent Decision

- **GUI編集セッションとRustコアの責務分離**:
  - GUIのセル入力、ドラッグ並替え、未完成入力の保持、Undo / Redoは0msの同期レスポンスを維持するため、WebView内のimmutable draftセッション（`src/core/`）が引き続き所有する。キー入力やドラッグごとにIPC往復を行わない。
  - CLI実行バイナリ（`masterdata`）およびDesktopのネイティブバックエンド操作（`open` / `create` / `sample` / `save` / `build` / `publish` / `reveal`）は、Rustコア（`src-tauri`）で共有実装する。
- **単一ネイティブCLIとサイドカー廃止**:
  - `masterdata` CLIは `src-tauri` のRustバイナリターゲットとしてビルドし、Node.jsランタイム不要で単一バイナリとして高速起動できるようにする。
  - Desktopアプリ（`masterdata-desktop`）はTauriプロセス内で同一のRustコア（`ProjectService`）を直接実行し、Node SEAの外部サイドカープロセス（`masterdata-host`）を不要にする。
- **データ・フォーマット・安全性の完全維持**:
  - `masterdata.toml` の検証、ソースのみ保存時のバイト単位保持、設定変更時の未知キー保持をRustで実装する。
  - YAML 1.2のAST保持（コメント、クオート種別、Mapキー順、`fields` / `custom.fields` / `members` / `records` の配列順、CRLF / LF改行、64bit整数 `long` / `ulong` 精度、`$tags`、空コレクション `{}` / `[]` と非空2スペースblock正規化）および構文エラーファイルの無変更保持をRustで実装する。
  - 保存トランザクション（事前・直前の外部変更検査、`.masterdata/transactions/<uuid>`、一時ファイル置換、失敗時ロールバック）と `.NET` / MasterMemory Build・reload検証・Publish（`.masterdata/latest-build.json` のダイジェスト検証を含む）をRustで実装する。

## Acceptance / validation

- Rust製 `masterdata` バイナリ単体で `init`、`inspect`、`validate`、`config`、`table`、`type`、`record`、`rename`、`source`、`fix`、`tx`、`build`、`publish` の全操作が動作し、コメント・クオート・配列順・CRLF・64bit `ulong`（`18446744073709551615`）・`$tags` を保持すること。
- 一部YAML破損・意味エラー混在時の通過規則と `--strict` / `validate` / `build` / `publish` の品質ゲート、複数ソース表での `--source` 必須規則、古いBuildのPublish拒否がRustバイナリで成立すること。
- Desktopアプリのネイティブ操作（`open` / `create` / `sample` / `save` / `build` / `publish`）がTauriプロセス内のRustコアで動作し、`cargo test`・`npm test`・UI回帰・実機起動確認を通過すること。

## Review

別passでGUIの即時操作感（WebView内draft維持）とRustバックエンド/CLI統合の境界、YAML AST（コメント・クオート・CRLF・64bit整数・配列順）の往復保持、トランザクション保存と `.NET` Build / Publishの安全性をchallenge reviewした。Blocking / Questions: None。Approved as Proposed: Yes。Autonomous approval eligible: Yes（2026-10-21 Humanの「rust採用がよさそう。」の決定に基づく実装アーキテクチャおよびCLI提供形態の具体化であり、既存データ形式と保護規則を維持する）。Human gate: None。
