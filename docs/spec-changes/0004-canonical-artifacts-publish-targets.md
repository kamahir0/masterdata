# 仕様変更: canonical build artifactsとpublish targets modelを承認・適用する（Specification change）

Status: Applied

> Historical audit record. Current implementation authority is the canonical specification listed below. Full proposal / review detail remains in Git history.

## Why

project-local canonical artifactとexternal publish destinationを分離し、buildとpublishの責務、manifest ownership、target modelをcanonical化した。

## Canonical result

- [Build pipeline](../specs/build-pipeline.md) — `BUILD-ARTIFACT-001..005`, `PUBLISH-001..010`
- [Project layout](../specs/project-layout.md) — `PROJECT-CONFIG-003`, `PROJECT-PATH-001`

## Approval / application

Human maintainerがcanonical artifact / publish targets modelを明示承認し、canonical documentsへ適用した。
