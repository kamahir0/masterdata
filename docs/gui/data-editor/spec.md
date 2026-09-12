# GUI仕様: Data Editor

Status: Approved

## 目的

record Data documentをsource textへ直接触れずtable形式で編集できるmain editor surfaceとして提供する。初期sliceでは、選択中source data fileに含まれるrecordsをspreadsheet形式で表示・編集し、file単位でdirty / Saveを管理する。

source保存のcanonical contractは[Source Record Edit](../../specs/source-edit.md)が所有し、Data EditorはそのGUI interactionを定義する。

## レイアウト（Layout）

### GUI-DATA-LAYOUT-001

record Data documentを選択した場合、main areaはspreadsheet形式のgridを表示しなければならない（MUST）。列はlogical Tableのfield、行は選択中source data file内のrecord occurrenceに対応する。

### GUI-DATA-LAYOUT-002

fieldの表示順は[Table / Key](../../specs/table-and-keys.md)のschema declaration orderに従わなければならない（MUST）。GUI独自の列順をdomain意味として保存してはならない（MUST NOT）。

### GUI-DATA-LAYOUT-003

同じlogical Tableが複数source data fileへ分割されていても、初期sliceのgridは選択中fileに属するrecordsだけを表示しなければならない（MUST）。rowは選択file内のsource record orderで表示し、同一Primary Key valueを持つ複数source recordが存在してもdeduplicateしてはならない（MUST NOT）。この表示順をdomain / binary semanticsへ昇格させてはならない（MUST NOT）。

### GUI-DATA-LAYOUT-004

validation diagnosticsの一覧は、main gridの編集を妨げない下部`Problems` panelで表示できなければならない（MUST）。Problems panelはmodalとして編集を占有してはならず（MUST NOT）、開閉可能な補助surfaceとして扱う。

### GUI-DATA-LAYOUT-005

source diffはmain gridとは別のfile単位`Diff` view / editorとして表示しなければならない（MUST）。Diffを確認するためにdirty bufferを保存または破棄する必要があってはならず（MUST NOT）、Diff表示自体をSaveやvalidationのgateとして扱ってはならない（MUST NOT）。

## 状態（States）

### GUI-DATA-STATE-001

base snapshotに存在するexisting recordの通常cell editで編集可能なのはRequired Primitiveの非key fieldでなければならない（MUST）。Primary / Secondary Key構成field、Enum、Value Object、Nullable、Array、Custom Type等はexisting recordではread-onlyとして表示しなければならない（MUST）。unsupported fieldを含むTable全体を非表示にしてはならない（MUST NOT）。

base snapshotに存在しないAdded record draftについては、別のApproved GUI specificationが初回Save前のediting scopeを定義してよい（MAY）。この例外からexisting recordのkey field editabilityを導出してはならない（MUST NOT）。Added record draftがSave成功して新しいbase snapshotのexisting recordになった後は、通常の本requirementのscopeへ戻らなければならない（MUST）。

### GUI-DATA-STATE-002

cell変更によりsource data fileが未保存状態になった場合、そのfileをdirtyとして扱わなければならない（MUST）。dirtyの単位はrecordやTableではなくsource data fileである。

### GUI-DATA-STATE-003

同一file内の複数record / field変更は同じdirty bufferに属し、1回のfile Saveでまとめて永続化しなければならない（MUST）。別source fileのdirty stateは独立して保持しなければならない（MUST）。

### GUI-DATA-STATE-004

cleanなsource data fileが外部変更された場合、GUIはdisk / workspace上の最新内容へ自動Reloadしなければならない（MUST）。利用者の未保存bufferが存在しない状態で古いsnapshotを編集可能に表示し続けてはならない（MUST NOT）。

### GUI-DATA-STATE-005

dirtyなsource data fileが外部変更された場合、GUIはlocal dirty bufferを保持したままConflict状態へ遷移しなければならない（MUST）。外部変更を理由にdirty bufferを自動破棄してはならず（MUST NOT）、local bufferでdisk内容を自動上書きしてもならない（MUST NOT）。

