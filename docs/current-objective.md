# Current Objective

## Role

この文書はHuman-selectedなcurrent priorityとwork package boundaryの唯一のownerである。
Approved behaviorはcanonical specification、現在のStageは[Development State](execution-state.md)、implementation realityはcode / tests / Gitで確認する。

## Objective

現在のHuman priorityは、**ApprovedとなったDesktop制作v1（P1–P3）をまとまったwork packageとして実装し、日常制作をDesktop GUIだけで完遂できるcandidateを作る**ことである。

2026-09-18、Human maintainerは仕様変更0016–0018を一括Approvalし、44の新Requirementをcanonical ownerへ適用した。Stageは`implementation-ready`であり、次の作業はApproved contractからの実装である。

## Why now

Complex Value Authoring v1までで型・Table・record authoringとBuild / Publishの基盤は成立した。次は個別機能追加ではなく、Project作成から編集・確認・Build・Publishまでの日常workflowを一つの完成単位として接続する。

## Completion boundary

Desktop制作v1の完了候補は、少なくとも次を満たす。

- 新規ProjectをGUIから安全に作成し、型・Table・Data file・recordを作成できる。
- Data Editorでsearch/filter/sort、scalar range paste/fill、preview、Undo/Redo、Tag編集をsource-preservingに行える。
- Table Overviewで分割fileを横断して保存済みsnapshotとProfile selectionを確認し、source occurrenceへ安全に戻れる。
- Project SettingsでProfile / Publish targetをlossless TOML editとして変更・保存できる。
- Buildは保存済みsource/configだけを使用し、明示Profileでcanonical artifact setを生成できる。
- Publishはreceipt validationとpreview / all-target preflightを経てC# / binary targetへ配置できる。
- external conflict、stale preview、Save All部分失敗、Migration Recovery Required、Publish部分失敗でlocal inputやunmanaged fileを誤って失わない。
- focused tests、repository required checks、Desktop実機制作scenarioを通す。
- 10万record（分割file）、20列Table、1万cell pasteの固定生成inputについてload/query/preview/validation時間とpeak memoryを測定し、環境情報とともにevidenceを残す。

完成時はcandidate SHAをDevelopment Stateへ記録し、独立verificationへ進める。

## Canonical implementation inputs

### P1 — 日常編集

- [Authoring Batch](specs/authoring-batch.md)
- [Authoring Query / Overview](specs/authoring-query.md)
- [Data Editor Grid Authoring](gui/data-editor/grid-authoring.md)
- [Typed Migration Initializer](gui/typed-initializer.md)

### P2 — Workspace・Tag・設定

- [Source Tag Edit](specs/source-tag-edit.md)
- [Data Editor Tag Authoring](gui/data-editor/tag-authoring.md)
- [Table Overview](gui/table-overview/spec.md)
- [Project Config Edit](specs/project-config-edit.md)
- [Project Settings](gui/project-settings/spec.md)

### P3 — Project入口・Build / Publish

- [Project Initialization](specs/project-init.md)
- [Project Workflow](gui/project-workflow.md)
- [Build Request / Publish Preview](specs/build-request-preview.md)
- [Build / Publish GUI](gui/build-publish/spec.md)

既存のSource Edit、Record Mutation、Build Selection、Build pipeline、Migration、Runtime host等のApproved ownerも引き続き適用する。仕様変更0016–0018はApplied audit recordであり、implementation authorityではない。

## Implementation order

依存を壊さず一つのcandidateへ収束させるため、shared semantics / application service → Tauri adapter → React surface → cross-surface lifecycle → end-to-end / performance evidenceの順を基本とする。内部task分割はこの順序を満たす範囲でimplementation側に委ねる。

## Explicit non-scope

- P4: existing key編集、source file rename/move等。
- P5: expression / computed / programmable view。
- P6: Referenceの完成、Web authoring完成。
- Git stage / commit / pushの自動化。
- arbitrary YAML/TOML raw editorを通常制作経路にすること。
- product latency SLAやrecord上限を今回の測定だけから発明すること。
- Approved specificationにないobservable behaviorをimplementation convenienceで追加すること。

## Relevant authorities

- [Authoring system v1 RFC](rfcs/0008-authoring-system-v1.md)
- [0016 Applied record](spec-changes/0016-desktop-daily-editing.md)
- [0017 Applied record](spec-changes/0017-desktop-workspace-settings.md)
- [0018 Applied record](spec-changes/0018-desktop-build-delivery.md)
- [Specification workflow](contributing/specification-workflow.md)
- [Development workflow](execution-workflow.md)
