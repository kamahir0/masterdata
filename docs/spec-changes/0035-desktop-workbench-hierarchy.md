# Desktop workbenchの情報階層

Status: Applied

## Why / Adopted Decision

Desktopの定常画面で全体commandと詳細操作が並び、Table / Typeを作業対象として辿りにくかった。Human-selected UI改善Objectiveに基づき、shared workspace metadataによるlogical navigationと、必要時に開く詳細操作を採用した。source/config/CLIの互換性は不変。

一次資料から、対象を見て選べるnavigation（[NN/g: recognition rather than recall](https://www.nngroup.com/articles/ten-usability-heuristics/)）、詳細設定を必要時に開く構成（[NN/g: progressive disclosure](https://www.nngroup.com/articles/progressive-disclosure/)）、階層と開閉が明瞭なsidebar（[Apple HIG: Sidebars](https://developer.apple.com/design/human-interface-guidelines/sidebars)）、対象の文脈で実行できるcommand（[Microsoft: Commanding basics](https://learn.microsoft.com/en-us/windows/apps/design/basics/commanding-basics)）を判断軸とした。Desktop実機では上部command列、Data query/batch列、0件のProblems、Settingsの並列formが同時表示の主因だった。

## Canonical result

- [GUI app shell](../gui/app-shell.md) — `GUI-SHELL-NAV-001`
- [Project Workflow](../gui/project-workflow.md) — `GUI-PROJECT-001`
- [Workspace Explorer](../gui/explorer/spec.md) — `GUI-EXPLORER-001`
- [Data Editor](../gui/data-editor/spec.md) — `GUI-DATA-LAYOUT-006`
- [Table Overview](../gui/table-overview/spec.md) — `GUI-OVERVIEW-004`
- [Project Settings](../gui/project-settings/spec.md) — `GUI-SETTINGS-001`

## Approval Record

Approval mode: Agent-autonomous。2026-09-24、Human-selected Desktop UX改善Objectiveをbasisとしてreview-specを別passで実施。Blocking Issues / Non-blocking Issues / Questions: None identified。Approved as Proposed: Yes。Eligible: Yes。Human gate: None。Intent、既存GUI契約、source authority、testability、compatibility、documentation ownershipを確認しcanonical ownerへ適用した。詳細proposalはGit historyで復元する。
