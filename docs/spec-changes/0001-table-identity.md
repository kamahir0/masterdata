# 仕様変更: 現在のtable identity boundaryを採用（Specification change）

Status: Applied

> Historical audit record. Current implementation authority is the canonical specification listed below. Full proposal / review detail remains in Git history.

## Why

current scaffoldのtable identity boundaryを正式採用し、file locationやgenerated nameではなくcanonical table identity ownerへ集約した。

## Canonical result

- [Table / Keys](../specs/table-and-keys.md) — `SCHEMA-TABLE-002`
- [Schema language](../specs/schema-language.md) — current scaffold identity routing
- [Terminology](../product/terminology.md) — glossary routing

## Approval / application

Human decisionによりcurrent-scaffold directionを承認。canonical documentationへatomicに適用し、関連RFCをAcceptedへ移行した。
