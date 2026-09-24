# Current Objective

## Role

この文書はHuman-selectedなcurrent priorityとDONE boundaryのownerである。observable semanticsはcanonical specification、resume地点は[Development State](execution-state.md)、implementation realityはcode / tests / Gitで確認する。

## Objective

**Desktopの主要な作業対象をProject / Table / Typeとして辿れるようにし、定常画面に同時表示する操作と情報を整理して、編集・確認・Buildを迷わず続けられるUIへ改善する。**

## Completion slices

- UI設計の一次資料とmacOS実機操作から現行画面の情報階層・操作導線を評価し、このProjectのsource authorityと安全な編集契約に沿う設計へ反映する。
- Project / Table / Type中心のnavigation、必要時に開く詳細操作、Project作成後の次操作導線をDesktopへ実装する。
- dirty buffer、Overviewの保存済みsnapshot、Migration / Publish確認、diagnostics、keyboard navigationを維持する。
- focused regression tests、Tauri compile validation、macOS実機操作、repository checks、fresh review、required remote CI reconciliationを完了する。

## Canonical requirements

- [GUI app shell](gui/app-shell.md) — `GUI-SHELL-LAYOUT-001`, `GUI-SHELL-LAYOUT-002`, `GUI-SHELL-NAV-001`, `GUI-SHELL-STATE-001`
- [Project Workflow](gui/project-workflow.md) — `GUI-PROJECT-001`, `GUI-PROJECT-002`
- [Workspace Explorer](gui/explorer/spec.md) — `GUI-EXPLORER-001`, `GUI-EXPLORER-INT-001`
- [Data Editor](gui/data-editor/spec.md) — `GUI-DATA-LAYOUT-001`, `GUI-DATA-LAYOUT-004`, `GUI-DATA-LAYOUT-006`
- [Table Overview](gui/table-overview/spec.md) — `GUI-OVERVIEW-001`, `GUI-OVERVIEW-004`
- [Project Settings](gui/project-settings/spec.md) — `GUI-SETTINGS-001`

## Explicit non-scope

- Project identity、YAML / Table / Type semantics、Build / Publish結果の変更。
- frontendによるfilesystem discoveryまたはpath / filenameからのdomain identity推論。
- source format、public API、CLI contract、Git workflowの変更。
- UI frameworkの置換やDesktop全体のvisual redesign。
