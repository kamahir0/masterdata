# GUI仕様

GUI behaviorがuserのobservable workflowに影響する場合、それは仕様化の対象である。GUI specはdomain specと同じ
`Draft`、`Proposed`、`Approved`、`Implemented`、`Deprecated` lifecycleを使用する。Requirement IDには `GUI-`
prefixを付け、その後にsurfaceとstableな3桁のnumberを続ける。

GUI specでは `masterdata-core` のsemanticsを重複させず、behaviorを記述する。layout、state、selection、editing、
validation、focus、keyboardとmouse interaction、loading、empty/error/disabled state、unsaved change、build-in-progress
behaviorを扱ってよい。shared domain operationに対するadapter boundaryはTauri commandである。

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
Requirement IDをreviewし、compatibility/acceptance noteを必要に応じて更新する。このtaskではTable Editorのimageも
behaviorも追加しない。

各canonical GUI ruleは、1つのsurface specificationに1つだけ置く。observableなlayout、state、selection、editing、
keyboard、focus、validation、loading、empty/error/disabled、unsaved-change、build-progress behaviorには、
`GUI-TABLE-SEL-001` のようなRequirement IDを使用する。shared domain meaningは `masterdata-core` に残し、GUI specは
userのobservableなadapter behaviorとTauriとのboundaryだけを記述する。