### GUI-DATA-STATE-006

複数source data fileは同時にdirtyであってよい（MAY）。file / record / Table間のnavigationだけを理由にdirty bufferを破棄、保存、または確認dialog表示してはならない（MUST NOT）。

### GUI-DATA-STATE-007

dirty stateは「一度編集したか」ではなく、現在のlocal bufferと最後に保存・読込されたbase snapshotとの差分有無で決定しなければならない（MUST）。全変更をbase snapshotと同一の内容へ戻した場合、そのsource data fileは自動的にcleanへ戻らなければならない（MUST）。

### GUI-DATA-STATE-008

選択fileのload中は進行状態を表示し、未確定または前fileのgrid contentを新しいfileの編集可能stateとして表示してはならない（MUST NOT）。Save中はそのSave candidateに属するfileの編集を一時的に停止してよい（MAY）が、他fileのdirty bufferを破棄してはならない（MUST NOT）。

## 操作（Interactions）

### GUI-DATA-EDIT-001

利用者は編集可能cellを直接変更できなければならない（MUST）。初期sliceではsingle-cell editingを提供する。range selection、一括paste、fill handle、複数record同時編集、履歴付きUndo/Redo等の高度なspreadsheet操作は必須としない。

### GUI-DATA-EDIT-002

Primitive cell editorは入力をlosslessにshared application boundaryへ渡せなければならない（MUST）。特に`long` / `ulong`をfrontendのlossy numeric representationへ変換してはならず（MUST NOT）、[SOURCE-EDIT-003](../../specs/source-edit.md)に従わなければならない（MUST）。

active cell editを確定した時点でlocal bufferへ反映し、domain validation上invalidな入力であってもvalidationだけを理由に編集確定を拒否してはならない（MUST NOT）。exact widget種別やscalar formatting controlはimplementation detailとする。

### GUI-DATA-SAVE-001

Saveは現在activeなsource data fileに対する明示操作でなければならない（MUST）。Saveによって別のdirty fileを暗黙に保存してはならない（MUST NOT）。

### GUI-DATA-SAVE-002

validation resultの有無はSave可否のgateにしてはならない（MUST NOT）。domain validation上invalidな値でも、[Source Record Edit](../../specs/source-edit.md)のsource commit safetyを満たす限り、validationだけを理由にSaveを禁止してはならない（MUST NOT）。

### GUI-DATA-SAVE-003

Project切替、Project Reload、window close等、現在保持しているdirty bufferを失う操作を開始する場合、GUIは少なくとも`Save All` / `Don't Save` / `Cancel`を選択できる確認を提示しなければならない（MUST）。

- `Save All`: dirtyな各fileの保存を試行し、すべて安全に保存できた場合だけ元の操作を続行する。Conflict、Failure、Outcome Unknownがある場合は元の操作を完了せず、未保存bufferを保持してrecoveryを提示する。
- `Don't Save`: dirty bufferを破棄して元の操作を続行する。破棄を伴うことを利用者が認識できなければならない。
- `Cancel`: 元の操作を中止し、dirty bufferをそのまま保持する。

### GUI-DATA-SAVE-004

file / record / Table間の通常navigationでは保存確認を表示せず、対象fileのdirty bufferを保持したまま別selectionへ移動できなければならない（MUST）。

### GUI-DATA-SAVE-005

Saveの`Success`後はcommitされたcontentを新しいbase snapshotとしてdirtyを再評価しなければならない（MUST）。`Conflict` / `Failure` / `Outcome Unknown`ではlocal inputを失ってはならない（MUST NOT）。Outcome Unknownではsourceを再取得してactual workspace stateを確認するまでautomatic retry / overwriteを行ってはならない（MUST NOT）。

### GUI-DATA-DIFF-001

利用者は未保存変更または保存予定変更に対応するsource diffを確認できなければならない（MUST）。diffはvalidationやSaveの許可条件ではなく、変更内容確認のsurfaceである。

