# Desktop制作v1 canonical specification index

このindexは2026-09-18にApproved / AppliedとなったDesktop制作v1（P1–P3）の新規domain/application specificationへの導線である。Requirementのauthorityは各link先にあり、このindexはnormative ruleを複製しない。

## P1 — 日常編集

- [Authoring Batch](../authoring-batch.md) — scalar range mutation / clipboard codec
- [Authoring Query](../authoring-query.md) — search / filter / sort / saved Overview snapshot

## P2 — Workspace・Tag・設定

- [Source Tag Edit](../source-tag-edit.md) — `$tags` source-preserving mutation
- [Project Config Edit](../project-config-edit.md) — Profile / Publish target lossless TOML editing

## P3 — Project入口・Build / Publish

- [Project Initialization](../project-init.md) — GUI Create Project向けshared initialization safety
- [Build Request / Publish Preview](../build-request-preview.md) — captured saved-input Build requestとread-only Publish preview

## GUI owners

対応するuser-visible behaviorは[GUI specification index](../../gui/README.md)を参照する。Data Editor Grid / Tag、Table Overview、Project Settings、Project Workflow、Build / Publish、Typed Migration InitializerがDesktop制作v1 packageに含まれる。

## Applied records

- [0016 — P1 日常編集](../../spec-changes/0016-desktop-daily-editing.md)
- [0017 — P2 Workspace・Tag・設定](../../spec-changes/0017-desktop-workspace-settings.md)
- [0018 — P3 Project入口・Build / Publish](../../spec-changes/0018-desktop-build-delivery.md)

Applied recordはapproval / migration historyでありimplementation authorityではない。実装は各Approved canonical ownerのRequirement IDから行う。
