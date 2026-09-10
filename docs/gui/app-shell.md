# GUI app shell（GUIアプリシェル）

Status: Draft

Tauri v2 applicationは薄いdesktop adapterである。起動時に `project_info` Tauri commandを呼び出す。
commandはshared `masterdata-app` serviceを呼び、domain workを `masterdata-core` に委譲し、serializableな
`ProjectInfo`を返す。React frontendはfilesystemをinspectせず、YAMLをparseせず、CLIをspawnしない。

初期shellには次を含む。

- root/config/source pathを表示するproject identity card
- [Workspace Explorer](explorer/spec.md)によるproject sourceのfile / folder navigation
- 選択したsource kindに応じた中央typed editor area。初期record authoring sliceでは[data editor](data-editor/spec.md)を使用する
- projectの再読み込みaction
- current projectのsource validation action。`validate` Tauri commandを通じてshared application serviceの既存`ValidationReport`とstructured diagnosticsを表示する。
- current projectのfull canonical build action。`build` Tauri commandを`dryRun = false`で呼び出し、既存のbuild resultとstructured diagnosticsを表示する。external publish targetはこのactionで更新しない。

基本layoutは、左のWorkspace Explorer、中央のtyped editor area、必要に応じた右inspector / detail surface、上部またはplatformに適したcommand surfaceで構成する。
record data YAMLを選択した場合、中央は列=field、行=選択file内recordのspreadsheet型Data Editorとなる。
folder位置はdomain semanticsを決めず、document kindに応じてeditorを選択する。

将来的にはExplorerからfolder、Table、record data、Value Object、Enum等のsource artifactを作成できるauthoring modelを目指すが、各create operationのsemanticsと専用editorは対応するcanonical仕様が整った段階で追加する。現在のrecord editing Objectiveでは、それらの作成機能は実装scope外である。

GUI errorはstructuredなdiagnostic code、kind、path、line/column、schema path、record identity、suggestion、
related requirement referenceを保持する。frontendはTauri boundaryでflattenしない限り、そのdataのどこまでrenderするかを
選択してよい。

Open Questions: project picker UX、dirty file navigation / window close、native file watcherとexternal modification recovery、right inspector / diff / validation detailの具体的layout。
