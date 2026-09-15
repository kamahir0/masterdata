# Current Objective

## Role

この文書はHuman-selectedなcurrent priorityとwork package boundaryの唯一のownerである。
Approved behaviorは[仕様index](specs/README.md)の各canonical specification、現在のStageは
[Development State](execution-state.md)、implementation realityはcode / tests / Gitで確認する。
この文書は未承認のproduct behaviorのimplementation authorityではない。

## Objective

現在のHuman priorityは、**Data Editorでcomplex fieldをraw YAML手編集なしで安全に入力・編集できるComplex Value Authoring v1を完成させる**ことである。

直前のType Editor v1 Objectiveはcandidate `fc03700ad188773cee4c0b291f5661368bd44979`のfinal verificationでBlockingなしとなり、2026-09-15時点で`objective-complete`へ到達した。その後Humanが次候補比較で推薦されたcomplex field record inputについて「進める」と選択したため、本Objectiveをcurrent priorityとした。

現在のApproved Data Editorは、existing recordではRequired Primitiveの非key fieldだけを通常editableとし、Nullable / Array / Enum / Flags / Value Object / Custom Type等をread-onlyとする。またAdd Rowは全fieldがRequired PrimitiveのTableだけをsupportedとする。各complex valueのdomain semantics自体はApproved Type System familyで定義済みであり、次に必要なのはそれらをData Editorのshared authoring boundaryへ安全に接続することである。

## Why now

Table EditorとType Editorによりschema / type authoringはGUIから行えるようになったが、定義したcomplex typeをrecord値として入力する場面ではData Editorの初期scopeに戻り、raw YAML手編集が必要になる断点が残っている。

Product VisionはYAMLをSource of Truthとして保ちながらschema-aware editingとshared Rust semanticsをDesktop / Web / CLIで共有する方向を示している。Complex Value Authoringは、既にApprovedなType Systemを実際のrecord authoring UXへ接続し、authoring surfaceの縦切りを閉じる次のwork packageである。

## Completion boundary

本Objectiveは、Human-approvedなcomplex value authoring contractに従い、少なくとも次を満たしたfinal candidateがverificationでBlockingなしとなった時点で完了する。

- Data EditorがApproved Type Systemからresolvedされたcomplex field shapeをshared application snapshotとして受け取り、frontendがYAML parseやtype resolutionを独自実装しない。
- v1でsupportするfield category / modifierと、existing record edit・Added record draftのどちらへ適用するかがcanonical specificationで明示される。
- supported complex valueの入力はlosslessで、`long` / `ulong`を含むnested valueでもfrontendのlossy numeric representationへ強制変換しない。
- source mutationはYAML Source of Truth、source provenance、source-preserving candidate、file単位dirty / Save、lost-update prevention、Conflict / Failure / Outcome Unknownの既存安全境界と整合する。
- validation resultはediting / Save可否のdomain gateにせず、現在bufferに対するshared validation diagnosticとして表示する既存Data Editor方針を維持する。
- existing recordのPrimary / Secondary Key mutation、schema/type mutation、Build / Publish / Gitの暗黙実行を本Objectiveへ混入しない。
- focused core/application regression、Tauri adapter test、React workflow test、repository required checks、final verificationでBlockingがないことを確認する。

## Current design decision

**Complex field record inputを次priorityとして選択済み。authoring strategyは未選択。**

[Complex Value Authoring v1 strategy RFC](rfcs/0007-complex-value-authoring-strategy.md)で、初期sliceの境界とshared authoring modelを比較している。現在は`decision-required`であり、RFCのdesign directionをHumanが選択するまでcanonical specification変更や本格実装へ進まない。

## Explicit non-scope

現Objectiveでは次を含めない。

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
- [Complex Value Authoring v1 strategy RFC](rfcs/0007-complex-value-authoring-strategy.md) — current design comparison。implementation authorityではない
- [Data Editor](gui/data-editor/spec.md)
- [Data Editor Record Mutation](gui/data-editor/record-mutation.md)
- [Source Record Edit](specs/source-edit.md)
- [Source Record Mutation](specs/source-record-mutation.md)
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