### GUI-DATA-DIFF-002

Diff viewは選択中source data fileのbase snapshotと現在のlocal bufferに対応するSave candidateとの差分をfile単位で表示しなければならない（MUST）。Diff viewとgridの間を移動してもdirty bufferを保持し、可能な範囲でgrid selection / focus contextを復元しなければならない（MUST）。

### GUI-DATA-BUILD-001

source data fileがdirtyでもBuildの開始を禁止してはならない（MUST NOT）。Buildは保存済みsourceだけを入力とし、dirty bufferの未保存変更を暗黙に含めてはならない（MUST NOT）。

### GUI-DATA-BUILD-002

Build開始時にdirty fileが存在する場合、未保存変更がBuildへ含まれないことを利用者が認識できる表示を行わなければならない（MUST）。Buildを理由にSaveまたはSave Allを暗黙実行してはならない（MUST NOT）。

### GUI-DATA-CONFLICT-001

Conflict状態のfileに通常Saveを実行した場合、外部変更を暗黙に上書きせず保存を停止し、少なくとも`Compare` / `Reload` / `Overwrite`を選択できるrecovery pathを提示しなければならない（MUST）。

- `Compare`: local dirty bufferとcurrent external sourceを比較する。
- `Reload`: local dirty bufferを破棄し、workspace contentを読み直す。破棄を伴うことを利用者が認識できなければならない。
- `Overwrite`: [SOURCE-EDIT-009](../../specs/source-edit.md)のexplicit authorization / recheckを経てlocal Save candidateでcurrent external contentを置き換える。

## キーボード（Keyboard）

### GUI-DATA-KEY-001

file Saveにはplatform標準のSave shortcut（macOSではCmd+S、その他一般的DesktopではCtrl+S）を提供しなければならない（MUST）。shortcutはactiveなsource data fileだけを通常Saveし、Save Allとして動作してはならない（MUST NOT）。

### GUI-DATA-KEY-002

gridはkeyboardだけでcell selectionとsingle-cell editingを行えなければならない（MUST）。Arrow keysでselection移動、EnterまたはF2相当でedit開始、Escapeで現在のcell edit cancel、Enter / Tab相当でedit確定と移動を可能にする。platform / component library差によりexact keyを追加してもよいが（MAY）、keyboard-only編集経路を失ってはならない（MUST NOT）。

## フォーカス（Focus）

selected cell、editing cell、Problems panel、Diff view間はkeyboardでも移動可能でなければならない（MUST）。Problems entryから対応cellへ移動した場合はそのcellをselection / focus対象にしなければならない（MUST）。Diff viewからgridへ戻る場合は可能な範囲で直前のselection / focus contextを復元しなければならない（MUST）。

## 検証（Validation）

### GUI-DATA-VAL-001

validation resultは編集体験のfeedbackとして表示するが、Save禁止条件として扱ってはならない（MUST NOT）。frontend独自のdomain validationを正本にせず、shared application/domain semanticsから得たdiagnosticsを使用しなければならない（MUST）。

### GUI-DATA-VAL-002

validationは現在のeditor bufferに対して自動実行し、buffer変更後は短いdebounceを置いて再評価しなければならない（MUST）。Saveをvalidation開始条件にしてはならず（MUST NOT）、利用者は保存前でも最新bufferに対応するdiagnosticsを確認できなければならない（MUST）。debounceの具体的時間は実装調整値とする。

### GUI-DATA-VAL-003

cellへ対応づけられるdiagnosticが存在する場合、該当cellはgrid内でvalidation状態を識別できるinline marker / decorationを持たなければならない（MUST）。状態伝達を背景色・文字色など色だけに依存させてはならない（MUST NOT）。

### GUI-DATA-VAL-004

Problems panelはdiagnosticsを一覧表示し、cellへ対応づけ可能なentryを選択した場合は該当file / record / fieldのcellへ移動できなければならない（MUST）。cellへ一意に対応づけられないfile / project-level diagnosticもProblems panelから失ってはならない（MUST NOT）。

