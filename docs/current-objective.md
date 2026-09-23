# Current Objective

## Role

この文書はHuman-selectedなcurrent priorityとDONE boundaryのownerである。observable semanticsはcanonical specification、resume地点は[Development State](execution-state.md)、implementation realityはcode / tests / Gitで確認する。

## Objective

**macOS Desktopの初回起動とProject入口を、OS標準操作が確実に機能し、未選択状態をエラーとして見せないWelcome experienceへ修正する。**

## Completion slices

- macOSのwindow close requestが、clean stateではwindowを閉じ、dirty stateでは既存のSave All / Don't Save / Cancel guardを経て安全に閉じる。
- `Open Project`からnative folder pickerを開き、選択したdirectoryをshared application serviceで開く。cancelは現在stateを変更しない。
- Project未選択時はExplorer errorではなく、Open / CreateとRecent Projectsへ進めるWelcome surfaceを表示する。
- explicitに選択したProjectを開けない場合は、未選択または既存Project stateを壊さず、recovery可能なdiagnosticをWelcomeまたはcurrent Project surfaceへ残す。
- focused frontend tests、Tauri compile validation、macOS実機操作、repository checks、fresh review、required remote CI reconciliationを完了する。

## Canonical requirements

- [GUI app shell](gui/app-shell.md) — `GUI-SHELL-PROJECT-001`, `GUI-SHELL-LIFECYCLE-001`, `GUI-SHELL-STATE-001`
- [Project Workflow](gui/project-workflow.md) — `GUI-PROJECT-001`, `GUI-PROJECT-002`

## Explicit non-scope

- Project discovery、Project identity、YAML semanticsのfrontendへの複製。
- filesystem browser、command palette、theme system、window layout persistenceの追加。
- Recent Projects以外のProject management機能やcloud同期。
- Desktop全体のvisual redesign。

## Audit

2026-09-24 JST、HumanはmacOS検証で、window closeが機能しないこと、初回画面の`Open Project`が機能しないこと、Project未選択時にExplorerへerrorを表示するUXが不適切であることを報告し、VS CodeのWelcome pageを参考として提示した。
