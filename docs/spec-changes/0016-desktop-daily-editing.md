# 仕様変更: Desktop制作v1 — P1 日常編集

Status: Proposed

## 根拠と分類（Source Evidence and Classification）

- Decision: 2026-09-16、HumanはRFC 0008を説明した応答への「進める」により、Option B / P1–P3、file単位編集、保存前Undo、scalar一括入力、計算列の後段化を選択した。
- Requirement: 元の依頼は創造性を発揮してまとまった仕様案を作ること。本changeはその詳細化であり、product実装やspecification Approvalの依頼ではない。
- Constraint: source preservation、exact occurrence、lossless transport、validation非blocking、既存key read-only、shared Rustを維持する。
- Proposal: 以下のcodec、operator、shortcut、history合成は今回の具体案。MUST等は**採用後の契約文案**であり、Humanが細部まで既に決定したという意味ではない。

## Affected Specifications

| Owner | 現Status / affected ID | canonical適用先 |
| --- | --- | --- |
| [Data Editor](../gui/data-editor/spec.md) | Approved; `GUI-DATA-EDIT-001`, `GUI-DATA-LAYOUT-003`, `GUI-DATA-KEY-002` | 既存ruleの差分と新GUI-GRID family |
| [Record Mutation GUI](../gui/data-editor/record-mutation.md) | Approved; `GUI-DATA-ROW-004`, `GUI-DATA-ROW-007`、Interaction details / 非目標 | history・sort/filter合成を更新 |
| [Source Edit](../specs/source-edit.md) | Approved; `SOURCE-EDIT-001`〜`SOURCE-EDIT-016`を維持 | 新規`docs/specs/authoring-batch.md`へAUTHORING-BATCH family |
| [Type System](../specs/type-system/README.md) | Approved各ownerを参照、変更なし | 新規`docs/specs/authoring-query.md`へAUTHORING-QUERY family |
| [Table Editor](../gui/table-editor/spec.md) | Approved; `GUI-TABLE-INT-001` | typed initializer追加 |
| [Type Editor](../gui/type-editor/spec.md) | Approved; `GUI-TYPE-INT-003` | typed initializer追加 |

新規owner pathは承認後の配置計画。現在のcanonical fileには未承認deltaを入れない。
[0017](0017-desktop-workspace-settings.md)、[0018](0018-desktop-build-delivery.md)と一括reviewする。各ruleは上表の一箇所へ移す。

## Confirmed Decisions

P1–P3のscope選択と既存安全境界。Overviewは保存済みread-only、bulkは単一file、UndoはSave成功まで。

## New Requirements

### AUTHORING-BATCH-001

一括編集は一つのfileのbase snapshot、current local buffer revision、resolved schema/type revision、ordered target occurrence/fieldと入力を受け、shared applicationでcandidateを作らなければならない（MUST）。既存recordとAdded draftを識別し、PKだけ、表示row indexだけでtargetを解決してはならない（MUST NOT）。wire形は固定しない。

対象はtop-level fieldのRequired / Nullable Primitive、Value Object、normal Enumとする。Flags、Custom Type、Array、nested leafをrange対象にしてはならない（MUST NOT）。existing key、Pending delete、unsupported/unresolved fieldは編集対象外。Added draftのkeyは既存契約に従って入力可能とする。

### AUTHORING-BATCH-002

operationは「全targetのcandidateを適用」または「buffer不変」のどちらかでなければならない（MUST）。source定位不能、duplicate target、read-only混入、矩形不足、codec failureはoperation failureとし、黙ってskip/truncateしてはならない（MUST NOT）。domain-invalid valueのdiagnosticはoperation failureやSave gateにしてはならない（MUST NOT）。同一入力でbyte-identical candidateならno-opとする。

既存source-preserving patchとrecord mutation compositionを使い、既存local editsを失ってはならない（MUST NOT）。適用はbufferだけに行い、Save / Build / Migrationを暗黙実行してはならない（MUST NOT）。

### AUTHORING-BATCH-003

clipboard textはshared codecで矩形TSVとしてdecodeしなければならない（MUST）。以下をv1 codecとする。

