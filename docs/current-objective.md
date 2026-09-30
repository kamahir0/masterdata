# Current Objective

## Role

この文書はHuman-selectedなcurrent priorityとDONE boundaryのownerである。observable semanticsはcanonical specification、resume地点は[Development State](execution-state.md)、implementation realityはcode / tests / Gitで確認する。

## Objective

**Table header / drag handle / scroll edgeとwindow上部の密度を整え、編集面を広く、操作しやすくする。**

## Completion slices

- field名と操作の重なりを解消し、Legacyを参考にdrag handleを整える。
- scroll端のaffordanceと上部command surfaceを調整する。
- keyboard / native window操作、drag / virtualizationを保ち、focused / Desktop evidenceとrepository checkで検証する。

## Canonical requirements

- [Unified Table Editor](gui/table-editor/spec.md) `GUI-UNIFIED-005`, `GUI-UNIFIED-007`, `GUI-UNIFIED-008`
- [App shell](gui/app-shell.md) `GUI-SHELL-LAYOUT-002`, `GUI-SHELL-NAV-001`, `GUI-SHELL-LIFECYCLE-001`

## Explicit non-scope

- source semantics、Save / history、Table query、他surfaceのvisual theme再設計。
