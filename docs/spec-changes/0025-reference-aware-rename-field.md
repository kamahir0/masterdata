# 仕様変更: Reference-aware RenameField

Status: Applied

## Why

Schema MigrationのRenameFieldはPrimary / Secondary Keyまではresolved dependencyとして追随できた一方、Reference source/target componentへ到達するとfail closedしていた。Reference v1がImplementedとなり、explicit RenameField intentから安全に同一logical fieldへのReference componentだけを追随できるため、raw YAML手編集へ戻る断点を解消する。

## Adopted decision

- RenameFieldのtargetは従来どおりlogical Table identity + current field nameで解決する。
- 同じfieldへresolveするReference source componentとReference target componentはnew field nameへsource-preservingに追随する。
- Reference domain `name`、`csharpName`、target Table、component orderその他のrelationship semanticsは変更しない。
- source locationが安全に特定できない場合はfail closedし、whole-file reserializeやheuristic renameを行わない。
- DropFieldはreplacementを推測できないため、Reference dependencyがあれば引き続きfail closedする。
- stable Field ID、MessagePack key identity、rename lineageを導入しない。

## Canonical owners

- [Schema Migration v1](../specs/schema-migration.md) — MIGRATION-007 / MIGRATION-008
- [Table Editor](../gui/table-editor/spec.md) — GUI-TABLE-INT-002
- [Index / Reference](../specs/index-and-reference.md) — Reference relationship semantics

## Approval / application

2026-09-22 JST。Current Objective「Schema Evolution & Migrationをproduction-readyにする」内のadditiveなsafe migration coverageであり、Human gate条件に該当しない。Reference v1のApproved identityとsource-preserving Migration contractからbehaviorが一意に決まり、DropField destructive semanticsは変更しないためautonomous review/applicationとする。

Implementation evidenceはcode / tests / Gitをcurrent authorityとして確認する。
