# 仕様変更: Masterdata YAML subsetのprimitive scalar境界（Specification change）

Status: Applied

> Historical audit record. Current implementation authority is the canonical specification listed below. Full proposal / review detail remains in Git history.

## Why

parser/library defaultから独立したMasterdata YAML scalar classificationを採用し、Primitive Typesのstrict validation contractへ接続した。

## Canonical result

- [Primitive Types](../specs/type-system/primitives.md) — `TYPE-PRIMITIVE-003`, `TYPE-PRIMITIVE-007`
- [YAML subset](../specs/yaml-subset.md) — `YAML-SUBSET-009..014`

## Approval / application

Human maintainerがYAML subset semanticsとPrimitive Typesへの接続deltaを承認。YAML subsetのApproved化後、Primitive Typesへatomicに適用した。
