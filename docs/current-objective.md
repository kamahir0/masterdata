# Current Objective

## Role

この文書はHuman-selectedなcurrent priorityとwork package boundaryの唯一のownerである。
Approved behaviorはcanonical specification、現在のStageは[Development State](execution-state.md)、implementation realityはcode / tests / Gitで確認する。

## Objective

現在のHuman priorityは、**P4としてexisting recordのkey field編集とsource file rename / moveを安全なauthoring operationとして仕様化し、Human Approval可能なimplementation packageへ収束させる**ことである。

2026-09-19、HumanはDesktop制作v1（P1–P3）のobjective-complete後の次priorityとしてP4を選択した。P4はApproved behaviorへまだ存在しないため、まずspecification refinement / reviewを行い、observable semanticsとsafety boundaryを確定する。Human Approval前にproduct implementationへ進めない。

## Why now

Desktop制作v1によりProject作成、typed authoring、query / batch、Settings、Build / Publishまでの日常制作workflowは成立した。一方、existing recordのPrimary / Secondary Key構成fieldはread-onlyであり、source fileのrename / moveもSource Creation / Explorer / Source Editの非目標として残っている。

P4は、この2つの明示的な制作上の制約を、既存のYAML Source of Truth、source-preserving edit、lost-update protection、Recovery Required、shared Rust application/core boundaryを維持したまま解消するwork packageである。

## Human-selected scope

2026-09-19、HumanはP4の詳細scopeとして次を選択した。

### P4-A — Existing record key field edit

- existing recordのPrimary Key / Secondary Key構成fieldをdirect single-cell editで編集可能にする。
- paste / fill / range Set Null等のAuthoring Batchはinitial P4-A scopeへ含めず、existing keyをbatch edit対象外とする現在のcontractを維持する。
- key edit専用の追加modal confirmationを必須にせず、通常のtyped cell edit / validation / Save lifecycleへ統合する。
- MessagePack field `key`やschemaのPrimary / Secondary Key定義変更とは区別する。
- record occurrenceのtargetingはkey valueではなくexisting source provenanceを引き続き使用する。
- source-preserving edit、lossless typed value、file単位Save、Conflict / Failure / Outcome Unknown、validationとSaveの分離を維持する。

### P4-B — Source file rename / move

- rename / moveは同じconfigured source root内に限定する。configured source roots間moveはinitial scope外。
- target source fileにdirty bufferがある場合、move前にSave / Don't Save / Cancelでそのfileのdirty stateを解決する。unrelated dirty bufferは保持する。
- destinationが既に別entryとして存在する場合はConflictとし、initial scopeではOverwriteを提供しない。
- case-only renameをsupported Desktop環境で扱えるcontractとする。
- source pathをTable / Type等のdomain identityへ昇格させない。
- filesystem mutationはshared application / host capability boundaryへ置き、frontendへpath safetyやsource semanticsを複製しない。

## Completion boundary

このdesign/specification phaseの完了候補は、少なくとも次を満たす。

- P4-A / P4-Bを別のdurable specification change artifactとして整理する。
- Human-selected scopeを各changeへ反映し、review可能な`Proposed`へ収束させる。
- current Approved specs、ADR、Product terminology、current implementationと照合してAffected Specifications、compatibility impact、acceptance evidenceを明確にする。
- 独立`review-spec`でBlockingを解消する。
- Human Approval後にのみcanonical specificationへ適用し、implementation-readyへ進める。

## Explicit non-scope

- P5: expression / computed / programmable view。
- P6: Referenceの完成、Standalone / Connected Web authoring完成。
- MessagePack field `key`のschema編集、Primary / Secondary Key定義そのもののschema mutation。
- existing keyへのpaste / fill / range Set Null等のbatch mutation。
- configured source roots間のsource move。
- source destination Overwrite。
- source file delete / duplicate、folder rename / move、arbitrary filesystem operation。
- Git stage / commit / pushのproduct機能化。
- Approved specificationにないobservable behaviorをimplementation convenienceで追加すること。

## Relevant authorities

- [Authoring system v1 RFC](rfcs/0008-authoring-system-v1.md)
- [Source Record Edit](specs/source-edit.md)
- [Authoring Batch](specs/authoring-batch.md)
- [Table / Primary Key / Secondary Key](specs/table-and-keys.md)
- [Data Editor](gui/data-editor/spec.md)
- [Data Editor Grid Authoring](gui/data-editor/grid-authoring.md)
- [Project layout](specs/project-layout.md)
- [Source Artifact Creation](specs/source-creation.md)
- [Workspace Explorer](gui/explorer/spec.md)
- [GUI app shell](gui/app-shell.md)
- [Runtime hosts](specs/runtime-hosts.md)
- [P4-A Proposed change](spec-changes/0019-existing-record-key-edit.md)
- [P4-B Proposed change](spec-changes/0020-source-file-rename-move.md)
- [Development workflow](execution-workflow.md)
