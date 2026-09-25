# Source Tag Edit仕様

Status: Approved

Domain: Source Editing

この仕様はRecord Tag `$tags` のsource-preserving authoring contractを定義する。Tagのdomain semanticsとBuild Selectionは[Build Selection](build-selection.md)、通常field editは[Source Record Edit](source-edit.md)、record add/deleteは[Source Record Mutation](source-record-mutation.md)が所有する。適用記録は[仕様変更0017](../spec-changes/0017-desktop-workspace-settings.md)を参照する。

## 規範要件

### SOURCE-TAG-001

Tag editは通常record member editと別のsemantic requestで、base snapshot内のoccurrenceまたはAdded draftの`$tags`だけを対象にしなければならない（MUST）。既存domain fieldsのeditorへ`$tags`をschema fieldとして加えてはならない（MUST NOT）。Pending deleteにはTag edit不可。
`records`を明示したschema document内のrecord occurrenceも対象に含め、schema declaration自体をTag editで変更してはならない（MUST NOT）。
requestはordered string entriesを保持し、Add / Remove / Replace entryを提供する。case修正、trim、deduplicate、sortを暗黙実行してはならない（MUST NOT）。invalid lexical tag / duplicateはshared diagnosticとし、安全なsource candidateならSaveを拒否しない。

### SOURCE-TAG-002

既存`$tags`がstring sequenceなら個別entryをsource-preservingにpatchし、変更対象外entryとcommentを保持しなければならない（MUST）。non-sequence、non-string element、location曖昧なsourceは理由付きread-onlyとし、推測変換やsubtree/full-file再serializeをしてはならない（MUST NOT）。通常value editや閲覧まで無効にしない。
未指定`$tags`へ最初のentryを追加する場合はrecord mapping末尾へmetadata entryを挿入する。全entryを削除した既存propertyは`$tags: []`のempty sequenceとして保持する（MUST）。未指定から追加して全て戻した場合は元の未指定へ戻し、他変更がなければbyte-identical / cleanとなる。unrelated commentは削除しない。
block / flow string sequenceとblock record mappingへの新規property追加を最低対応範囲とする。それ以外はsafe localization可否をshared layerが報告する。

### SOURCE-TAG-003

Tag、existing value edits、Added drafts、Pending deletesを同一fileの一candidateへcompositionしなければならない（MUST）。deletionが最終candidateで優先し、Undo Deleteで削除前Tagと値を復元する。Added draftのTagは初期未指定、入力した場合はdeclared fieldsの後にmetadataとして出力する。
Source Editのfile commit / result / lost-update / explicit Overwrite / Outcome Unknown / 非連動とDesktop制作v1のhistory contractを適用しなければならない（MUST）。Tag editはschema/key mutationを開始しない。

## 受け入れ証拠

absent→add→removeのbyte同一、既存`[]`、flow/block/comment、invalid/duplicate、Added/Deleted/Undo、同PK別rowの保全を検証する。
