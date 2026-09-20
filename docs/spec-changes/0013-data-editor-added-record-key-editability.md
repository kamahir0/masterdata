# 仕様変更: Added record draftのkey field editability

Status: Applied

> Historical audit record. Current implementation authority is the canonical specification listed below. Full proposal / review detail remains in Git history.

## Why

existing record key mutationを広げず、base snapshotに存在しないAdded record draftだけPrimary / Secondary Key fieldを初回入力できるようData Editor contractを調整した。

## Canonical result

- `docs/gui/data-editor/spec.md` — `GUI-DATA-STATE-001`
- `docs/gui/data-editor/record-mutation.md` — Added record mutation contract

## Approval / application

Human approval: 2026-09-12。Canonical application commit: `4e36a20c79b3bc570885bfded674313707d5bd99`。
