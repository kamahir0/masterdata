# GUI仕様

GUI behaviorがuserのobservable workflowに影響する場合、それは仕様化の対象である。GUI specはdomain specと同じ
`Draft`、`Proposed`、`Approved`、`Implemented`、`Deprecated` lifecycleを使用する。Requirement IDには `GUI-`
prefixを付け、その後にsurfaceとstableな3桁のnumberを続ける。

GUI specでは `masterdata-core` のsemanticsを重複させず、behaviorを記述する。layout、state、selection、editing、
validation、focus、keyboardとmouse interaction、loading、empty/error/disabled state、unsaved change、build-in-progress
behaviorを扱ってよい。shared domain operationに対するadapter boundaryはTauri commandである。

## Surface index

- [GUI app shell](app-shell.md) — Desktop shellとshared application boundary（Approved）
- [Workspace Explorer](explorer/spec.md) — 左ペインのfile / folder navigationとtyped editor selection（Approved）
- [Data Editor](data-editor/spec.md) — record data YAMLのspreadsheet型editorとfile単位dirty / Save（Approved）
- [Data Editor Grid Authoring](data-editor/grid-authoring.md) — range selection、paste/fill preview、query composition、Undo/Redo（Approved）
- [Data Editor Tag Authoring](data-editor/tag-authoring.md) — Record Tagのtyped authoring（Approved）
- [Source Creation](source-creation/spec.md) — Explorerからのfolder / source artifact creation flow（Approved）
- [Table Editor](table-editor/spec.md) — Schema Migration v1のplan / diff / Add・Rename・Drop Field GUI（Approved）
- [Type Editor](type-editor/spec.md) — Type Migration v1のPlan / Diff / Value Object・Enum・Flags・Custom Type編集GUI（Approved）
- [Typed Migration Initializer](typed-initializer.md) — Table / Type Add operationのshared schema-aware initializer（Approved）
- [Table Overview](table-overview/spec.md) — 保存済みsnapshotのTable横断viewとProfile preview（Approved）
- [Project Settings](project-settings/spec.md) — Profile / Publish targetのtyped config editing（Approved）
- [Project Workflow](project-workflow.md) — Create Project、logical navigation、Recent Projects（Approved）
- [Build / Publish](build-publish/spec.md) — saved-input Build、Publish preview / confirmation / result（Approved）
- [Color Theme](color-theme/spec.md) — 表示テーマ（Light / Dark / System）と永続化（Approved）

Desktop制作v1（P1–P3）のdomain/application ownerへの導線は[canonical package index](../specs/desktop-authoring-v1/README.md)を参照する。

新しいsurface specificationは [_template.md](_template.md) から始める。大きなsurfaceでは、visual artifactを
specificationの隣に置く。

```text
docs/gui/table-editor/
├── spec.md
├── default.png
├── validation-error.png
└── empty-state.png
```

specに列挙するすべてのimageは、`Normative`（review済みvisualがacceptance contractの一部で、対象stateとviewportを
記述する）または `Reference-only`（behaviorを追加しないdesign aid）のいずれかにlabelしなければならない（MUST）。
reference imageがwritten normative requirementを上書きすることはない。imageを変更した場合は、影響するGUI
Requirement IDをreviewし、compatibility/acceptance noteを必要に応じて更新する。

各canonical GUI ruleは、1つのsurface specificationに1つだけ置く。observableなlayout、state、selection、editing、
keyboard、focus、validation、loading、empty/error/disabled、unsaved-change、build-progress behaviorには、
`GUI-DATA-EDIT-001` のようなRequirement IDを使用する。shared domain meaningは `masterdata-core` に残し、GUI specは
userのobservableなadapter behaviorとTauriとのboundaryだけを記述する。
