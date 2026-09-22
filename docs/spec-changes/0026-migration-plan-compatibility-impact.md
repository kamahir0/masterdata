# 仕様変更: Migration Plan compatibility impact

Status: Applied

## Why

Schema / Type MigrationのPlanはsource Diffとmigration-local validationを提示できるが、generated C#やreleased artifactへのimpactは別のCompatibility画面を開かないと確認できなかった。Released Compatibility v1がImplementedとなったため、同じreviewed before/candidate snapshotをread-only analyzerへ渡してPlan reviewへimpactを添付する。

## Adopted decision

- Table / Type Migration Planはbefore snapshotと同一Planのtransformed candidateからReleased Compatibility reportを計算する。
- compatibility reportは補助impact情報でありMigration success gateではない。
- unrelated invalid source等でcompatibility analysisが成立しない場合はdiagnosticを返し、Migration closureがvalidならPlan生成を継続する。
- frontendはcompatibility classificationを再実装せずshared reportを表示する。
- analyzerはread-onlyのままでMigration Apply、Build、Publish、Git、version変更を開始しない。

## Affected Specifications

- [Table Editor](../gui/table-editor/spec.md) — GUI-TABLE-INT-004
- [Type Editor](../gui/type-editor/spec.md) — GUI-TYPE-INT-004
- [Released Compatibility v1](../specs/compatibility/released-compatibility.md) — COMPAT-RELEASED-001 / 008 / 012 を再利用し、意味は変更しない。

## Approval / application

2026-09-22 JST。Current Objective「Schema Evolution & Migrationをproduction-readyにする」内のadditive authoring integrationであり、existing read-only analyzerとMigration closure boundaryからbehaviorが一意に決まる。public persisted formatやbreaking APIを変更しないためautonomous approval/applicationとする。

Implementation evidenceはcode / tests / Gitをcurrent authorityとして確認する。
