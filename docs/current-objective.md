# Current Objective

## Role

この文書はHuman-selectedなcurrent priorityとwork package boundaryの唯一のownerである。
Approved behaviorは[仕様index](specs/README.md)の各canonical specification、現在のStageは
[Development State](execution-state.md)、implementation realityはcode / tests / Gitで確認する。
この文書は未承認のproduct behaviorのimplementation authorityではない。

## Objective

現在のHuman priorityは、**GUIのTable EditorからSchema Migration v1の`AddField` / `RenameField` / `DropField`をplan・diff確認付きで安全に実行し、既存Table schemaをYAML手編集なしで変更できる最初のschema-aware authoring体験を完成させる**ことである。

Data Editor record mutation Objectiveはcandidate `71d155d4f9bd76af932af61481ac40f9bc47d2ef`のfinal verificationでBlockingなしとなり、2026-09-13時点で`objective-complete`に到達している。その後Humanが、次候補として推薦したTable Editor v1について「進めて」と選択したため、本Objectiveをcurrent priorityとする。

Schema transformation semanticsのauthorityは既存Approved [Schema Migration v1](specs/schema-migration.md)であり、本ObjectiveはGUI convenienceのために別のfield identity、key allocation、dependency rewrite、source rewrite、transaction semanticsを発明しない。GUI observable workflowのauthorityはApproved [Table Editor](gui/table-editor/spec.md)である。2026-09-13にHumanがTable Editor v1とspec change 0014を承認し、Recovery Requiredのcross-surface gateも[GUI app shell](gui/app-shell.md)へatomicに適用した。必要なobservable behavior、completion boundary、failure semantics、non-scopeはApproved authorityから決定でき、未解決のSpecification Gap / Human decision / Approvalはないため、implementation-readyである。

implementation realityでは、Approved Migration v1が`AddField` / `RenameField` / `DropField`を定義している一方、current `masterdata-core`の`MigrationCommand`は`AddField`のみ実装済みである。この差はcanonical specificationを狭める理由ではなく、Approved behaviorへ実装を追随させるimplementation gapとして扱う。本Objectiveにはshared core/applicationでの`RenameField` / `DropField`実装と、そのGUI Table Editorへの接続を含める。

Humanが既に選択した方針どおり、使い勝手・データ安全性・互換性を大きく左右しないroutine interaction detailは既存UI方針、platform convention、accessibility、testabilityに従って仕様側で決定し、実使用後に必要なら調整する。

## Why now

現在のGUIはWorkspace ExplorerからTable / Data / Value Object / Enum / Flags / Custom Typeを新規作成でき、Data documentではrecordの追加・削除・Required Primitive value編集・validation・Diff・SaveまでGUI内で行える。一方、作成済みTable schemaを変更する専用editorはなく、field追加・rename・dropではYAML手編集または別surfaceへ戻る断点が残っている。

Product Visionはschema-aware editingを長期方向として明示しており、Schema Migration v1にはsource-preserving transformation、deterministic Plan、destructive authorization、lost-update preflight、multi-file rollback / Recovery Requiredまで既にApproved contractがある。次にそのshared semanticsをTable Editorへ接続することで、新しいdomain ruleを増やさずschema authoringの主要な断点を閉じられる。

## Completion boundary

