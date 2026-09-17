# GUI仕様: Data Editor Tag Authoring

Status: Approved

この仕様はData EditorでRecord Tagを編集するGUI contractを定義する。source mutationは[Source Tag Edit](../../specs/source-tag-edit.md)、Tag semanticsは[Build Selection](../../specs/build-selection.md)が所有する。適用記録は[仕様変更0017](../../spec-changes/0017-desktop-workspace-settings.md)を参照する。

## 規範要件

### GUI-TAG-001

Data Editorは選択recordのTag editorをdomain列と別に提供し、add/remove/replaceをkeyboardで操作できなければならない（MUST）。tag確定を一history操作とし、入力中textと確定entryを区別する。空textも勝手に破棄せず、確定すればinvalid tagとして診断する。
候補は読込済みsource tagsと保存済みprofile tagsのunionをshared layerから受け取り、一覧が部分的ならその旨を表示する（MUST）。候補選択は任意で新tag入力を許可する。Add Rowと既存rowの両方に提供し、filterやProfile選択を理由にsource tagsを変更しない。

## 受け入れ証拠

Tagのadd/remove/replace、invalid/duplicate、Added draft / existing record、Delete / Undoとのcomposition、keyboard-only操作を検証する。
