# Current Objective

## Role

この文書はHuman-selectedなcurrent priorityとwork package boundaryの唯一のownerである。
Approved behaviorは[仕様index](specs/README.md)の各canonical specification、現在のStageは
[Development State](execution-state.md)、implementation realityはcode / tests / Gitで確認する。
この文書は未承認のproduct behaviorのimplementation authorityではない。

## Objective

現在のHuman priorityは、**GUIで既存recordを編集・保存し、差分と検証結果を確認できる最初の体験を完成させる**ことである。
2026-09-10に、AddField source commit safetyの完了後、この体験の具体化へ進む推薦にHumanが「進めて」と指示した。

Humanは初期編集対象をRequired Primitiveの非key field、Save / dirty管理をVS Codeに近いsource data file単位、validation resultをSave可否のgateにしない方針として選択した。GUI全体は左にVS Code型のfile / folder Explorer、中央に選択source kindごとのtyped editorを持ち、record Data documentでは列=field・行=recordのspreadsheet型editorを表示する。dirty中Build、external modification、dirty buffer保護、dirty判定、Problems / Diff表示についてもHuman-selected behaviorを反映済みである。

HumanはroutineなGUI interaction detailについて、使い勝手・データ安全性・互換性を大きく左右する選択だけ確認し、それ以外は既存方針と一般的UX慣習に従って仕様refinement側で決定し、実使用後に調整する方針を選択した。

現在、[GUI app shell](gui/app-shell.md)、[Workspace Explorer](gui/explorer/spec.md)、[Data Editor](gui/data-editor/spec.md)、[Source Record Edit](specs/source-edit.md)を`Status: Proposed`まで整理した。review-spec観点では、現在のinitial existing-record authoring contractについてBlocking issueを認めていない。実装前にHuman Approvalを行い、Approved authorityとして確定する必要がある。

## Why now

AddField source commit safetyはcandidate `d18fb43e896921e7ec2ec618c9b71640e9d02545`のfinal verificationを経て完了した。
安全性基盤を積み上げるだけでは、YAMLを手書きせずに編集するという製品の動機は実現しない。
そのためMigration operationを増やす前に、既存の共有validation / build基盤を利用者の編集体験へ接続する。

## Completion boundary

- ProjectをGUIから開き、左のWorkspace Explorerからsource data fileを選択できるようにする。
- 中央のData Editorで、そのfile内recordsをspreadsheet形式で確認し、Required Primitiveの非key fieldを編集できるようにする。
- source data file単位でdirty stateと明示Saveを管理し、同一fileの複数変更を1回のSaveで永続化する。
- validation errorはSave禁止条件にせず、local editor bufferに対するvalidation feedbackをcell markerとProblems panelから確認できるようにする。
- base snapshotとlocal bufferのsource diffをfile単位Diff viewで確認できるようにする。
- source-preserving Saveにより、変更対象外のcomment、quote、indentation、blank line、line ending、record/member order等を保持し、未変更fileをbyte-for-byte保持する。
- external modificationではclean fileを自動Reloadし、dirty fileはConflictとしてlocal bufferを保護し、Compare / Reload / Overwriteの明示recoveryを提供する。
- Project切替 / Project Reload / window close等のbuffer破棄操作ではSave All / Don't Save / Cancelで保護し、通常file navigationではdirty bufferを保持する。
- dirty中でもBuild可能とするが、Buildは保存済みsourceだけを使用し、暗黙Saveしない。
- `long` / `ulong`を含むscalar transportでroundingを起こさない。
- Save failure / Outcome Unknownで未保存入力を失わず、staleなautomatic retry / overwriteを行わない。
- 関連tests、required checks、final verificationでBlockingがないことを確認する。read-only viewerや仕様作成だけでこのObjectiveを完了扱いにしない。

## Explicit non-scope

現時点のpriorityに次は含めない。初期対応範囲の詳細はRFCとProposed仕様を参照する。

- Explorerからの新規folder / Table / record / Value Object / Enum等の作成、rename、delete、move。これらは将来のauthoring modelとして維持する。
- recordの追加・削除、schema編集、RenameField / DropField、MasterReference設計。
- Enum / Value Object / Nullable / Array / Custom Typeやkey fieldの初期編集対応。
- spreadsheetのrange selection、一括paste、fill handle、複数record同時編集、履歴付きUndo/Redo。
- logical Table全体を複数data file横断で一括編集するTable View。
- Programmable View / Computed Column / Annotation Columnのruntime、保存format、expression / JavaScript engine。将来product directionとしてData Editor仕様に記録済みである。
- Standalone / Connected WebとNative Hostの実装、distribution全体。
- GUI Publish、Unity integration全体の同時完成、Git stage / commit / push UI。
- workflow control-planeの再設計、固定agent role / launcher追加。
- Approved semanticsの無承認変更、未確定public API / syntax / formatの実装による先取り。

## Next candidate

このObjective完了後、Explorerからのsource artifact作成、対応型・spreadsheet操作の拡張、Programmable View、Table横断view、Build Profile / Publishへの接続、Unityを含む一連の利用確認を候補として比較する。
RenameField / DropFieldは[Schema Migration仕様](specs/schema-migration.md)に残る後続機能であり、撤回していない。
いずれも自動昇格せず、current realityとproduct valueを確認してHumanが次priorityを選ぶ。

## Relevant authorities

- [Product vision](product/vision.md)
- [GUI仕様index](gui/README.md)
- [GUI app shell（Proposed）](gui/app-shell.md)
- [Workspace Explorer（Proposed）](gui/explorer/spec.md)
- [Data Editor（Proposed）](gui/data-editor/spec.md)
- [Source Record Edit（Proposed）](specs/source-edit.md)
- [最初のrecord authoring RFC（Draft）](rfcs/0005-first-record-authoring-experience.md) — Human-selected product choicesとdecision history
- [YAML subset](specs/yaml-subset.md)、[Table / Key](specs/table-and-keys.md)、[Primitive Types](specs/type-system/primitives.md)
- [Build Selection](specs/build-selection.md)、[Runtime hosts](specs/runtime-hosts.md)、[Schema Migration](specs/schema-migration.md)
- [YAML正本ADR](adr/0001-yaml-is-source-of-truth.md)、[shared core ADR](adr/0002-rust-core-shared-by-cli-and-gui.md)、[host capability ADR](adr/0006-host-capability-composition.md)
- [Specification workflow](contributing/specification-workflow.md)、[Development workflow](execution-workflow.md)
