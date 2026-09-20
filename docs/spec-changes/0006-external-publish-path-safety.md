# 仕様変更: external publish targetのfilesystem path safetyを定義する（Specification change）

Status: Applied

> Historical audit record. Current implementation authority is the canonical specification listed below. Full proposal / review detail remains in Git history.

## Why

external publishでfilesystem-equivalent path、symlink traversal、ownership-region overlapをmutation前に判定するpath-safety contractを追加した。

## Canonical result

- [Build pipeline](../specs/build-pipeline.md) — `PUBLISH-PATH-001..010`
- [Project layout](../specs/project-layout.md) — `PROJECT-PATH-001`
- [ADR 0004](../adr/0004-file-location-has-no-semantic-meaning.md) — path is not domain identity

## Approval / application

Human maintainerがabsolute external targetを許可し、target root / ancestor symlink traversalをrejectし、publish target ownership regionをdisjointにするscopeを承認。canonical Build Pipeline / Project Layoutへ適用した。
