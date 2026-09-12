# GUI仕様: Data Editor Record Mutation

Status: Proposed

## 目的

Data Editor上でselected source data fileへrecordを追加・削除し、Source Creationで作成したempty Data documentからYAML手編集なしでrecord authoringを開始できるようにする。

record structure mutationのcanonical source contractは[Source Record Mutation](../../specs/source-record-mutation.md)、既存record value editとfile Save lifecycleは[Data Editor](spec.md)および[Source Record Edit](../../specs/source-edit.md)が所有する。本仕様はAdd Row / Delete / Undo、draft rowのediting scope、focus、disabled state等のGUI behaviorだけを所有する。

## 規範要件

### GUI-DATA-ROW-001

selected Data documentのTable schemaが全fieldをRequired Primitiveとして宣言している場合、Data Editorはkeyboardでも到達可能な`Add Row` actionを提供しなければならない（MUST）。Add Rowはselected source data fileのlocal bufferへ1つのAdded record draftをappendしなければならず（MUST）、別Data fileまたはlogical Table全体へrecordを追加してはならない（MUST NOT）。

Add Row実行だけでworkspace sourceを即時保存してはならない（MUST NOT）。

### GUI-DATA-ROW-002

Added record draftはschema declaration orderの全fieldをrowとして表示しなければならない（MUST）。初回Save前のAdded record draftでは、Primary Key / Secondary Key構成fieldを含む全fieldをeditableとして扱わなければならない（MUST）。

初回Save success後、そのrowはexisting recordとして通常の[Data Editor](spec.md)編集scopeへ移行し、key field等を引き続き特別にeditableとして扱ってはならない（MUST NOT）。

### GUI-DATA-ROW-003

Added record draftのcell inputはlossless textとしてshared application boundaryへ渡さなければならない（MUST）。`long` / `ulong`をJavaScript `number`等へ変換してはならない（MUST NOT）。

Added record draftがdomain-invalidであっても、validationだけを理由にcell edit、draft保持、file Saveを禁止してはならない（MUST NOT）。invalid stateはshared validation diagnosticとして表示する。

### GUI-DATA-ROW-004

Add Row実行後は、新しいdraft rowをgrid内で識別できるstateを表示しなければならず（MUST）、色だけに依存してはならない（MUST NOT）。keyboard利用者が連続して入力を開始できるよう、原則としてnew rowの先頭fieldへselection / focusを移さなければならない（MUST）。

新規draft rowが複数存在する場合も、それぞれsource file内のlocal append orderで表示しなければならない（MUST）。

### GUI-DATA-ROW-005

TableにNullable / Array / Enum / Flags / Value Object / Custom Type等、初期Add Row scope外のfieldが1つでも含まれる場合、Add Rowを実行可能として表示してはならない（MUST NOT）。利用者はAdd Rowが利用できない理由をData Editorから確認できなければならない（MUST）。

このdisabled stateを理由にexisting recordの表示、existing Required Primitive non-key field edit、Delete Recordを無効化してはならない（MUST NOT）。

### GUI-DATA-ROW-006

existing recordにはkeyboardでも到達可能なDelete actionを提供しなければならない（MUST）。Deleteはselected source occurrenceをlocal `Pending delete`へ遷移させ、即時workspace mutationを行ってはならない（MUST NOT）。

Pending delete rowは、削除予定であることをtext/icon等で識別できなければならず（MUST）、通常editable rowとして入力を受け続けてはならない（MUST NOT）。Save前に`Undo Delete`を実行できなければならない（MUST）。

### GUI-DATA-ROW-007

existing recordに未保存cell editがある状態でDeleteした場合、そのlocal editを破棄してはならない（MUST NOT）。Pending delete中はfinal Save candidateからrecordを除外するが、Undo Delete後にはDelete直前のlocal edit stateを復元しなければならない（MUST）。

Added record draftにDeleteを実行した場合はadditionをcancelしなければならず（MUST）、Pending delete rowとして残す必要はない（MUST NOT）。他のlocal mutationがなければdirty stateは自動的にcleanへ戻らなければならない（MUST）。

### GUI-DATA-ROW-008

Add / Delete / Undo後は現在のlocal Save candidateを対象にbuffer validationを再実行しなければならない（MUST）。pending/staleな旧diagnosticsを現在candidateの確定結果として表示してはならない（MUST NOT）。

Pending deleteによりcandidateから除外されたrecordだけに属するdiagnosticは、最新validation完了後にcurrent Problems / cell markerへ残してはならない（MUST NOT）。Added record draftのdiagnosticは対応row/cellへmapping可能な場合、既存Problems navigation contractに従って移動できなければならない（MUST）。

