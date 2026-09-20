# 仕様変更: canonical artifact-set receiptでpublish-onlyの入力を検証する（Specification change）

Status: Applied

> Historical audit record. Current implementation authority is the canonical specification listed below. Full proposal / review detail remains in Git history.

## Why

publish-onlyがcurrent YAML freshnessではなく、最後に成功したcoherent canonical artifact setをreceiptで検証して配布できるcontractを採用した。

## Canonical result

- [Build pipeline](../specs/build-pipeline.md) — `ARTIFACT-SET-001..008` and related build/publish requirements
- Project identity / canonical artifact location remains owned by [Project layout](../specs/project-layout.md)

## Approval / application

Human maintainerがreceipt-valid artifact setのpublish、implicit build/freshness revalidation禁止、full buildによるcoherent receipt発行を承認し、Build Pipelineへ適用した。
