# 仕様変更0026: Migration Plan compatibility impact

Status: Applied

## Affected Specifications

- Schema Migration v1
- Type Migration v1
- Table Editor / Type Editor GUI specifications

## Why

Migration PlanへReleased Compatibility reportを添付するintegrationは、retired subsystemへの依存を増やすため終了した。

## Canonical result

Table / Type Migration Planは、operation、target、affected files/occurrences、local validation diagnostics、before/after Diff、
destructive authorization、stale/lost-update safetyだけを表示する。Migrationはsource transformation correctnessを所有し、
release consumer compatibilityを推測しない。

## Approval / application

仕様変更0030のretirement decisionと同時に適用済み。旧implementation planとreview detailはGit historyに保持する。
