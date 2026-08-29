# GUI app shell（GUIアプリシェル）

Status: Draft

Tauri v2 applicationは薄いdesktop adapterである。起動時に `project_info` Tauri commandを呼び出す。
commandはshared `masterdata-app` serviceを呼び、domain workを `masterdata-core` に委譲し、serializableな
`ProjectInfo`を返す。React frontendはfilesystemをinspectせず、YAMLをparseせず、CLIをspawnしない。

初期shellには次を含む。

- root/config/source pathを表示するproject identity card
- tableとtypeに対するplaceholder navigation
- projectの再読み込みaction
- placeholderのValidateとBuild control。backend commandはshared application serviceを使い、より詳細なinteractionは将来のGUI scopeに残す。

計画中のlayoutは、左navigation、中央のrecord/editor area、右inspector、上部のSave/Validate/Build actionである。
GUI errorはstructuredなdiagnostic code、kind、path、line/column、schema path、record identity、suggestion、
related requirement referenceを保持する。frontendはTauri boundaryでflattenしない限り、そのdataのどこまでrenderするかを
選択してよい。

Open Questions: project picker UX、unsaved changes policy、native file watcher strategy。
