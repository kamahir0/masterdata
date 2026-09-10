# GUI仕様: Data Editor

Status: Draft

## 目的

record data YAMLを、source textを直接編集しなくてもtable形式で編集できるmain editor surfaceとして提供する。初期sliceでは、選択中のsource data fileに含まれるrecordsをExcel風のgridで表示・編集し、file単位でdirty / Saveを管理する。

## レイアウト（Layout）

### GUI-DATA-LAYOUT-001

record data YAMLを選択した場合、main areaはspreadsheet形式のgridを表示する。列はlogical Tableのfield、行は選択中source data file内のrecordに対応する。

### GUI-DATA-LAYOUT-002

fieldの表示順は既存schema/domain authorityに従い、GUI独自の列順をdomain意味として保存してはならない。

### GUI-DATA-LAYOUT-003

同じlogical Tableが複数source data fileへ分割されていても、初期sliceのgridは選択中fileに属するrecordだけを表示する。将来のTable横断viewは別surfaceとして追加できる。

## 状態（States）

### GUI-DATA-STATE-001

初期sliceで編集可能なのはRequired Primitiveの非key fieldとする。Primary / Secondary Key構成field、Enum、Value Object、Nullable、Array、Custom Type等はread-onlyとして表示する。

### GUI-DATA-STATE-002

cell変更によりsource data fileが未保存状態になった場合、そのfileをdirtyとして扱う。dirtyの単位はrecordやTableではなくsource data fileである。

### GUI-DATA-STATE-003

同一file内の複数record / field変更は同じdirty bufferに属し、1回のfile Saveでまとめて永続化する。別source fileのdirty stateは独立する。

### GUI-DATA-STATE-004

cleanなsource data fileが外部editor等によって変更された場合、GUIはdisk上の最新内容へ自動Reloadする。利用者の未保存bufferが存在しない状態で、古いsnapshotを表示し続けてはならない。

### GUI-DATA-STATE-005

dirtyなsource data fileが外部変更された場合、GUIはlocal dirty bufferを保持したままconflict状態へ遷移する。外部変更を理由にdirty bufferを自動破棄してはならず、local bufferでdisk内容を自動上書きしてもならない。

## 操作（Interactions）

### GUI-DATA-EDIT-001

利用者は編集可能cellを直接変更できる。初期sliceではExcel風の表形式とcell editingを提供するが、range selection、一括paste、fill handle、複数record同時編集、履歴付きUndo/Redo等のspreadsheet高度操作は必須としない。

### GUI-DATA-SAVE-001

Saveは現在のsource data fileに対する明示操作である。Saveによって別のdirty fileを暗黙に保存してはならない。

### GUI-DATA-SAVE-002

validation resultの有無はSave可否のgateにしない。domain validation上invalidな値でも、filesystem / path safety / external modification等のoperation-level preconditionを満たす限りSave操作自体を禁止しない。

### GUI-DATA-DIFF-001

利用者は未保存変更または保存予定変更に対応するsource diffを確認できなければならない。diffはvalidationやSaveの許可条件ではなく、変更内容を確認するためのsurfaceである。

### GUI-DATA-BUILD-001

source data fileがdirtyでもBuildの開始を禁止しない。Buildは保存済みsourceだけを入力とし、dirty bufferの未保存変更を暗黙に含めてはならない。

### GUI-DATA-BUILD-002

Build開始時にdirty fileが存在する場合、未保存変更がBuildへ含まれないことを利用者が認識できる表示を行う。Buildを理由にSaveまたはSave Allを暗黙実行してはならない。

### GUI-DATA-CONFLICT-001

conflict状態のfileに通常Saveを実行した場合、外部変更を暗黙に上書きせず保存を停止し、少なくともCompare / Reload / Overwriteを選択できるrecovery pathを提示する。

- Compare: local dirty bufferとdisk上の外部変更を比較できる。
- Reload: local dirty bufferを破棄し、disk上の内容を読み直す。破棄を伴うことを利用者が認識できる必要がある。
- Overwrite: 明示操作としてlocal dirty bufferをdiskへ書き込み、外部変更を置き換える。通常Saveと同じ暗黙動作にしてはならない。

## キーボード（Keyboard）

### GUI-DATA-KEY-001

file Saveにはplatform標準のSave操作（例: Cmd/Ctrl+S）を利用できる設計を目指す。exact shortcut、cell navigation、edit開始・確定・cancel keyは後続reviewで確定する。

## フォーカス（Focus）

selected cell、editing cell、validation detail、diff surface間のfocus transitionは後続reviewで定義する。mouse操作だけを唯一の編集経路にしない。

## 検証（Validation）

### GUI-DATA-VAL-001

validation resultは編集体験のfeedbackとして表示するが、Save禁止条件として扱わない。frontend独自のdomain validationを正本にせず、shared application/domain semanticsから得たdiagnosticsを使用する。

### GUI-DATA-VAL-002

編集中bufferの状態と保存済みsourceに対するvalidation resultを利用者が混同しない表示にする。exact timing、cell/row/file/projectへのdiagnostic mappingは後続reviewで確定する。

## エラー（Errors）

### GUI-DATA-ERR-001

filesystem I/O failure、permission、path safety、external modificationとの競合等により安全に永続化できない場合はSave operation failureまたはconflictとして扱う。validation errorとは区別する。

### GUI-DATA-ERR-002

Save failure時に未保存入力を失ってはならない。通常I/O failureや保存結果不明時のexact recovery semanticsはcanonical source-edit仕様と合わせて確定する。

## アクセシビリティ（Accessibility）

gridのrow / column / cell関係、editable/read-only、selected/editing、validation状態を視覚表現だけに依存させない。exact ARIA/grid semanticsとkeyboard behaviorは後続reviewで確定する。

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

JavaScript等の任意code executionを採用するか、expression languageを採用するか、両者を段階的に提供するかは未決定とする。sandbox、performance、determinism、Desktop/Web共通実行、依存関係、保存・共有formatを比較して別途仕様化する。

annotation / computed / presentation情報をTable schemaやMasterMemory runtime dataへ暗黙に混入させない。どの情報をproject sourceとして共有・Git管理するか、user-local view stateとするかも後続仕様で決定する。

## 未解決事項（Open Questions）

- row identityと、同一PKを持ち得るsource recordの選択・表示方法。
- long / ulongを含むexact scalar transportと入力中representation。
- dirty cellを元値へ戻した場合のfile dirty判定。
- source preservation、file Save atomicity、保存結果不明時のrecovery。
- dirty中のfile / project切替、Reload、window close behavior。
- validation表示の場所（cell、row、panel等）とdiff surfaceの具体的layout。
- loading / saving中のselection、editing、shortcut、focus behavior。
- programmable viewのexpression / code model、aggregate semantics、sandbox、evaluation timing、performance budget。
- computed / annotation / presentation definitionのproject-shared / user-local persistence boundary。
