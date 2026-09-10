# GUI仕様: Workspace Explorer

Status: Draft

## 目的

Project内のsource workspaceを、VS Codeに近いfile / folder treeとして左ペインに表示し、利用者が編集対象のsourceを選択できるようにする。
Explorerはdomain treeではなくworkspace navigationであり、folder配置そのものへTable / Value Object / Enum等のsemantic meaningを追加しない。

## レイアウト（Layout）

### GUI-EXPLORER-001

Desktop authoring画面は左側にWorkspace Explorerを持つ。ExplorerはProjectで利用可能なsource fileとfolderを階層表示し、中央editorとは独立したnavigation surfaceとして扱う。

### GUI-EXPLORER-002

fileを選択した場合、中央main areaはそのsource documentに対応するtyped editorを表示する。editor種別の判定はfolder名・folder位置ではなく、既存のsource semanticsとdocument kindに従う。

### GUI-EXPLORER-003

初期record authoring sliceでは、record data YAMLを選択した場合に[data editor](../data-editor/spec.md)を開く。他kindの専用editorは段階的に追加できるが、unsupportedなdocumentを別kindとして誤解釈してはならない。

## 状態（States）

### GUI-EXPLORER-STATE-001

source data fileのdirty stateはfile単位でExplorerから識別可能でなければならない。別fileのdirty stateは独立して保持する。

### GUI-EXPLORER-STATE-002

loading、unavailable、external modification、save failure等の状態をfile selectionそのものと混同しない。exact表示とrecovery actionはsource-edit contractと後続GUI reviewで確定する。

## 操作（Interactions）

### GUI-EXPLORER-INT-001

file selectionは対応するtyped editorを開く。record / Tableのdomain selectionをfilesystem pathのsemantic identityとして扱ってはならない。

### Target creation model

長期的なauthoring modelでは、ExplorerからfolderおよびTable、record data、Value Object、Enum等のsource artifactを作成できる入口を持つ方向とする。ただし、各artifactのcreate semantics、template、命名、保存先、validation、rename/deleteはそれぞれのcanonical domain仕様が整った時点で別途仕様化する。

現在のGUI record editing Objectiveでは、新規folder / Table / Value Object / Enum等の作成機能そのものはimplementation scopeに含めない。

## キーボード（Keyboard）

Explorer navigation、rename/create shortcut、typed editorへのfocus移動、Save shortcutのexact key bindingは未決定。platform慣習とaccessibilityを考慮して後続reviewで定義する。

## フォーカス（Focus）

Explorerでfileを選択してeditorを開いた際の初期focus、keyboard navigation、focus restorationは未決定。

## 検証（Validation）

ExplorerはYAMLやdomain semanticsをfrontend独自にparse / validateしない。structured resultはshared application/domain boundaryから受け取る。

## エラー（Errors）

filesystem I/O、permission、external modification、path safety等でfile operationを完了できない場合、validation errorとは別のoperation failureとして表示する。exact recovery UXはsource-edit仕様と合わせて決める。

## アクセシビリティ（Accessibility）

Treeとしてのrole、selection、expanded/collapsed state、dirty indicatorを視覚表現だけに依存させない。exact keyboard semanticsとscreen-reader labelは後続reviewで確定する。

## 参照artifact（Reference Artifacts）

None.

## 未解決事項（Open Questions）

- Project root配下のどのfile / folderをExplorerに表示するか。generated / internal artifactのfiltering policy。
- create / rename / delete / moveを導入する時点のconfirmation、collision、path safety、undo/recovery。
- dirty fileがある状態でのfile / project切替とwindow close behavior。
- external modification時の表示、compare / reload / overwrite UX。
- Explorer上のdiagnostic badgeやstatus aggregationの粒度。