### GUI-DATA-VAL-005

buffer変更後にvalidationが未完了の場合、直前のdiagnosticsを現在bufferの確定結果であるかのように表示してはならない（MUST NOT）。pending / staleであることを識別できる状態を持ち、最新結果だけをProblemsとcell markerへ適用しなければならない（MUST）。

### GUI-DATA-VAL-006

validation operation自体が失敗した場合、validation resultが利用不能であることをProblemsまたはeditor statusから確認できなければならない（MUST）。validation execution failureをSave禁止条件へ変換してはならず（MUST NOT）、最後の成功diagnosticsを現在bufferの最新結果として偽ってはならない（MUST NOT）。

## エラー（Errors）

### GUI-DATA-ERR-001

filesystem I/O failure、permission、path safety、external modificationとのConflict等により安全に永続化できない場合は、[Source Record Edit](../../specs/source-edit.md)のSave resultに対応するoperation failureとして表示しなければならない（MUST）。validation errorとは区別しなければならない（MUST）。

### GUI-DATA-ERR-002

Save failure時に未保存入力を失ってはならない（MUST NOT）。Failure / Outcome Unknownではrecovery actionを利用者が確認できなければならない（MUST）。

## アクセシビリティ（Accessibility）

gridのrow / column / cell関係、editable / read-only、selected / editing、dirty、validation、Conflict等の状態をassistive technologyから識別可能にしなければならない（MUST）。Problems entryと対応cellの関係もkeyboard / screen reader利用時に辿れる必要がある。状態伝達を色だけに依存させてはならない（MUST NOT）。

## 参照artifact（Reference Artifacts）

None.

## 初期sliceの非目標

- recordの追加・削除。
- schema、Table、Value Object、Enum等の編集・作成。
- key fieldの編集。
- logical Table全体を複数file横断で一括編集するview。
- range operation、一括paste、fill handle、multi-cell editing、履歴付きUndo/Redo。
- Git stage / commit / push。

これらはExplorer + typed editorsという全体構造の将来拡張を妨げない。

## 将来拡張方向: Programmable View / Computed Columns

Human-requestedな将来product directionとして、Data Editorはschema由来の固定field列だけでなく、authoringを補助するvirtual / annotation列を追加できる方向を保持する。これは現Current Objectiveのimplementation scopeではなく、runtime language、保存format、security modelを現時点で固定するものでもない。

将来的なcolumn modelとして、少なくとも次を区別できる設計を検討する。

- Source column: Table schemaのfieldに対応し、record YAMLへ保存される列。
- Computed / View column: source recordを書き換えず、式またはprogrammable logicから導出される仮想列。
- Annotation column: コメント、確認状態、作業メモ等、runtime master dataとは分離されたauthoring補助列。

Computed / View columnでは、Excel関数に近いexpressionまたはより自由なprogrammable logicにより、current recordだけでなく複数recordを参照した計算を可能にする方向とする。例として、複数recordを`groupId`でgroupingし、各recordの`weight`をgroup内合計で割って確率を表示するようなaggregate計算を想定する。

同じprogrammable evaluation基盤から、cell / row / recordのpresentationを導出できる方向も保持する。条件付き書式の固定UIに限定せず、background color、text color、emphasis等を計算結果やrecord集合に応じて決定できることを目指す。

JavaScript等の任意code executionを採用するか、expression languageを採用するか、両者を段階的に提供するかは未決定とする。sandbox、performance、determinism、Desktop / Web共通実行、依存関係、保存・共有formatを比較して別途仕様化する。

annotation / computed / presentation情報をTable schemaやMasterMemory runtime dataへ暗黙に混入させない。どの情報をproject sourceとして共有・Git管理するか、user-local view stateとするかも後続仕様で決定する。

## 未解決事項（Open Questions）

None identified for the initial existing-record Data Editor. Programmable View / Computed / Annotation column、Table横断view、高度なspreadsheet操作、layout customization等はcurrent Objective外の将来機能として別途仕様化する。