- separatorはTAB。quoted field外のLFまたはCRLFをrow delimiterとする。bare CRはquoted field内でのみ許可する。
- field先頭の`"`はquoted field開始。内部の`""`はliteral `"`。終端quote後はseparator、row delimiter、EOFだけを許可する。
- unquoted fieldのquoteはmalformed。quoted field内のTAB / LF / CRLF / CRは文字としてそのまま保持する。
- 末尾のrow delimiter一個は終端とし、余分なempty rowを作らない。二個目以降はempty rowを表す。末尾TABは最後のempty fieldを作る。
- 空textは1 row × 1 empty field。row幅不一致、未閉quote、終端quote後の余分な文字は全体failure。header推測・trim・Unicode normalizationを行わない。

copyは全fieldをquoteし、内部quoteを二重化、列はTAB、行はLF、最後のrow delimiterなしでencodeしなければならない（MUST）。これによりempty string、embedded newline、末尾empty fieldを区別する。HTML clipboardやspreadsheet formulaを実行してはならない（MUST NOT）。

### AUTHORING-BATCH-004

decodeした各field textはshared resolved typeで次のauthoring inputに変換しなければならない（MUST）。TSV quotingはdelimiter escapingだけで、YAML quote指定ではない。

| 対象 | 入力の意味 |
| --- | --- |
| string / string-underlying Value Object / normal Enum | text全体をstringとして保持。`null`、`001`、`=...`もliteral。Enumの未知memberはdomain-invalid |
| 数値 / bool / numeric Value Object | YAML subsetのplain numeric/bool tokenとしてlosslessに表せるtextだけをそのcategoryとして保持。domain range / category不一致はdiagnostic。quoted YAML、collection・tag・alias・unsupported scalar、それ以外のtextは全体をliteral stringとして保持しdomain-invalidを示す |
| すべてのscalar対象でempty text | empty string。0、null、未入力placeholderに変換しない |

`null` textはstring系では文字列、数値/bool系でもliteral stringとして保持し、null化の暗黙syntaxにはしない（MUST NOT）。Nullableのnull化は明示`Set Null`で行い、Requiredを含むrangeへは適用しない。
数値/bool入力に前後whitespaceがある場合もtrimせずliteral stringとして保持する。例えばint列の`1.0`、`abc`、range外整数はcoerceせずdiagnostic対象とする。

copyはvalid non-null scalarのみを対象とし、string/Enumはliteral value、boolは`true`/`false`、integerはexact decimal、float/doubleはshared layerのround-trip可能なfinite decimal textとする（MUST）。float/doubleの整数相当値にも小数点または指数を含め、paste時にinteger categoryへ変わらない表現とする。null、invalid、complexを含むrangeはcopyを理由付きで停止し、曖昧なempty cellとして出力してはならない（MUST NOT）。read-only keyのcopyは許可する。cutはv1外。

### AUTHORING-QUERY-001

filter / search / sortはshared layerがtyped snapshotから導出し、source bytes、dirty、canonical record orderingを変えてはならない（MUST NOT）。初期状態はqueryなし、source順でなければならない（MUST）。File Viewはcurrent buffer、Overviewは0017の保存済みsnapshotを使う。

検索は全top-level valid non-null scalarの表示textに対するcase-sensitive部分一致のORとする。literal string/Enum symbol、bool小文字、数値はAUTHORING-BATCH-004のcopy textを使用する。query emptyは検索制約なし。complex、invalid、nullのtextを検索一致と推測しない（MUST NOT）。

### AUTHORING-QUERY-002

column filterは複数条件のANDとし、以下のoperatorだけをv1で提供しなければならない（MUST）。operator/input不正はquery failureで、旧結果をcurrentとして表示してはならない（MUST NOT）。query stateはproject source/configへ保存しない。

| Shape | Operator |
| --- | --- |
| int / uint / long / ulongとnumeric Value Object | equals, not-equals, less-than, greater-than |
| string / string Value Object | equals, not-equals, contains（ordinal、case-sensitive、normalizationなし） |
| bool | equals, not-equals |
| normal Enum | symbol equals, not-equals |
| 全field | is-null, is-invalid |

