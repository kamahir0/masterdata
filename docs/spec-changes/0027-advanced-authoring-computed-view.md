# 仕様変更0027: Advanced Authoring — Computed View v1

Status: Applied

## Affected Specifications

- Schema language
- Authoring Query
- Table Overview
- Schema / Type Migration

## Why

独自expression DSLを持つauthoring-only Computed View v1は、将来のProgrammable View要件を早期に固定し、parser・type checker・evaluator・
persisted format・migration integrationを追加するため、現在のproduct scopeから除去した。

## Decision / canonical result

2026-09-23 JSTのHuman-selected cleanup decisionにより、`kind: view`、computed column、expression DSL、View CRUD、Overview projection、
expression-aware migrationをcurrent contractとして保持しない。Programmable Viewは将来priorityとしてのみ残し、現行DSLとの互換性や実装方式を
予約しない。

## Approval / application

仕様変更0031のretirement decisionを適用済み。旧proposalの詳細はGit historyに保持し、current implementation authorityにはしない。
