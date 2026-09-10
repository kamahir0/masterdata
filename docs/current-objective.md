# Current Objective

## Role

この文書はHuman-selectedなcurrent priorityとwork package boundaryの唯一のownerである。
Approved behaviorは[仕様index](specs/README.md)の各canonical specification、現在のStageは
[Development State](execution-state.md)、implementation realityはcode / tests / Gitで確認する。
この文書は未承認のproduct behaviorのimplementation authorityではない。

## Objective

現在のHuman priorityは、**GUIで既存recordを編集・保存し、差分と検証結果を確認できる最初の体験を完成させる**ことである。
2026-09-10に、AddField source commit safetyの完了後、この体験の具体化へ進む推薦にHumanが「進めて」と指示した。
続いてHumanは、初期編集対象をRequired Primitiveの非key field、Save / dirty管理をVS Codeに近いsource data file単位、
validation resultをSave可否のgateにしない方針として選択した。
さらにGUI全体は、左にVS Code型のfile / folder Explorer、中央に選択source kindごとのtyped editorを持ち、record data YAMLでは列=field・行=recordのspreadsheet型editorを表示する方向を選択した。

[最初のGUIレコード編集・保存体験RFC](rfcs/0005-first-record-authoring-experience.md)と[Workspace Explorer](gui/explorer/spec.md)、[Data Editor](gui/data-editor/spec.md)へproduct choiceを記録し、
必要なGUI / shared source-edit contractを仕様化・review・Human Approvalへ進める。
readiness gateを満たしたら同じObjectiveの実装へ進み、UI操作と保存結果まで検証する。
現時点ではimplementation-readyではなく、保存・競合・dirty lifecycle等のobservable detailをcanonical Draftで閉じる必要がある。

## Why now

AddField source commit safetyはcandidate `d18fb43e896921e7ec2ec618c9b71640e9d02545`のfinal verificationを経て完了した。
安全性基盤を積み上げるだけでは、YAMLを手書きせずに編集するという製品の動機は実現しない。
そのためMigration operationを増やす前に、既存の共有validation / build基盤を利用者の編集体験へ接続する。

## Completion boundary

- 左のWorkspace Explorerからsource data fileを選択し、中央のData Editorでそのfile内recordsをspreadsheet形式で確認・編集できるようにする。
- 初期の編集対象と操作、file単位Save・競合・validation表示・dirty policyを明確にし、各canonical ownerへ仕様化する。
- source正本、型・Table・Build Selection、shared application / host boundaryを既存Approved authorityから参照する。
- 必要なHuman decision、Specification Gap、review、Human Approvalを閉じた後に実装する。
- 選定された範囲で、Projectを開く → Explorerでdata fileを選ぶ → record / fieldを編集 → 差分・検証結果を確認 → fileを保存 → 開き直して確認、を通す。
- validation errorはSave禁止条件にせず、保存済みsourceに対するvalidation resultを利用者が確認できるようにする。
- external modification、保存失敗、未保存変更の保護を承認されたcontractに従って検証する。
- 関連tests、required checks、final verificationでBlockingがないことを確認する。read-only viewerや仕様作成だけでこのObjectiveを完了扱いにしない。

## Explicit non-scope

現時点のpriorityに次は含めない。初期対応範囲の詳細はRFCとGUI Draftを参照する。

- Explorerからの新規folder / Table / record / Value Object / Enum等の作成、rename、delete、move。これらは将来のauthoring modelとしては維持する。
- RenameField / DropField、MasterReference、Build Profileの別機能開発。
- Enum / Value Object / Nullable / Array / Custom Typeやkey fieldの初期編集対応。
- spreadsheetのrange selection、一括paste、fill handle、複数record同時編集、履歴付きUndo/Redo。
- Standalone / Connected WebとNative Hostの実装、distribution全体。
- GUI Publish、Unity integration全体の同時完成、Git commit/push UI。
- workflow control-planeの再設計、固定agent role / launcher追加。
- Approved semanticsの無承認変更、未確定public API / syntax / formatの実装による先取り。

## Next candidate

このObjective完了後、Explorerからのsource artifact作成、対応型・spreadsheet操作の拡張、Table横断view、Build Profile / Publishへの接続、Unityを含む一連の利用確認を候補として比較する。
RenameField / DropFieldは[Schema Migration仕様](specs/schema-migration.md)に残る後続機能であり、撤回していない。
いずれも自動昇格せず、current realityとproduct valueを確認してHumanが次priorityを選ぶ。

## Relevant authorities

- [Product vision](product/vision.md)
- [GUI仕様index](gui/README.md)、[GUI app shell（Draft）](gui/app-shell.md)
- [Workspace Explorer（Draft）](gui/explorer/spec.md)、[Data Editor（Draft）](gui/data-editor/spec.md)
- [最初のrecord authoring RFC（Draft）](rfcs/0005-first-record-authoring-experience.md) — Human-selected product choicesと残るOpen Questions
- [YAML subset](specs/yaml-subset.md)、[Table / Key](specs/table-and-keys.md)、[Type System](specs/type-system/README.md)
- [Build Selection](specs/build-selection.md)、[Runtime hosts](specs/runtime-hosts.md)
- [YAML正本ADR](adr/0001-yaml-is-source-of-truth.md)、[shared core ADR](adr/0002-rust-core-shared-by-cli-and-gui.md)、[host capability ADR](adr/0006-host-capability-composition.md)
- [Specification workflow](contributing/specification-workflow.md)、[Development workflow](execution-workflow.md)
