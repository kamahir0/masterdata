# GUI仕様: Workspace Explorer

Status: Proposed

## 目的

Project内のsource workspaceをVS Codeに近いfile / folder treeとして左ペインに表示し、利用者が編集対象sourceを選択できるようにする。
Explorerはdomain treeではなくworkspace navigationであり、folder配置そのものへTable / Value Object / Enum等のsemantic meaningを追加しない。

## レイアウト（Layout）

### GUI-EXPLORER-001

Desktop authoring画面は左側にWorkspace Explorerを持たなければならない（MUST）。ExplorerはProjectに設定されたsource rootごとのfile / folder hierarchyを表示し、中央typed editorとは独立したnavigation surfaceとして扱う。

複数source rootがある場合は、利用者がどのconfigured rootに属する項目か識別できなければならない（MUST）。source root外の`.masterdata/output`、cache、generated artifact等をExplorerのsource treeへ暗黙に混在させてはならない（MUST NOT）。

### GUI-EXPLORER-002

fileを選択した場合、中央main areaはそのsource documentに対応するtyped editorまたはunsupported/read-only stateを表示しなければならない（MUST）。editor種別の判定はfolder名・folder位置ではなく、既存のsource semanticsとdocument kindに従わなければならない（MUST）。

### GUI-EXPLORER-003

初期record authoring sliceでは、record Data documentを選択した場合に[Data Editor](../data-editor/spec.md)を開かなければならない（MUST）。現在のObjectiveで専用editorを持たないdocument kindまたは非Masterdata fileを、別kindとして誤解釈して編集可能にしてはならない（MUST NOT）。

## 状態（States）

### GUI-EXPLORER-STATE-001

source data fileのdirty stateはfile単位でExplorerから識別可能でなければならない（MUST）。別fileのdirty stateは独立して表示・保持しなければならない（MUST）。

### GUI-EXPLORER-STATE-002

loading、save in progress、save failure、Conflict、Outcome Unknown等のsource stateは、単なるfile selectionと区別して識別できなければならない（MUST）。exact icon / colorは固定しないが、色だけを唯一の状態伝達手段にしてはならない（MUST NOT）。

### GUI-EXPLORER-STATE-003

cleanなsource fileが外部変更されて再読込された場合、Explorer selectionを可能な範囲で維持しなければならない（MUST）。dirty fileの外部変更はData Editorと[Source Record Edit](../../specs/source-edit.md)のConflict lifecycleへrouteし、dirty stateを暗黙に消してはならない（MUST NOT）。

## 操作（Interactions）

### GUI-EXPLORER-INT-001

file selectionは対応するtyped editorを開かなければならない（MUST）。record / Tableのdomain selectionをfilesystem pathのsemantic identityとして扱ってはならない（MUST NOT）。

### GUI-EXPLORER-INT-002

folderはexpand / collapseできなければならず（MUST）、file / folder selectionとexpand stateを区別しなければならない（MUST）。通常のfile switchingだけを理由にdirty bufferのSave確認を出してはならない（MUST NOT）。

### GUI-EXPLORER-INT-003

Project切替、Project Reload、window close等、dirty bufferを失う可能性がある上位操作はData Editor specificationの`Save All` / `Don't Save` / `Cancel` lifecycleに従わなければならない（MUST）。Explorer独自の別unsaved-changes policyを実装してはならない（MUST NOT）。

## キーボード（Keyboard）

### GUI-EXPLORER-KEY-001

Explorer treeはkeyboardだけでitem間移動、folder expand / collapse、file openを行えなければならない（MUST）。Arrow key / Enter等のplatform-standard tree interactionを基本とし、exact implementationが異なる場合も同等のkeyboard-only操作を失ってはならない（MUST NOT）。

Save shortcutは現在activeなeditor/fileへ委譲し、Explorer focus中であることだけを理由に別fileを保存してはならない（MUST NOT）。

## フォーカス（Focus）

### GUI-EXPLORER-FOCUS-001

Explorerでfileを開いた場合、中央editorへ移動できる明確なkeyboard pathを持たなければならない（MUST）。editorからExplorerへ戻った場合は、可能な範囲で最後に選択していたtree itemへfocusを復元しなければならない（MUST）。

## 検証（Validation）

### GUI-EXPLORER-VAL-001

ExplorerはYAMLやdomain semanticsをfrontend独自にparse / validateしてはならない（MUST NOT）。document classification、validation、structured diagnosticはshared application/domain boundaryから受け取らなければならない（MUST）。

現在のfile / subtreeにdiagnosticがあることをExplorer上でbadge等により補助表示してよい（MAY）が、初期sliceではProblems panelがcanonical detail surfaceであり、Explorer badgeを必須にしない。

## エラー（Errors）

### GUI-EXPLORER-ERR-001

filesystem I/O、permission、path safety、source classification、external modification等でfileを開けない場合、既存のdirty bufferや他file selection stateを不必要に破壊してはならない（MUST NOT）。利用者が原因を確認できるstructured error stateを表示しなければならない（MUST）。

## アクセシビリティ（Accessibility）

### GUI-EXPLORER-A11Y-001

Treeとしてのrole、selection、expanded / collapsed、dirty、Conflict等のstateをassistive technologyから識別可能にしなければならない（MUST）。状態伝達を視覚上の色やiconだけに依存させてはならない（MUST NOT）。

## 参照artifact（Reference Artifacts）

None.

## 将来のcreation model

長期的なauthoring modelでは、ExplorerからfolderおよびTable、record data、Value Object、Enum等のsource artifactを作成できる入口を持つ方向とする。ただし各artifactのcreate semantics、template、命名、保存先、validation、rename / delete / moveは、それぞれのcanonical domain仕様が整った時点で別途仕様化する。

現在のGUI record editing Objectiveでは、新規folder / Table / Value Object / Enum等の作成機能そのものはimplementation scopeに含めない。

## 未解決事項（Open Questions）

None identified for the initial existing-record Explorer. Creation / rename / delete / move、diagnostic aggregation badge、tree filter / search、favorite / recent source等は将来UXとして別途扱う。
