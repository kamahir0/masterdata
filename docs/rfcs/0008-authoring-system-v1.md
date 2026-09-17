# RFC: 日常の制作を完結するMasterdata authoring system v1

Status: Accepted

## Purpose

このRFCはDesktopで「定義する → 探す → まとめて編集する → 確認する → Unityへ渡す」を完結させるAuthoring system v1の方向選択とtrade-offを記録するrationale ownerである。implementation authorityはApproved canonical specificationにある。

## Decision history

2026-09-16、HumanはOption Bの**Desktop制作v1（P1–P3）**を選択した。中心方針はfile単位authoring、保存前Undo、scalar一括入力、保存済みsnapshotを使うOverview / Build、明示previewを経るPublishである。計算列、existing key/source移動、Reference完成、Web完成は後段とした。

2026-09-18、Human maintainerは詳細仕様変更[0016](../spec-changes/0016-desktop-daily-editing.md)・[0017](../spec-changes/0017-desktop-workspace-settings.md)・[0018](../spec-changes/0018-desktop-build-delivery.md)を一括Approvalした。3 changeは`Applied`となり、詳細behaviorはcanonical ownerへ移った。

## Options considered

- **A: 単機能を順次追加** — changeは小さいが、制作workflowの断点が残る。
- **B: Desktop制作v1をまとめる** — 既存のauthoring / Build / Publish基盤を一つの日常workflowへ接続する。採用。
- **C: Web・Reference・programmable runtimeまで同時完成** — scopeと相互依存が大きく、P1–P3のcompletion boundaryを遠ざけるため今回非採用。

## Preserved architecture

- YAMLをSource of Truthとし、編集対象外source textをpreserveする。
- CLI / GUIはshared Rust application/core semanticsを使用し、GUIからCLI subprocessでdomain処理しない。
- .NET/MasterMemory責務は既存bridgeへ委譲する。
- path、Table identity、PK、source occurrenceを混同しない。
- validation errorだけでsafe source Saveを禁止しない。
- Buildは保存済みinputからcanonical artifact setを作り、Publishとは別operationとする。
- Publishはreceipt / path safety / all-target preflight / target-local failure contractを維持する。

## Canonical package

P1のauthorityはAuthoring Batch / Query、Data Editor Grid Authoring、Typed Migration Initializer。
P2のauthorityはSource Tag Edit、Table Overview、Project Config Editと対応GUI specifications。
P3のauthorityはProject Initialization、Build Request / Publish Previewと対応GUI specifications。

正確なowner一覧とcompletion boundaryは[Current Objective](../current-objective.md)およびApplied record 0016–0018を参照する。本RFCの提案文やoption説明をrequirementの第二ownerとして使用してはならない。

## Deferred

P4 existing key / source move、P5 expression / computed view、P6 Reference / Web、およびGit automationは別のspecification changeで扱う。今回のperformance workは測定evidenceを要求するが、未承認のlatency SLAや製品上限を導入しない。
