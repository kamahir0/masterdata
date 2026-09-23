# 仕様変更0030: Released Compatibility退役

Status: Applied

## Why

Product Simplification & Scope Cleanupの一環として、Released Compatibilityを中途半端なimpact-analysis subsetへ縮小せず、独立product capabilityとして
全面退役した。

## Canonical result

- baseline/current snapshot comparison、multi-axis report、compatibility core/app/CLI/Tauri/GUI DTOをcurrent treeから除去した。
- Migration Planはlocal transformation safetyとDiffだけを扱い、release compatibilityを推測しない。
- Table identityは`SCHEMA-TABLE-002`、MessagePack keyは`SCHEMA-KEY-001`、Referenceは`REF-*`、Build/Publishは各canonical ownerを使用する。
- receipt、Git state、generated artifact、project versionを互いのidentityやcompatibility判定へ昇格させない境界を維持する。

## Approval / application

2026-09-23 JSTのHuman decisionを適用済み。0024と0026はretired audit recordへ縮退し、実装・docs routing・testsを削除した。
