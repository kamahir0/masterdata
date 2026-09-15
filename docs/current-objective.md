# Current Objective

## Role

この文書はHuman-selectedなcurrent priorityとwork package boundaryの唯一のownerである。
Approved behaviorは[仕様index](specs/README.md)の各canonical specification、現在のStageは
[Development State](execution-state.md)、implementation realityはcode / tests / Gitで確認する。
この文書は未承認のproduct behaviorのimplementation authorityではない。

## Objective

現在のHuman priorityは、**Data Editorでcomplex fieldをraw YAML手編集なしで安全に入力・編集できるComplex Value Authoring v1を完成させる**ことである。

直前のType Editor v1 Objectiveはcandidate `fc03700ad188773cee4c0b291f5661368bd44979`のfinal verificationでBlockingなしとなり、2026-09-15時点で`objective-complete`へ到達した。その後Humanがcomplex field record inputを次priorityとして選択した。

## Why now

Table EditorとType Editorによりschema / type authoringはGUIから行えるようになったが、定義したcomplex typeをrecord値として入力する場面ではData Editorの初期scopeに戻りraw YAML手編集が必要になる断点が残っている。

Approved Type SystemはValue Object、Enum、Flags Enum、Custom Type、Required / Nullable / Arrayのsemanticsを既に所有する。次に必要なのはdomain semanticsをfrontendへ複製せず、shared Rust authoring boundaryからData Editorへ接続することである。

## Completion boundary

本ObjectiveはHuman-approvedなcomplex value authoring contractに従い、少なくとも次を満たしたfinal candidateがverificationでBlockingなしとなった時点で完了する。

- Data EditorがApproved Type Systemからresolvedされたcomplex field shapeをshared application snapshotとして受け取り、frontendがYAML parseやtype resolutionを独自実装しない。
- v1でsupportするfield category / modifierとexisting record edit・Added record draftの適用範囲がcanonical specificationで明示される。
- supported complex valueの入力はlosslessで、`long` / `ulong`を含むnested valueでもfrontendのlossy numeric representationへ強制変換しない。
- source mutationはYAML Source of Truth、source provenance、source-preserving candidate、file単位dirty / Save、lost-update prevention、Conflict / Failure / Outcome Unknownの既存安全境界と整合する。
- validation resultはediting / Save可否のdomain gateにせず、current bufferに対するshared validation diagnosticとして表示する。
- existing recordのPrimary / Secondary Key mutation、schema/type mutation、Build / Publish / Gitの暗黙実行を本Objectiveへ混入しない。
- focused core/application regression、Tauri adapter test、React workflow test、repository required checks、final verificationでBlockingがないことを確認する。

## Current approved direction

**Shared schema-driven value authoring modelをexisting editとAdd Rowで共用するOption C、P1 Fine-grained preservation、D1 YAML `null` placeholderはHuman Approval済みである。**

Human maintainerは2026-09-16、[Complex Value Authoring v1 strategy RFC](rfcs/0007-complex-value-authoring-strategy.md)のOption Cを選択し、その後P1 / D1を選択した。さらに独立reviewでBlocking / Non-blocking / QuestionsがNone、`Approved as Proposed: Yes`となった[spec-change 0015](spec-changes/0015-complex-value-authoring.md)をproposal全体として明示Approveした。

承認済みdeltaは同じcanonical mergeで次のApproved specificationへ適用する。

- [Source Record Edit](specs/source-edit.md): shared resolved value authoring boundary、lossless nested value、fine-grained source preservation / fail-closed。
- [Source Record Mutation](specs/source-record-mutation.md): complex Add Record capability、Added record draftのD1 `null` placeholder、existing/new recordのshared value semantics。
- [Data Editor](gui/data-editor/spec.md): existing non-key complex field edit、schema-aware complex editor、nested diagnostic / focus。
- [Data Editor Record Mutation](gui/data-editor/record-mutation.md): complex TableのAdd Row、typed draft input、existing/new record共通editor semantics。

spec-change 0015は`Applied`のaudit recordとなり、implementation authorityは上記canonical specificationが所有する。現在のApproved authorityから実装に必要なobservable semanticsをrecoverでき、追加のHuman decision / Approval / Specification Gapは確認されていない。

## Explicit non-scope

- existing recordのPrimary / Secondary Key構成field mutation。
- Table schema field add / rename / drop、Primary / Secondary Key editing。
- type declaration / member / Custom Type field mutation。
- source file / folder rename、delete、move、duplicate。
- `$tags` authoring、record reorder / duplicate、bulk add / bulk delete。
- spreadsheet range selection、一括paste、fill handle、general Undo/Redo。
- arbitrary raw YAML editor、formatter、general text editor。
- released-version compatibility system全体。
- Build Profile / Publish、Standalone / Connected Web全体。
- Build / Publish / Gitの自動実行。

## Next candidate

本Objective完了後は、Primary / Secondary Key editingを含む次段schema authoring、source rename / delete / move、spreadsheet操作拡張、Programmable View、Build Profile / Publish、Standalone / Connected Webへのauthoring surface展開を候補として比較する。

次priorityは自動昇格せず、current realityとproduct valueを確認してHumanが選択する。

## Relevant authorities

- [Product vision](product/vision.md)
- [Complex Value Authoring v1 strategy RFC](rfcs/0007-complex-value-authoring-strategy.md) — Accepted design rationale。implementation authorityではない
- [Complex Value Authoring specification change](spec-changes/0015-complex-value-authoring.md) — Applied audit record。current semanticsのownerではない
- [Data Editor](gui/data-editor/spec.md) — Approved implementation authority
- [Data Editor Record Mutation](gui/data-editor/record-mutation.md) — Approved implementation authority
- [Source Record Edit](specs/source-edit.md) — Approved implementation authority
- [Source Record Mutation](specs/source-record-mutation.md) — Approved implementation authority
- [Type System](specs/type-system/README.md)
- [Primitive Types](specs/type-system/primitives.md)
- [Field Modifiers](specs/type-system/field-modifiers.md)
- [Value Objects](specs/type-system/value-objects.md)
- [Enum / Flags](specs/type-system/enums.md)
- [Custom Types](specs/type-system/custom-types.md)
- [Table / Primary Key / Secondary Key](specs/table-and-keys.md)
- [Masterdata YAML subset](specs/yaml-subset.md)
- [Runtime hosts / capability](specs/runtime-hosts.md)
- [YAML Source of Truth ADR](adr/0001-yaml-is-source-of-truth.md)
- [shared core ADR](adr/0002-rust-core-shared-by-cli-and-gui.md)
- [host capability ADR](adr/0006-host-capability-composition.md)
- [Specification workflow](contributing/specification-workflow.md)
- [Development workflow](execution-workflow.md)