通常operatorはvalid non-null値だけを評価し、それ以外はnot-equalsもfalse。is-nullは明示null/Added placeholder、is-invalidはshared field validationがinvalidと判定した値に一致する。Required nullは両方に一致しなければならない（MUST）。query入力自体は対象typeのvalid non-null値を要求する。type unresolved等でfield validityを確定できない場合、is-invalidを推測せずそのqueryをUnavailableとする。検索制約とcolumn filter群はANDで合成する。
float/doubleの数値filterとFlags / Array / Customの構造queryはv1外とし、数値comparison capabilityを暗黙追加しない。

### AUTHORING-QUERY-003

表示sortは単一columnのascending / descending / noneとし、比較可能なint / uint / long / ulong / string / Value Objectだけに提供しなければならない（MUST）。comparisonはApproved Primitive / Value Object ownerに従う。valid値、null、その他invalidの順を両方向で保ち、descendingはvalid値内だけを反転する。同値と各非valid群は入力source順でstableとする（MUST）。
Enum / bool / float / double / complexのsortはv1外。schema column順をview独自順に変えない。

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

fileごとの未保存履歴は、scalar edit確定、complex control一操作、Add Row、Delete、Undo Delete、batch Apply、および0017のTag確定を単位にUndo/Redoできなければならない（MUST）。typingの一文字ごとではなくedit確定を単位とする。new editはそのfileのRedoをclearする。
履歴は値だけでなくAdded draft、Pending delete、source provenance、nested sequence identity等を戻し、同じlocal stateから同じcandidateを再生成できなければならない（MUST）。Undo Deleteは削除前のeditsを戻す新操作として履歴へ入り、general Undoで再びPending deleteへ戻せる。Added draft削除をUndoすればそのdraftの入力が戻る。

### GUI-GRID-005

Save / explicit Overwrite successは対象fileの履歴をclearする（MUST）。他fileの履歴は変えない。Failure / Conflict / Outcome Unknownではbufferと履歴を保持する。履歴操作はdiskを書き換えてはならない（MUST NOT）。Outcome Unknown中のSaveはactual source確認まで停止する既存契約に従う。
履歴をUndoしてcleanになってもRedoを維持する。ただしclean external reload、明示Reload / Don't Save、Project終了、成功Migrationによる当該base更新では履歴をclearする（MUST）。ConflictをUndoだけで解消済み扱いせずactual source再確認を要する。save中の当該fileの編集/Undo/Redoは停止し、保存結果と新入力の混在を防ぐ。
履歴はsession内だけとし、永続recoveryを保証しない。memory都合で履歴を捨てる場合は事前に通知し、current bufferは保持しなければならない（MUST）。

### GUI-GRID-006

grid navigation modeではCmd/Ctrl+C / Vをcopy/paste、Cmd/Ctrl+ZをUndo、Cmd+Shift+ZまたはCtrl+Y / Ctrl+Shift+ZをRedo、Shift+Arrowをrange拡張へ割り当てる（MUST）。cell/nested text control編集中はclipboardとUndoをcontrol内text編集へ委ね、range操作やfile Undoを同時実行しない（MUST NOT）。Escapeはactive edit cancelを優先し、navigation modeではrangeをactive cellへ縮める。
Enter/F2によるedit開始、Enter/Tabによる確定移動とfile Save shortcutは既存契約を保つ。IME composition中にEnterを確定移動へ誤解釈しない（MUST NOT）。Deleteキーをbulk row deleteへ割り当てない。preview終了後はsurviving active cell、なければgridへfocusを戻す。
selection範囲、対象件数、read-only理由、preview失効、履歴有無をassistive technologyへ伝えなければならない（MUST）。

### GUI-TABLE-INT-008

AddField initializerはshared resolved value authoring modelからschema-aware controlを構成しなければならない（MUST）。通常入力にraw YAML / JSONを要求してはならない（MUST NOT）。型/modifier変更時は古いinitializerを別型へcoerceせずunsetへ戻し、既存Planを失効させる。
未入力とexplicit nullを区別し、required initializerが未入力/invalidならMigration Planのpreconditionとして扱う（MUST）。Data Editorのinvalid Save許可をMigration initializer validityへ転用してはならない（MUST NOT）。64-bit、nested値、valid constantの意味は既存ownerに従う。

