# 仕様変更: Migration Recovery Required時のproject-level source mutation gate

Status: Applied

> Historical audit record. Current implementation authority is the canonical specification listed below. Full proposal / review detail remains in Git history.

## Why

Migration Recovery Required中のmutation停止をTable Editor単体ではなくproject-level GUI capabilityとして全source mutation surfaceへ適用した。

## Canonical result

- `docs/gui/app-shell.md` — `GUI-SHELL-STATE-001`, `GUI-SHELL-CAPABILITY-001`
- `docs/gui/table-editor/spec.md` — related Table Editor contract
- `docs/specs/schema-migration.md` — `MIGRATION-010`

## Approval / application

Human approval: 2026-09-13。App Shellのcanonical capabilityへatomicに適用し、Table Editorから同ownerを参照する形にした。review Blockingなし。
