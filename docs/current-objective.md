# Current Objective

## Role

この文書はHuman-selectedなcurrent priorityとwork package boundaryの唯一のownerである。
Approved behaviorは[仕様index](specs/README.md)の各canonical specification、現在のStageは
[Development State](execution-state.md)、implementation realityはcode / tests / Gitで確認する。
この文書は未承認のproduct behaviorのimplementation authorityではない。

## Objective

現在のHuman priorityは、**既存Value Object / Enum / Flags Enum / Custom TypeをGUIからraw YAML手編集なしで安全に変更できるType Editor v1を完成させる**ことである。

直前のTable Editor Objectiveはcandidate `d765f40d225130b29a44427dd8cc1a536ce168a6`のfinal verificationでBlockingなしとなり、2026-09-14時点で`objective-complete`へ到達した。その後Humanが次候補比較で推薦されたType Editorについて「進める」と選択したため、本Objectiveをcurrent priorityとする。

各type categoryの静的なschema/data/generated C# semanticsはApproved [Type System](specs/type-system/README.md) familyが所有する。一方、既存type declarationを変更するときのdependency rewrite、source mutation、lost-update、destructive authorization、multi-file rollback、既存data transformationを所有するcanonical mutation contractはまだ存在しない。Approved [Schema Migration v1](specs/schema-migration.md)はTable fieldの`AddField` / `RenameField` / `DropField`に限定され、Type Editorをscope外としている。

したがって、本Objectiveは実装へ直接進まず、まず[Type Editor v1 mutation strategy RFC](rfcs/0006-type-editor-mutation-strategy.md)でmutation boundaryを確定する。RFCが採用された場合は、そのdecisionをType Migration / GUI Type Editorのcanonical specificationへ移し、Human Approval lifecycleを完了してからimplementation-readyへ進む。

## Why now

現在のGUIはWorkspace ExplorerからTable / Data / Value Object / Enum / Flags Enum / Custom Typeを新規作成できる。Data documentはData Editorでrecord authoringでき、Table schemaはTable EditorでSchema Migration v1のAdd/Rename/Drop FieldをPlan / Diff付きで実行できる。一方、作成済みtype documentは専用editorを持たず、definition変更ではYAML手編集へ戻る断点が残っている。

Product Visionはschema-aware editingとshared Rust semanticsを長期方向としている。Type System自体はApproved済みなので、次に必要なのはtype-definition mutationをどのshared semantic boundaryで安全に扱うかを確定し、そのboundaryへGUIを接続することである。

## Completion boundary

本Objectiveのfinal completion boundaryは、RFC 0006のHuman decisionと後続Approved specificationによって確定する。設計段階では少なくとも次を満たす方向でrefineする。

- Workspace Explorerで`kind: type` documentを選択した場合、shared source semanticsからtype categoryをresolveし、Value Object / Enum / Flags Enum / Custom Typeの専用Type Editorを開ける。
- Type Editorはtype identity、source provenance、category-specific declarationをshared application snapshotから表示し、frontendがYAMLを独自parseしてdomain meaningを再構成しない。
- mutation operation set、dependent source rewrite、destructive behavior、existing data treatmentはApproved mutation specificationだけをauthorityとし、GUI convenienceのために暗黙のrewrite / coercion / compatibility policyを発明しない。
- source mutation前のreview surface、lost-update prevention、failure/recovery semanticsは採用されたshared mutation contractに従い、frontend/Tauriへdomain transaction semanticsを複製しない。
- successful type mutationはBuild / Publish / Git / generated artifact更新を暗黙に開始しない。
- Data Editor等の既存dirty buffer、Recovery Required、host capabilityはApproved GUI app shell / editor lifecycleと矛盾なくcompositionする。
- focused core/application regression、Tauri adapter test、React workflow test、repository required checks、final verificationでBlockingがないことを確認する。

## Current design decision

RFC 0006では次の3案を比較している。

1. selected type fileだけをtyped direct editする。
2. dependency rewrite不要な変更だけにType Editor v1を限定する。
3. shared Type Migration v1を導入し、Type EditorをPlan / Diff付きのthin GUI adapterにする。

現時点の推薦は3である。推薦operation setは、Value Object conversion setting、Enum/Flags member Add/Rename/Drop、Custom Type field Add/Rename/Dropに限定し、type declaration rename、underlying変更、member numeric value変更、Custom Type field type/modifier/key/reorder等をv1非対象とする。これは未承認proposalであり、Human decisionとcanonical specification approval前にはimplementation authorityにならない。

## Explicit non-scope

Human decisionとApproved specificationで別途採用されない限り、現Objectiveでは次を暗黙に含めない。

- type declaration name rename。
- Value Object / Enum / Flagsのunderlying変更。
- Enum / Flags member numeric value変更・reorder。
- Custom Type field type / modifier / MessagePack key変更・reorder。
- type category conversion。
- Table Primary / Secondary Key editing。
- Data Editorのcomplex field input拡張。
- source file / folder rename、delete、move、duplicate。
- arbitrary raw YAML editor、formatter、general text editor。
- released-version compatibility system全体。
- Build Profile / Publish、Standalone / Connected Web全体。
- Build / Publish / Gitの自動実行。

## Next candidate

本Objective完了後は、complex fieldを含むrecord入力、Primary / Secondary Key editingを含む次段schema authoring、source rename / delete / move、spreadsheet操作拡張、Programmable View、Build Profile / Publish、Standalone / Connected Webへのauthoring surface展開を候補として比較する。

次priorityは自動昇格せず、current realityとproduct valueを確認してHumanが選択する。

## Relevant authorities

- [Product vision](product/vision.md)
- [Type Editor v1 mutation strategy RFC](rfcs/0006-type-editor-mutation-strategy.md) — current design comparison。implementation authorityではない
- [GUI specification index](gui/README.md)
- [GUI app shell](gui/app-shell.md)
- [Workspace Explorer](gui/explorer/spec.md)
- [Source Creation](gui/source-creation/spec.md)
- [Data Editor](gui/data-editor/spec.md)
- [Table Editor](gui/table-editor/spec.md)
- [Schema Migration v1](specs/schema-migration.md)
- [Masterdata YAML subset](specs/yaml-subset.md)
- [Table / Primary Key / Secondary Key](specs/table-and-keys.md)
- [Type System](specs/type-system/README.md)
- [Primitive Types](specs/type-system/primitives.md)
- [Field Modifiers](specs/type-system/field-modifiers.md)
- [Value Objects](specs/type-system/value-objects.md)
- [Enum / Flags](specs/type-system/enums.md)
- [Custom Types](specs/type-system/custom-types.md)
- [C# naming](specs/type-system/csharp-naming.md)
- [Runtime hosts / capability](specs/runtime-hosts.md)
- [YAML Source of Truth ADR](adr/0001-yaml-is-source-of-truth.md)
- [shared core ADR](adr/0002-rust-core-shared-by-cli-and-gui.md)
- [host capability ADR](adr/0006-host-capability-composition.md)
- [Specification workflow](contributing/specification-workflow.md)
- [Development workflow](execution-workflow.md)