### GUI-DATA-ROW-009

Diff viewはAdded record draftとPending deleteを含むcurrent file Save candidateをbase snapshotと比較して表示しなければならない（MUST）。gridとDiff間を移動してもAdd / Delete / existing cell editのlocal bufferを保持しなければならない（MUST NOT discard）。

可能な範囲で、Diffからgridへ戻る際は直前に操作していたsurviving row / cellへfocus contextを復元しなければならない（MUST）。

### GUI-DATA-ROW-010

Add / Deleteを含むdirty fileは既存Data Editorのfile単位Save、Save All、Project Reload / switch / close guard、external-change polling、Conflict Compare / Reload / Overwrite、Failure / Outcome Unknown recoveryに従わなければならない（MUST）。

Conflict / Failure / Outcome UnknownでAdded record draft、Pending delete、existing cell editを失ってはならない（MUST NOT）。ReloadまたはDon't Saveでlocal mutationを破棄する場合は既存destructive lifecycle contractに従う。

### GUI-DATA-ROW-011

Add / Deleteを含むdirty fileが存在してもBuild actionを禁止してはならない（MUST NOT）。Buildは保存済みsourceだけを入力とし、Added record draftまたはPending deleteを暗黙Saveしてはならない（MUST NOT）。未保存mutationがBuildへ含まれないことを利用者が認識できる既存表示を維持しなければならない（MUST）。

### GUI-DATA-ROW-012

frontendはrecord YAML rendering、record occurrence resolution、Primitive scalar classification、candidate composition、Primary / Secondary Key validationを独自実装してはならない（MUST NOT）。GUIはshared applicationから得たschema capability、candidate preview、validation diagnostic、Save resultを使用しなければならない（MUST）。

## Interaction details

- `Add Row`はData Editor toolbarまたはgrid近傍の明確なactionとして配置してよい（MAY）。exact placement、icon、tooltipはimplementation detailとする。
- Deleteはrow actionから実行してよい（MAY）。初期sliceではPending deleteがSave前にUndo可能であるため、Delete開始時の追加modal confirmationを必須としない。
- Pending delete rowを完全に非表示にせず、少なくともUndo可能な状態が現在file内で発見可能であることを優先する。
- Added record draftの初期text値は空文字列でよい（MAY）。shared validationが現在candidateのinvalidityを報告し、Save validation gateは設けない。
- General-purpose history Undo/Redo stackは導入しない。`Undo Delete`はPending delete専用の局所recoveryである。

## 検証

少なくともReact状態遷移testとshared core/application testで次を検証する。

- empty `records`のData fileでAdd Rowするとdraft rowが現れ、first cellへfocusされ、fileがdirtyになる。
- new draftではkey fieldもeditableで、`ulong::MAX`相当textがroundingされない。
- Add Row後にdraftをDeleteするとadditionがcancelされ、他変更がなければcleanへ戻る。
- existing edited rowをDeleteするとPending deleteになり、Undoでpre-delete editが復元される。
- Pending deleteをSaveするとselected source occurrenceだけが消え、別同一PK recordは残る。
- complex / nullable / array等を含むTableではAdd Rowがdisabled reason付きで、existing row Deleteは利用可能なままである。
- Add / Delete後のlatest validationだけがProblems / cell markerへ反映される。
- Diffがaddition / deletionを表示し、gridへ戻ってもlocal mutationが残る。
- external conflict、Save failure、Outcome Unknownでlocal Add / Delete stateが失われない。
- dirty BuildがSaveを暗黙実行せず、保存済みsourceだけを使用する。

## Compatibility

既存Data Editorのexisting-record edit behavior、keyboard Save、dirty lifecycle、validation、Diff、Build semanticsを変更しない。Added record draftだけが初回Save前にkey fieldをeditableとする例外であり、Save success後は既存`GUI-DATA-STATE-001`のscopeへ戻る。

本仕様はsource record orderをdomain identityへ昇格させず、logical Tableの既存semanticsを変更しない。

## Open Questions

None identified for the initial Required-Primitive-only Add / source-occurrence Delete GUI slice.

## 非目標

- Nullable / Array / Enum / Flags / Value Object / Custom Typeを含むTableのAdd Row UI。
- `$tags` authoring。
- record duplicate、move / reorder、bulk add / bulk delete。
- range selection、一括paste、fill handle、general Undo/Redo。
- schema / type editor、source file / folder mutation。
- Table-wide aggregate editor。
- Programmable View / Computed / Annotation column。
