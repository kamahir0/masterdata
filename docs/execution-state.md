# Development State

Stage: implementation-ready
Candidate: none
Work base: a838bff7ccd08ad213fc56f7d13e771db2eb1d20

## Active work

- Completed: CLI仕様（[docs/specs/cli.md](specs/cli.md)）とRustコア採用提案（[0013-rust-cli-and-backend-core.md](spec-changes/0013-rust-cli-and-backend-core.md)）の整理・別pass review・承認。
- In progress: `src-tauri` へのRustコア（YAML AST、設定、スキーマ・検証、操作・クエリ、保存トランザクション、.NET Build / Publish）、Rust CLIバイナリ（`masterdata`）、およびTauriインプロセス `ProjectService` の実装。
- Remaining: `cargo test`・`npm test`・実機CLI / Desktop検証、`review-code`、commit / push、CI確認。

## Blocking findings

None.