- Workspace ExplorerでTable schema documentを選択した場合、folder pathではなくshared source semantics / document kindに基づいてTable Editorを開く。
- Table Editorはlogical Table identity、schema source provenance、field declaration order、MessagePack key、type / modifier、Primary Key、Secondary Keyをshared application snapshotから表示し、frontend独自にYAMLをparseしてschema meaningを再構成しない。
- 初期mutation surfaceはApproved Migration v1の`AddField` / `RenameField` / `DropField`に限定する。field type変更、field reorder、MessagePack key変更、Primary / Secondary Key編集、Table rename等を暗黙に実装しない。
- `AddField`ではfield key、name、type、Nullable / Array modifier、および必要なexplicit constant initializerを入力できる。initializerやtype/value validityはshared Migration / Type System semanticsで判定し、frontendで簡易YAML/type validatorを複製しない。既存recordがある場合のinitializer requirementもMIGRATION-006へ委譲する。
- `RenameField`はlogical Table identity + current field nameをsemantic selectorとして使用し、MessagePack keyを変更しない。Primary / Secondary Key等のApproved dependency updateはshared Migration semanticsへ委譲し、frontendで文字列置換しない。
- `DropField`はdestructive operationとして明示し、mutation開始にはGUI confirmationとは別にbackendへmachine-actionable destructive authorizationを渡す。依存fieldを黙って削除・修復して成功扱いしない。
- source mutation前に必ずdeterministic Migration Planを作成し、operation / target、affected source files、affected record count、diagnostics、destructive stateを利用者が確認できるようにする。
- Planからaffected sourceごとのbefore / after Diffを確認できるようにする。Plan / Diffの表示自体はsourceを変更しない。
- Applyは現在表示中のPlanと同じsemantic command / base snapshotに対してのみ実行する。source/config/membershipがstaleならmutationせずstructured errorを表示し、re-planを要求する。
- Migration Planが変更対象として示すsource fileにData Editorのdirty bufferが存在する場合、Applyを開始せず、対象dirty fileを識別できるようにする。unrelated dirty bufferだけを理由にPlanまたはApplyを全面禁止せず、それらのbufferを保持する。
- successful Migration後はaffected sourceをworkspace authorityから再取得し、Table Editorを最新schemaへ更新する。affected clean Data Editor snapshotはstaleなままeditableにせず再読込し、unrelated dirty bufferは保持する。
- commit failure + rollback success / mutation未開始はSuccessと区別し、operation inputとPlan情報を失わずretry / re-planできる状態を保つ。`Recovery Required`ではその状態を明示し、Approved MIGRATION-010に従って安全なsource stateを再確立するまでGUIから追加のintentional source mutationを開始しない。
- Migration成功はBuild / Publish / Git / generated artifact更新を暗黙に開始しない。必要なら既存Buildを別operationとして実行する。
- Tauri frontendはfilesystem、YAML rewrite、Migration dependency resolution、source transactionを実装せず、shared Native Application Serviceを薄く呼び出す。
- current implementation gapであるshared `RenameField` / `DropField`をApproved Schema Migration v1どおり実装し、Add/Rename/Dropすべてでsource-preserving patch、postcondition、stale-plan preflight、rollback / Recovery Required semanticsを維持する。
- focused core/application regression、Tauri adapter test、React状態遷移test、repository required checks、final verificationでBlockingがないことを確認する。

## Explicit non-scope

現Objectiveでは次を含めない。

- field type変更、Nullable / Array modifier変更、field reorder、MessagePack key変更。
- Primary Key / Secondary Keyの追加・削除・reorder・nonUnique変更。
- Table identity / `csharpName`の変更。
- Value Object / Enum / Flags / Custom Typeの既存definition編集を行うType Editor。
- schema file / folderのrename、delete、move、duplicate。
- Data Editorのcomplex field record input拡張。
- arbitrary raw-YAML schema editor、formatter、general text editor。
- Migration v1外のSQL-like language、bulk migration script、ChangeFieldType、cross-table arbitrary transform。
- Build Profile / Publish、Standalone / Connected Web全体、Native Host lifecycle、distributionの同時完成。
- crash / power-lossまで含むglobal filesystem transaction保証。

## Next candidate

このObjective完了後は、Type Editor、complex fieldを含むrecord追加、Primary / Secondary Key editingを含む次段schema authoring、source rename / delete / move、spreadsheet操作拡張、Programmable View、Build Profile / Publish、Standalone / Connected Webへのauthoring surface展開を候補として比較する。

次priorityは自動昇格せず、current realityとproduct valueを確認してHumanが選択する。

## Relevant authorities

- [Product vision](product/vision.md)
- [GUI specification index](gui/README.md)
- [GUI app shell](gui/app-shell.md)
- [Workspace Explorer](gui/explorer/spec.md)
- [Table Editor](gui/table-editor/spec.md) — Approved GUI workflow authority
- [Data Editor](gui/data-editor/spec.md)
- [Schema Migration v1](specs/schema-migration.md) — Add/Rename/Drop semantics、Plan、source preservation、commit safetyのcanonical authority
- [Masterdata YAML subset](specs/yaml-subset.md)
- [Table / Primary Key / Secondary Key](specs/table-and-keys.md)
- [Type System](specs/type-system/README.md)
- [Primitive Types](specs/type-system/primitives.md)
- [Field Modifiers](specs/type-system/field-modifiers.md)
- [Runtime hosts / capability](specs/runtime-hosts.md)
- [YAML Source of Truth ADR](adr/0001-yaml-is-source-of-truth.md)
- [shared core ADR](adr/0002-rust-core-shared-by-cli-and-gui.md)
- [host capability ADR](adr/0006-host-capability-composition.md)
- [Specification workflow](contributing/specification-workflow.md)
- [Development workflow](execution-workflow.md)
