# 仕様変更: 複数publish targetのexecution / failure semanticsを定義する（Specification change）

Status: Applied

> Historical audit record. Current implementation authority is the canonical specification listed below. Full proposal / review detail remains in Git history.

## Why

all-target preflight後のexecution-time failureをtarget-localに閉じ、independent target継続、partial successのaggregate failure、structured per-target resultを定義した。

## Canonical result

- [Build pipeline](../specs/build-pipeline.md) — `PUBLISH-EXEC-001..005`
- Existing `PUBLISH-PATH-*` and `ARTIFACT-SET-*` remain canonical prerequisites

## Approval / application

Human maintainerがall-target preflight、independent continuation、target-local rollback、no cross-target rollback、partial-success overall Err、structured resultを承認し、Build Pipelineへ適用した。
