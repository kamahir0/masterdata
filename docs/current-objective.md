# Current Objective

## Role

この文書はHuman-selectedなcurrent priorityとwork package boundaryの唯一のownerである。
Approved behaviorは[仕様index](specs/README.md)の各canonical specification、現在のStageは
[Development State](execution-state.md)、implementation realityはcode / tests / Gitで確認する。
この文書は未承認のproduct behaviorのimplementation authorityではない。

## Objective

現在のHuman priorityは、**GUIのData Editorからrecordを追加・削除し、Source Creationで作ったempty Data documentをYAML手編集なしで実用的なrecord authoringへ進められる体験を完成させる**ことである。

Source Artifact Creation Objectiveはcandidate `f54bc545494cc40c014825fe64dc1d580edcbf34`のfinal verificationでBlockingなしとなり、2026-09-12に`objective-complete`へ到達した。その後Humanが次priorityとして第一推薦のData Editor record追加・削除を「進める」と選択したため、本Objectiveをcurrent priorityとする。

既存[Source Record Edit](specs/source-edit.md)は既存record member value変更だけを所有し、record追加・削除を明示的にnon-scopeとしている。そのauthorityを黙って拡張せず、record structure mutationのsource-preserving semanticsは新しい[Source Record Mutation proposal](specs/source-record-mutation.md)で所有する。GUI interactionも新しい[Data Editor Record Mutation proposal](gui/data-editor/record-mutation.md)へ分離する。

Added record draftで初回値としてkey fieldを入力できるようにするには、既存Approved `GUI-DATA-STATE-001`のapplicability boundaryを明確化する必要があるため、[spec change 0013](spec-changes/0013-data-editor-added-record-key-editability.md)をProposedとして作成した。existing recordのkey fieldは引き続きread-onlyであり、new draftだけを初回Save前の例外とする。

Humanが既に選択した方針どおり、使い勝手・データ安全性・互換性を大きく左右しないroutine interaction detailは既存UI方針、platform convention、accessibility、testabilityに従って仕様側で決定し、実使用後に必要なら調整する。

2026-09-12時点で上記2つの新規specificationとspec change 0013は`Status: Proposed`である。self-reviewでは既存Approved authorityとの未処理conflict、Open Question、実装不能なsemantic gapは残っていない。implementation authorityにするにはHuman Approvalと、0013のcanonical Data Editor specificationへの適用が必要である。

## Why now

現在のGUIはWorkspace ExplorerからTable / Data / Value Object / Enum / Flags / Custom Typeを新規作成でき、既存Data documentではRequired Primitive non-key fieldを編集・validation・Diff・file Saveできる。一方、新規Data documentはempty `records`から開始するため、最初のrecordをGUIで追加する手段がなく、creation直後にYAML手編集へ戻る断点が残っている。

最初のrecord追加・既存record削除をfile単位dirty / Save、source-preserving patch、lost-update recoveryへ統合することで、Project作成から基本的なrecord authoringまでGUIだけで連続して行える範囲を拡張する。

## Completion boundary

