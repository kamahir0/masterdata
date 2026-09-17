# GUI仕様: Data Editor Grid Authoring

Status: Approved

この仕様はData Editorのrange selection、batch preview、query composition、未保存履歴、keyboard precedenceを定義する。[Data Editor](spec.md)のsingle-cell / dirty / Save contractと[Authoring Batch](../../specs/authoring-batch.md)、[Authoring Query](../../specs/authoring-query.md)を前提とする。適用記録は[仕様変更0016](../../spec-changes/0016-desktop-daily-editing.md)を参照する。

## 規範要件

### GUI-GRID-001

Data Editorはsingle active cellと矩形rangeのanchor/focusを区別し、Shift+Arrowとpointerで連続rangeを選択できなければならない（MUST）。非連続selectionはv1外。columnはschema順、rowはquery結果順とする。
pasteはactive cellを左上とし、clipboard shapeだけを使う。選択rangeへのrepeat、sheet境界でのtruncate、row自動追加をしてはならない（MUST NOT）。fillは選択rangeへ利用者が入力した一つのtextを各target typeで解釈する。hidden rowへ適用しない。

### GUI-GRID-002

paste / fill / range Set Nullは、対象file、変更cell数、target row/column、before/after、diagnosticsを確認するpreviewと明示Apply / Cancelを提供しなければならない（MUST）。single-cell pasteも同経路を使う。
previewはbase、buffer revision、schema/type revision、query/selection revisionにbindし、どれか変わればApply不可にする（MUST）。Apply前にもshared layerで一致を確認する。Cancel/failureは元bufferとselectionを保つ。全cell no-opは履歴を増やさない。

### GUI-GRID-003

query変更はcell edit確定後に反映しなければならない（MUST）。入力が表現不能ならactive editを保持してquery変更を停止する。domain-invalidだけでは確定を拒否しない。
編集確定でrowがfilter対象外になった場合は同じ表示位置の次row、なければ前rowへselectionを移し、0件ならgridのempty stateへfocusを置く。sortで移動したsurviving occurrenceのselectionは追従する（MUST）。query変更はrangeをactive cell一つへ縮める。
Pending deleteは通常query結果と別のUndo可能な一覧として発見でき、range対象にしない。Add Row直後はfilter/sortをclearして新rowへfocusし、clearしたことを通知する（MUST）。これはsource順を変えない。

### GUI-GRID-004

fileごとの未保存履歴は、scalar edit確定、complex control一操作、Add Row、Delete、Undo Delete、batch Apply、およびTag確定を単位にUndo/Redoできなければならない（MUST）。typingの一文字ごとではなくedit確定を単位とする。new editはそのfileのRedoをclearする。
履歴は値だけでなくAdded draft、Pending delete、source provenance、nested sequence identity等を戻し、同じlocal stateから同じcandidateを再生成できなければならない（MUST）。Undo Deleteは削除前のeditsを戻す新操作として履歴へ入り、general Undoで再びPending deleteへ戻せる。Added draft削除をUndoすればそのdraftの入力が戻る。

### GUI-GRID-005

Save / explicit Overwrite successは対象fileの履歴をclearする（MUST）。他fileの履歴は変えない。Failure / Conflict / Outcome Unknownではbufferと履歴を保持する。履歴操作はdiskを書き換えてはならない（MUST NOT）。Outcome Unknown中のSaveはactual source確認まで停止する既存契約に従う。
履歴をUndoしてcleanになってもRedoを維持する。ただしclean external reload、明示Reload / Don't Save、Project終了、成功Migrationによる当該base更新では履歴をclearする（MUST）。ConflictをUndoだけで解消済み扱いせずactual source再確認を要する。save中の当該fileの編集/Undo/Redoは停止し、保存結果と新入力の混在を防ぐ。
履歴はsession内だけとし、永続recoveryを保証しない。memory都合で履歴を捨てる場合は事前に通知し、current bufferは保持しなければならない（MUST）。

### GUI-GRID-006

grid navigation modeではCmd/Ctrl+C / Vをcopy/paste、Cmd/Ctrl+ZをUndo、Cmd+Shift+ZまたはCtrl+Y / Ctrl+Shift+ZをRedo、Shift+Arrowをrange拡張へ割り当てる（MUST）。cell/nested text control編集中はclipboardとUndoをcontrol内text編集へ委ね、range操作やfile Undoを同時実行しない（MUST NOT）。Escapeはactive edit cancelを優先し、navigation modeではrangeをactive cellへ縮める。
Enter/F2によるedit開始、Enter/Tabによる確定移動とfile Save shortcutは既存契約を保つ。IME composition中にEnterを確定移動へ誤解釈しない（MUST NOT）。Deleteキーをbulk row deleteへ割り当てない。preview終了後はsurviving active cell、なければgridへfocusを戻す。
selection範囲、対象件数、read-only理由、preview失効、履歴有無をassistive technologyへ伝えなければならない（MUST）。

## 既存Data Editor contractへの適用

- `GUI-DATA-EDIT-001`のsingle-cell editingを維持し、本仕様のrange / paste / fill / Undo/Redoを追加する。fill handle、非連続range、bulk row add/deleteは対象外。
- `GUI-DATA-LAYOUT-003`の初期/reset表示はsource順を維持し、明示query時のみAuthoring Queryに従う。
- `GUI-DATA-KEY-002`のkeyboard-only編集を維持し、mode precedenceは`GUI-GRID-006`を使用する。
- `GUI-DATA-ROW-004`のappend/source順をdefaultとし、Add Row後のquery clearは`GUI-GRID-003`に従う。
- `GUI-DATA-ROW-007`のUndo Delete復元意味を維持し、general historyへ合成する。

## 受け入れ証拠

filter中paste/fill、境界超過、preview後query変更、編集でrowが消える場合、Addでquery clear、Pending deleteのUndo導線、Add→edit→delete→Undo、Undo DeleteのUndo、Save All部分失敗、clean external reload、ConflictとUndo、IME / text-control precedenceを検証する。
