# Current Objective

## Objective

2026-10-21 Human requestに基づき、GUIとYAMLデータ構造・プロジェクト概念（`masterdata.toml`、`sources/`、`.masterdata/`）・検証・Build / Publishを共有するAIエージェントファーストのCLIを策定し、GUIの即時編集セッション（WebView内draft）の操作感を維持したまま、CLIバイナリ（`masterdata`）とネイティブバックエンドコアをRustで実装する。

## Completion boundary

CLI仕様とRustコア採用の別pass review・承認、Rustコア（YAML AST / コメント / クオート / CRLF / 64bit整数 / 配列順保持、`masterdata.toml`、スキーマ・型・レコード操作、横断リネーム、トランザクション保存、.NET Build / Publish）およびRust製 `masterdata` CLI・Tauriインプロセスバックエンドの実装、focused validation、実機・代表作業確認、self-review、通常commit / push、CI確認。

## Non-scope

GUIの日常セル入力・ドラッグ・Undo/RedoごとのIPC往復化、SQLクエリエンジンの同梱、旧YAMLプロジェクト設定の自動移行、公開release、Unity起動。
