# 仕様変更: Field IDをMessagePack keyへ置き換える（Specification change）

Status: Applied

> Historical audit record. Current implementation authority is the canonical specification listed below. Full proposal / review detail remains in Git history.

## Why

MessagePackとは独立したpersistent numeric Field ID modelを廃止し、`key`をserialization layoutだけのexplicit integer keyとして扱うmodelへ移行した。

## Canonical result

- [Custom Types](../specs/type-system/custom-types.md) — affected `SCHEMA-CUSTOM-*`
- [Table / Keys](../specs/table-and-keys.md) — `SCHEMA-KEY-001` and persisted field model

## Approval / application

Human maintainerがEnum / Flags、Table / KeysのApproved化とあわせてdeltaを承認。Field Identity、Custom Type、schema-language、compatibility routingへatomicに適用した。
