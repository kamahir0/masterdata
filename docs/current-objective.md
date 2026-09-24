# Current Objective

## Role

この文書はHuman-selectedなcurrent priorityとDONE boundaryのownerである。observable semanticsはcanonical specification、resume地点は[Development State](execution-state.md)、implementation realityはcode / tests / Gitで確認する。

## Objective

**macOS Desktopを実操作して不自然なUXと不具合を検出し、通常のProject操作・編集・保存・終了を安全に行える状態へ改善する。**

## Completion slices

- Computer UseでProject入口、Create画面、編集、保存、Settings、終了の主要操作を検証し、再現した不具合を修正する。
- macOS標準のアプリ終了経路でも未保存変更を保護する。
- Project作成のdestination選択と中止・元画面への復帰を改善する。
- focused regression tests、Tauri compile validation、macOS実機操作、repository checks、fresh review、required remote CI reconciliationを完了する。

## Canonical requirements

- [GUI app shell](gui/app-shell.md) — `GUI-SHELL-PROJECT-001`, `GUI-SHELL-LIFECYCLE-001`, `GUI-SHELL-STATE-001`
- [Project Workflow](gui/project-workflow.md) — `GUI-PROJECT-001`, `GUI-PROJECT-002`

## Explicit non-scope

- Project discovery、Project identity、YAML semanticsのfrontendへの複製。
- filesystem browser、command palette、theme system、window layout persistenceの追加。
- Recent Projects以外のProject management機能やcloud同期。
- Desktop全体のvisual redesign。

## Audit

2026-09-24 JST、Humanは前セッションを引き継ぎ、Computer Use等によるデスクトップUXの検出・改善と、発見した不具合への自律的な対処を指示した。

2026-09-24 JST、HumanはmacOS検証で、window closeが機能しないこと、初回画面の`Open Project`が機能しないこと、Project未選択時にExplorerへerrorを表示するUXが不適切であることを報告し、VS CodeのWelcome pageを参考として提示した。
