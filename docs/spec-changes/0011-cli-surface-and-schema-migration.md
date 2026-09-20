# 仕様変更: CLI surfaceとSchema Migration v1のcanonical化

Status: Applied

> Historical audit record. Current implementation authority is the canonical specification listed below. Full proposal / review detail remains in Git history.

## Why

public CLI taxonomyとSchema Migration v1のsemantic operation / recovery boundaryをimplementation前にcanonical化した。

## Canonical result

- `docs/specs/cli.md` — CLI terminology / command surface
- `docs/specs/schema-migration.md` — Migration v1 semantics
- `docs/specs/table-and-keys.md` — affected `SCHEMA-TABLE-006`
- Existing Build / Runtime Host / Project Layout specs remain their canonical owners

## Approval / application

Human maintainerがreview済み0011 proposalを明示承認。CLI / Schema Migrationのnew canonical specsとTable / Keys deltaへ適用した。runtime implementationはこのaudit recordのauthorityではない。
