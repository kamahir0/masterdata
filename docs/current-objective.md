# Current Objective

## Role

この文書はHuman-selectedなcurrent priorityとwork package boundaryの唯一のownerである。
Approved behaviorはcanonical specification、現在のStageは[Development State](execution-state.md)、implementation realityはcode / tests / Gitで確認する。

## Objective

現在のHuman priorityは、**Approved P4 packageを実装し、existing record key fieldのdirect editとsame-root source file rename / moveをDesktop authoring workflowへ安全に統合するcandidateを作る**ことである。

2026-09-20 JST、Human maintainerは仕様変更0019 / 0020を一括Approvalした。deltaはcanonical specificationへ適用済みで、Stageは`implementation-ready`である。implementation authorityは各Approved canonical ownerであり、Applied artifactはaudit recordである。

## Completion boundary

P4 candidateは、少なくとも次を満たす。

### P4-A — Existing record key field edit

- existing recordのPrimary Key / Secondary Key構成fieldをdirect single-cell editingで変更できる。
- key membershipはeditable化後も識別可能。
- existing keyへのpaste / fill / range Set Null / single-cell paste等のAuthoring Batch mutationは引き続き拒否する。
- key edit専用modal confirmationを要求しない。
- source provenanceでtarget occurrenceを保持し、変更前後key valueだけで再特定しない。
- duplicate Primary Key / unique Secondary Key等のdomain diagnosticだけを理由にSaveを拒否しない。
- source-preserving Save、Conflict / Overwrite、Failure / Outcome Unknown、Undo/Redoの既存lifecycleを維持する。

### P4-B — Source file rename / move

- existing Masterdata source fileをsame configured source root内でrename / moveできる。
- destinationはvalidな`.yaml` / `.yml` source path、existing parent folder内に限定する。
- configured roots間move、destination Overwrite、source delete / duplicate、folder rename / moveは拒否または提供しない。
- target dirty fileはSave / Don't Save / Cancelでpath mutation前に解決し、unrelated dirty buffersを保持する。
- destination/source raceはConflictとしてmutation前に停止する。
- case-only renameをsupported hostで扱う。
- known Failureではcomplete old source + unchanged distinct destinationへ収束し、それを確認できなければOutcome Unknownとする。
- Success後はExplorer selection / open editor / path-bearing stateをnew pathへ追従させる。
- Recovery Required中はrename / moveを開始しない。

### Verification

- focused core/application/GUI regression evidenceを追加する。
- path mutationはhost/filesystem差を含むため、Tier 1 behaviorを検証できるtest/evidenceを残す。
- repository required checksを通し、self-review後にcandidate SHAをDevelopment Stateへ記録してverificationへ進める。

## Canonical implementation inputs

- [Source Record Edit](specs/source-edit.md) — `SOURCE-EDIT-017`
- [Authoring Batch](specs/authoring-batch.md) — existing key batch edit禁止を維持
- [Data Editor](gui/data-editor/spec.md) — `GUI-DATA-STATE-001`, `GUI-DATA-EDIT-001`
- [Source Path Mutation](specs/source-path-mutation.md) — `SOURCE-PATH-001..007`
- [Workspace Explorer](gui/explorer/spec.md) — `GUI-EXPLORER-STATE-004`, `GUI-EXPLORER-INT-004..006`, `GUI-EXPLORER-ERR-002`
- [GUI app shell](gui/app-shell.md) — Recovery Required mutation gate
- [Runtime hosts](specs/runtime-hosts.md) — shared application / host boundary
- [Project layout](specs/project-layout.md) — source pathはdomain identityではない

## Explicit non-scope

- existing keyへのbatch mutation。
- MessagePack field `key`、Primary / Secondary Key declarationのschema mutation。
- configured source roots間move。
- destination overwrite。
- source file delete / duplicate、folder rename / move。
- P5: expression / computed / programmable view。
- P6: Reference完成、Standalone / Connected Web authoring完成。
- Git stage / commit / pushのproduct機能化。
- Approved specificationにないobservable behaviorをimplementation convenienceで追加すること。

## Relevant audit records

- [0019 Applied record](spec-changes/0019-existing-record-key-edit.md)
- [0020 Applied record](spec-changes/0020-source-file-rename-move.md)
- [Development workflow](execution-workflow.md)