### GUI-TYPE-INT-010

AddCustomField initializerへGUI-TABLE-INT-008と同じshared editor / unsetとnullの区別 / Plan失効を適用しなければならない（MUST）。initializer要否とvalidityはType Migration ownerが決め、data draftのplaceholderを自動的なinitializerへ昇格させてはならない（MUST NOT）。

## Changed Requirements

- `GUI-DATA-EDIT-001`: single-cell必須、高度操作は必須でない → single-cellを維持し、新GUI-GRID familyのrange / Undoを追加する。fill handle、非連続range、bulk row操作は依然対象外。
- `GUI-DATA-LAYOUT-003`: source順固定 → initial/resetはsource順、明示query時だけAUTHORING-QUERYへ従う。選択file限定・非dedupは維持。
- `GUI-DATA-KEY-002`: 既存keyboard経路を保ち、mode precedenceをGUI-GRID-006へ参照。
- `GUI-DATA-ROW-004`: append/source順はdefaultとし、Add RowでqueryをclearするGUI-GRID-003へ参照。
- `GUI-DATA-ROW-007`: Undo Deleteの復元意味は維持し、general historyへの合成をGUI-GRID-004へ参照。
- Record Mutation GUIの「General-purpose history Undo/Redo stackは導入しない」とrange/Undo非目標を置換し、本familyをownerにする。record duplicate/reorder/bulk add/deleteの非目標は維持。
- `GUI-TABLE-INT-001` / `GUI-TYPE-INT-003`: initializer widget未固定から上記typed editor必須へ拡張。domain Migrationは変更しない。

## Compatibility Impact

source schema / serialized shape / generated API変更なし。queryはdomain comparison capabilityやcanonical orderingを拡張しない。
historyはlocal state、TSVは明示user inputのcodecでありYAML subset変更ではない。異なるtype列へのpasteでliteral stringがinvalidとなることをpreviewで確認できる。

## Implementation Impact

core: clipboard・query・batch composition。app: snapshot / revision / preview。GUI: range / history / typed initializer。Tauriはthin transport。
既存[authoring tests](../../apps/gui/tests/authoring.test.tsx)のAdd/Delete、64-bit、遅延応答、保存結果と、Table/Type migration testsをregression基盤にする。
現行widgetやtest名を新仕様のauthorityとは扱わない。.NET/CLI grammar/fixture直接更新はscope外。

### Acceptance

| Rule family | 必須evidence |
| --- | --- |
| AUTHORING-BATCH-001/002 | 同PK別occurrence、Added key、existing key混入、stale revision、no-op、invalid値のall-or-none buffer適用 |
| AUTHORING-BATCH-003/004 | quoted TAB/LF/CRLF/quote、末尾TAB/改行、空text、ragged row、`ulong::MAX`、intへ`1.0`、literal `null` / `=...`、null copy拒否 |
| AUTHORING-QUERY | 64-bit比較、ordinal string、invalid/null/同値のsort、search ORとfilter AND、unsupported operator、source不変 |
| GUI-GRID-001/002/003 | filter中paste/fill、境界超過、preview後query変更、編集でrowが消える場合、Addでquery clear、Pending deleteのUndo導線 |
| GUI-GRID-004/005/006 | Add→edit→delete→Undo、Undo DeleteのUndo、Save All部分失敗、clean external reload、ConflictとUndo、IME / text-control precedence |
| typed initializer | nested64-bit、unset/null区別、type変更でPlan失効、invalid initializerとinvalid data Saveの区別 |

focused unit / app / GUI workflow testとtemporary fixture copyで検証し、完成時にrepository required checksを行う。

## Potential ADRs

None identified. 既存shared Rust boundaryの拡張である。

## Open Questions

None identified for this proposed scope. exact codec library、history representation、DTO、widget配置は実装判断。上記詳細案は本proposal全体へのHuman Approval対象であり未承認。

## レビュー（Review）

一括review結果は[0018のReview](0018-desktop-build-delivery.md#review)に集約する。

## 承認記録（Approval Record）

未承認。`Status: Proposed`。0016–0018全体の明示Approval後にownerへatomicに適用する。
