# Current Objective

## Role

この文書はHuman-selectedなcurrent priorityとDONE boundaryのownerである。observable semanticsはcanonical specification、resume地点は[Development State](execution-state.md)、implementation realityはcode / tests / Gitで確認する。

## Objective

**Desktopの左ペインをsource root配下の素直なfile treeへ戻し、Data / Schema / Typeの編集画面を密度が高くキーボードでも連続編集しやすい構成へ作り直す。**

## Completion slices

- VS Code ExplorerとLegacy Editorの実機観察で確認した配置・編集操作を、このProjectのsource authorityと安全な編集契約に沿う仕様へ反映する。
- 左ペインをconfigured source rootのfile / folder treeだけにし、file操作とkeyboard navigationを提供する。Project操作は上部メニュー、Table Overviewは編集文脈から開く。
- Dataは一定の行高を持つgridと選択時だけ開く複合値editorへ、Schema / Typeは行起点の操作と一時的なPlan / Diff / Apply画面へ整理する。
- dirty buffer、Migration / Publish確認、diagnosticsを保持し、focused tests、Tauri compile validation、macOS実機操作、repository checks、fresh review、required remote CI reconciliationを完了する。

## Canonical requirements

- [GUI app shell](gui/app-shell.md) — `GUI-SHELL-NAV-001`, `GUI-SHELL-LAYOUT-001`, `GUI-SHELL-STATE-001`
- [Workspace Explorer](gui/explorer/spec.md) — `GUI-EXPLORER-001`, `GUI-EXPLORER-INT-001`
- [Data Editor](gui/data-editor/spec.md) — `GUI-DATA-LAYOUT-007`, `GUI-DATA-KEY-002`, `GUI-DATA-EDIT-003`
- [Table Editor](gui/table-editor/spec.md) — `GUI-TABLE-LAYOUT-005`, `GUI-TABLE-INT-004`
- [Type Editor](gui/type-editor/spec.md) — `GUI-TYPE-LAYOUT-004`, `GUI-TYPE-INT-004`

## Explicit non-scope

- Project identity、YAML / Table / Type semantics、Build / Publish結果の変更。
- frontendによるfilesystem discoveryまたはpath / filenameからのdomain identity推論。
- source format、public API、CLI contract、Git workflowの変更。
- UI frameworkの置換。
