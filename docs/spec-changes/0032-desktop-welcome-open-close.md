# 仕様変更: Desktop Welcome / native Project open / window close

Status: Applied

> Historical audit record. Current implementation authority is the canonical specification listed below. Full Proposed/refinement detail is preserved in Git history.

## Why

macOSでwindow closeと`Open Project`が機能せず、Project未選択をExplorer errorとして表示していたため、native lifecycleと初回入口を通常のDesktop UXへ修正した。

## Canonical result

- [GUI app shell](../gui/app-shell.md) — `GUI-SHELL-PROJECT-001`, `GUI-SHELL-LIFECYCLE-001`, `GUI-SHELL-STATE-001`
- [Project Workflow](../gui/project-workflow.md) — `GUI-PROJECT-001`, `GUI-PROJECT-002`

## Approval / application

Approval mode: Agent-autonomous。2026-09-24のHuman-selected Objectiveを根拠に、review-specでBlockingなし、Human gateなし、non-breakingかつtestableなUX correctionとして承認しcanonical ownerへ適用した。
