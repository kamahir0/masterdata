# 仕様変更: Existing record key field edit

Status: Applied

> Historical audit record. Current implementation authority is the canonical specification listed below. Full proposal / review detail remains in Git history.

## Why

existing recordのPrimary / Secondary Key構成fieldを通常のdirect single-cell editへ統合しつつ、existing-key batch mutation禁止とsource provenance / Save lifecycleを維持するP4-A contractを採用した。

## Canonical result

- [Source Record Edit](../specs/source-edit.md) — `SOURCE-EDIT-017`
- [Data Editor](../gui/data-editor/spec.md) — `GUI-DATA-STATE-001`, `GUI-DATA-EDIT-001`
- [Authoring Batch](../specs/authoring-batch.md) — `AUTHORING-BATCH-001` unchanged
- [Grid Authoring](../gui/data-editor/grid-authoring.md) — batch mutation contract unchanged

## Approval / application

Human approval: 2026-09-20 JST。Canonical application commit: `5ce8e0e8154e3d4f152c883f88bac34537d89ca6`。Implementation authorityはcanonical Approved specificationsのみ。
