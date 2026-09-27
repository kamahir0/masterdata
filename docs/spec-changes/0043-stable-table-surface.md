# 仕様変更 0043: Stable Table authoring surface

Status: Applied

## Why / adopted decision

schema draftやquery補助面がeditor上部へ挿入され、作業中のgridを動かしていた。通常のauthoring state変化ではgridのvertical originを固定し、schema状態を既存header、query / batchを重畳する補助面に移す。

## Canonical result

`docs/gui/table-editor/spec.md` — `GUI-UNIFIED-006`。source、Save対象、Undo単位、Application APIは変更しない。

## Approval provenance

Approval mode: Agent-autonomous。Human-selected Daily Authoring Objectiveに基づく。review-specのfresh challengeでBlockingなし、Human gateなし、非breakingと判定した。canonical applicationとimplementationは本artifactを含むGit履歴で追跡する。
