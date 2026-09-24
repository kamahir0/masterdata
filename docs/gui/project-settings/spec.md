# GUI仕様: Project Settings

Status: Approved

Project Settingsは`masterdata.toml`のProfileとPublish targetをtyped formで編集する。source mutationは[Project Config Edit](../../specs/project-config-edit.md)、Project config semanticsは[Project layout](../../specs/project-layout.md)、Build Profileは[Build Selection](../../specs/build-selection.md)、Publish targetは[Build pipeline](../../specs/build-pipeline.md)が所有する。適用記録は[仕様変更0017](../../spec-changes/0017-desktop-workspace-settings.md)を参照する。

## 規範要件

### GUI-SETTINGS-001

Project SettingsはProfileとPublish targetsをtyped formで編集し、config file単位のdirty / Diff / Saveを持たなければならない（MUST）。section移動や他editorへのnavigationでbufferを保存・破棄しない。Cmd/Ctrl+Sはactive settingsのconfigだけを保存する。
ProfilesとPublish Targetsは区別した領域で表示し、一方を閲覧しても他方の未保存form draftを破棄してはならない（MUST NOT）。Migration / Publishの確認、Recovery Required、Conflict等のsafety surfaceは関連operation中に引き続き確認できなければならない（MUST）。
form editのactive textを確定してからSaveし、表現不能なら入力を保持して停止する。domain-invalidならdiagnostics付きで確定できる。Save中は当該config編集を停止する。
settingsのgeneral Undo/Redoはv1必須ではない。text input内Undoは維持し、config全体の明示Discardは確認を経る。YAML file履歴と混同してはならない（MUST NOT）。

### GUI-SETTINGS-002

Project切替 / Reload / closeは、YAMLとconfigの全dirtyを同じSave All / Don't Save / Cancel guardへ含めなければならない（MUST）。Save Allはconfigを先に保存・project binding再解決し、config失敗/無効でYAML保存の安全なauthorityを取得できなければ以降を開始しない。config成功後にYAMLで失敗した場合は成功fileをrollbackせず、元の上位操作を止めて残bufferを保持する。all-or-none transactionではない。
通常config Save後にsource roots/identityが外部変更等で変わった場合は旧workspaceへ推測で接続しない（MUST NOT）。再open / reloadとdirty guardを要求する。
Migration Recovery Required中はconfig保存も停止する。read-only Diff / Compare / 診断は継続可能とする（MUST）。config dirtyだけで保存済みinputのBuild / Publishを禁止しないが、未保存設定は不使用と表示する。

### GUI-SETTINGS-003

設定form、Tag、Overviewのcontrolはlabelとstateをassistive technologyへ伝え、keyboard-onlyで操作・error確認・Diff・復帰ができなければならない（MUST）。unknown/unsupported設定を隠して消す代わりにreadonly reasonとlocationを提供する。
成功したconfig Saveはsaved profile/target一覧とread-only previewをinvalidateし、新snapshot取得後に更新する（MUST）。named profileのselectionを暗黙変更しない。

## 受け入れ証拠

active Saveはconfigだけ、Save Allでconfig成功→YAML失敗、config-invalid後の修正導線、Outcome Unknown、Recovery Required、navigation保持を検証する。
