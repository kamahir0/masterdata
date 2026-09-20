# 仕様変更: legacy build path configurationをhard cutする（Specification change）

Status: Applied

> Historical audit record. Current implementation authority is the canonical specification listed below. Full proposal / review detail remains in Git history.

## Why

legacy `build.output` / `build.binary_output`をcompatibility aliasとして推測移行せず、canonical configuration導入時にfail-closedでrejectする方針を採用した。

## Canonical result

- [Project layout](../specs/project-layout.md) — `PROJECT-CONFIG-003..006`, `PROJECT-PATH-001`
- [Build pipeline](../specs/build-pipeline.md) — canonical artifact configuration / migration guidance

## Approval / application

Human maintainerがlegacy keysの即時reject、自動migrationなし、structured migration diagnostic、rejection時no-mutationを承認し、canonical ownersへ適用した。