- 選択中Data documentのTableが、全fieldをRequired Primitiveとして宣言している場合、Data Editorから`Add Row`を実行し、selected source fileの末尾へ新しいrecord draftを追加できるようにする。
- 新規record draftはschema declaration orderの全fieldを持ち、初回Save前はPrimary / Secondary Key構成fieldを含む全fieldをlosslessなtext入力で編集できるようにする。`long` / `ulong`をfrontendのlossy numeric representationへ変換しない。
- Add Row自体をfile-local dirty mutationとして扱い、validation errorをSave gateにしない。新規draftのdomain-invalid valueもshared validationでdiagnosticを返し、Source of Truthへ保存するかどうかは既存explicit Save workflowに従う。
- TableにNullable / Array / Enum / Flags / Value Object / Custom Type等、初期row input scope外のfieldが含まれる場合、Add Rowを実行可能として誤表示せず、未対応理由を確認できるようにする。既存recordの表示・既存対応fieldの編集・record削除まで無効化してはならない。
- 既存recordはPrimary Key valueではなく、base snapshot内の選択source occurrenceを対象としてDeleteできるようにする。同一PK valueを持つ別recordを誤って削除しない。
- 既存recordをDeleteした時点では即時disk mutationせず、そのfileのlocal bufferで`Pending delete`として識別でき、Save前にUndoできるようにする。削除対象recordの既存local cell editsはDelete中も復元可能なbuffer stateとして保持する。
- 新規record draftをSave前にDeleteした場合は、そのadditionをcancelし、他の変更がなければfileをcleanへ戻せるようにする。
- 同一file内の既存cell edit、record addition、record deletionを1つのSave candidateへcompositionし、1回のfile Saveでcommitする。削除対象recordへのvalue editとDeleteが共存する場合はDeleteが最終candidateを所有し、Undo時にはDelete前のlocal edit stateを復元する。
- Addはselected source fileの`records`末尾へdeterministicに挿入し、Deleteはselected source occurrenceだけを除去する。対象構造変更と無関係なcomments、blank lines、line ending、quote / indentation、record/member order、その他source textを保持する。
- GUI Source Creationが生成するempty `records: []`をAdd Rowで安全にblock sequenceへ展開できるようにする。source shapeを安全に再特定できない場合はfull-file reserializationや近似patchへfallbackせず失敗する。
- structure mutation後もbuffer validation、Problems、Diffは現在のlocal Save candidateを対象とし、stale previewをcurrent resultとして表示しない。
- Save / Save All、external modification、Conflict Compare / Reload / Overwrite、Failure / Outcome Unknown recovery、dirty lifecycle、Buildは保存済みsourceのみという既存Data Editor contractをrecord structure mutationにも適用する。
- record addition / deletionはBuild、Publish、Git、schema Migration、generated artifact更新を暗黙に開始しない。
- source patch derivation、record value conversion、schema/validation semanticsをfrontendへ複製せず、shared core/application boundaryへ置く。
- focused core/application tests、React状態遷移test、repository required checks、final verificationでBlockingがないことを確認する。

## Explicit non-scope

現Objectiveでは次を含めない。

- Nullable / Array / Enum / Flags / Value Object / Custom Typeを含むTableへの新規record input UI。
- `$tags`の追加・編集。
- record duplicate、record move / reorder、drag & drop、複数record一括追加・一括削除。
- spreadsheet range selection、一括paste、fill handle、履歴付きgeneral Undo/Redo。Pending deleteに対する局所Undoは本Objectiveに含む。
- Table横断aggregate view、複数Data fileを1つのgridとして編集すること。
- schema / type / field / keyの追加・削除・rename・編集。
- source file / folderのrename、delete、move、duplicate。
- multi-file atomic transaction。
- Programmable View / Computed Column / Annotation Column。
- Build Profile / Publish、Standalone / Connected Web全体、Native Host lifecycle、distributionの同時完成。

## Next candidate

このObjective完了後は、Table / Type専用editor、complex fieldを含むrecord追加、source rename / delete / move、spreadsheet操作拡張、Programmable View、Build Profile / Publish、Standalone / Connected Webへのauthoring surface展開を候補として比較する。

次priorityは自動昇格せず、current realityとproduct valueを確認してHumanが選択する。

## Relevant authorities

- [Product vision](product/vision.md)
- [GUI specification index](gui/README.md)
- [GUI app shell](gui/app-shell.md)
- [Workspace Explorer](gui/explorer/spec.md)
- [Data Editor](gui/data-editor/spec.md) — existing-record GUI authority
- [Data Editor Record Mutation proposal](gui/data-editor/record-mutation.md) — `Status: Proposed`
- [Source Record Edit](specs/source-edit.md) — existing-member editとfile Save safetyのauthority
- [Source Record Mutation proposal](specs/source-record-mutation.md) — `Status: Proposed`
- [Spec change 0013: Added record draft key editability](spec-changes/0013-data-editor-added-record-key-editability.md) — `Status: Proposed`
- [Source Artifact Creation](specs/source-creation.md) — empty Data document creationのauthority
- [Masterdata YAML subset](specs/yaml-subset.md)
- [Table / Primary Key / Secondary Key](specs/table-and-keys.md)
- [Primitive Types](specs/type-system/primitives.md)
- [Field Modifiers](specs/type-system/field-modifiers.md)
- [Runtime hosts / capability](specs/runtime-hosts.md)
- [YAML Source of Truth ADR](adr/0001-yaml-is-source-of-truth.md)
- [shared core ADR](adr/0002-rust-core-shared-by-cli-and-gui.md)
- [host capability ADR](adr/0006-host-capability-composition.md)
- [Specification workflow](contributing/specification-workflow.md)
- [Development workflow](execution-workflow.md)
