# Current Objective

## Role

この文書はHuman-selectedなcurrent priorityとDONE boundaryのownerである。observable semanticsはcanonical specification、resume地点は[Development State](execution-state.md)、implementation realityはcode / tests / Gitで確認する。

## Objective

**Approved P4 packageを実装し、existing record key fieldのdirect editとsame-root source file rename / moveをDesktop authoring workflowへ統合し、verification済みcandidateへ到達する。**

## Completion slices

### P4-A — Existing record key field edit

- [Source Record Edit](specs/source-edit.md) `SOURCE-EDIT-017`
- [Data Editor](gui/data-editor/spec.md) `GUI-DATA-STATE-001`, `GUI-DATA-EDIT-001`
- [Authoring Batch](specs/authoring-batch.md) `AUTHORING-BATCH-001` のexisting-key batch mutation prohibitionを維持

### P4-B — Source file rename / move

- [Source Path Mutation](specs/source-path-mutation.md) `SOURCE-PATH-001..007`
- [Workspace Explorer](gui/explorer/spec.md) `GUI-EXPLORER-STATE-004`, `GUI-EXPLORER-INT-004..006`, `GUI-EXPLORER-ERR-002`
- [GUI app shell](gui/app-shell.md) `GUI-SHELL-CAPABILITY-001`
- [Runtime hosts](specs/runtime-hosts.md) のshared application / host boundary
- [Project layout](specs/project-layout.md) のsource path / identity boundary

### Verification

- changed behaviorにfocused core/application/GUI regression evidenceを置く。
- host/filesystem差を含むpath mutationのTier 1 evidenceを持つ。
- required repository checksとself-reviewを完了し、exact Candidateをfresh verificationしてBlockingなしにする。

## Explicit non-scope

- existing keyへのbatch mutation。
- MessagePack field `key`、Primary / Secondary Key declarationのschema mutation。
- configured source roots間move、destination overwrite。
- source file delete / duplicate、folder rename / move。
- P5 expression / computed / programmable view。
- P6 Reference完成、Standalone / Connected Web authoring完成。
- Git stage / commit / pushのproduct機能化。
- Approved specificationにないobservable behaviorをimplementation convenienceで追加すること。

## Audit

0019 / 0020は2026-09-20 JSTにHuman ApprovalされcanonicalへApplied済み。implementation authorityは上記canonical ownersであり、[0019](spec-changes/0019-existing-record-key-edit.md) / [0020](spec-changes/0020-source-file-rename-move.md)はhistorical audit recordである。
