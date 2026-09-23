# 仕様変更0024: Released Compatibility v1

Status: Applied

## Affected Specifications

- Table / Primary Key / Secondary Key
- Index / Reference
- Schema / Type Migration
- CLI / GUI migration plan

## Why

explicit baseline/current snapshot comparisonによるrelease compatibility analyzerは、current Masterdata productの責務を増やし、
source migration・artifact integrity・外部契約を重ねて扱うため採用されたproduct surfaceを終了した。

## Decision / canonical result

2026-09-23 JSTのHuman-selected Product Simplification & Scope Cleanupにより、Released Compatibility v1と、同機能を縮小した代替
impact analyzerのいずれもcurrent scopeへ残さない。Table identity、MessagePack key、Reference、Type System、Migration、Build/Publishの
各domain semanticsは、それぞれのcurrent canonical ownerが引き続き所有する。

## Approval / application

仕様変更0030のretirement decisionを適用済み。旧proposalの詳細はGit historyに保持し、current implementation authorityにはしない。
