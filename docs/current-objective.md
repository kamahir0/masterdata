# Current Objective

## Role

この文書はHuman-selectedなcurrent priorityとwork package boundaryの唯一のownerである。
Approved behaviorは[仕様index](specs/README.md)の各canonical specification、現在のStageは
[Development State](execution-state.md)、implementation realityはcode / tests / Gitで確認する。
この文書は未承認のproduct behaviorのimplementation authorityではない。

## Objective

現在のHuman priorityは、**既存Value Object / Enum / Flags Enum / Custom TypeをGUIからraw YAML手編集なしで安全に変更できるType Editor v1を完成させる**ことである。

直前のTable Editor Objectiveはcandidate `d765f40d225130b29a44427dd8cc1a536ce168a6`のfinal verificationでBlockingなしとなり、2026-09-14時点で`objective-complete`へ到達した。その後Humanが次候補比較で推薦されたType Editorについて「進める」と選択したため、本Objectiveをcurrent priorityとした。

2026-09-14、Humanは[Type Editor v1 mutation strategy RFC](rfcs/0006-type-editor-mutation-strategy.md)の比較から、**Shared Type Migration v1 + Plan / Diff**を採用することを明示的に選択した。したがって、selected type fileだけのdirect editやdependency-free subsetではなく、project-wide dependency rewriteとsource mutation safetyをshared semantic boundaryへ置く方向で本Objectiveを完成させる。

各type categoryの静的なschema/data/generated C# semanticsはApproved [Type System](specs/type-system/README.md) familyが所有する。採用decisionをcanonical behaviorへ移すcandidateとして[Type Migration v1](specs/type-migration.md)と[GUI Type Editor](gui/type-editor/spec.md)をProposed化する。これらはHuman Approvalまではimplementation authorityではない。

## Why now

現在のGUIはWorkspace ExplorerからTable / Data / Value Object / Enum / Flags Enum / Custom Typeを新規作成できる。Data documentはData Editorでrecord authoringでき、Table schemaはTable EditorでSchema Migration v1のAdd/Rename/Drop FieldをPlan / Diff付きで実行できる。一方、作成済みtype documentは専用editorを持たず、definition変更ではYAML手編集へ戻る断点が残っている。

Product Visionはschema-aware editingとshared Rust semanticsを長期方向としている。Type System自体はApproved済みなので、次に必要なのはtype-definition mutationをshared Type Migrationとして安全に扱い、そのboundaryへGUIを接続することである。

## Completion boundary

本Objectiveは、Human-approvedなType Migration v1 / Type Editor v1 contractに従い、少なくとも次を満たしたfinal candidateがverificationでBlockingなしとなった時点で完了する。

- Workspace Explorerで`kind: type` documentを選択した場合、shared source semanticsからtype categoryをresolveし、Value Object / Enum / Flags Enum / Custom Typeの専用Type Editorを開ける。
- Type Editorはtype identity、source provenance、category-specific declarationをshared application snapshotから表示し、frontendがYAMLを独自parseしてdomain meaningを再構成しない。
- v1 mutation operation setはValue Object conversion setting、Enum/Flags member Add/Rename/Drop、Custom Type field Add/Rename/Dropに限定する。
- type declaration rename、underlying変更、Enum/Flags member numeric value変更/reorder、Custom Type field type/modifier/key変更/reorder、type category conversionをv1へ含めない。
- mutation前にdeterministic Planとaffected-file Diffを生成し、dependent source rewrite、source preservation、postcondition、stale-plan prevention、destructive authorization、multi-file rollback / Recovery Requiredをshared core/application boundaryが所有する。
- Enum/Flags member renameはnumeric valueを保持したままexisting symbolic occurrenceを更新し、member dropはexisting occurrenceがあればreplacementを推測せずfail closedする。
- Custom Type field addはexisting value occurrenceがある場合にexplicit constant initializerを要求し、rename/dropはnested occurrenceを含むdependent mappingへshared semanticsで適用する。
- Migration Planがaffected sourceとして示すfileにData Editorのdirty bufferがある場合はApplyをblockし、unrelated dirty bufferは保持する。
- successful type mutationはBuild / Publish / Git / generated artifact更新を暗黙に開始しない。
- focused core/application regression、Tauri adapter test、React workflow test、repository required checks、final verificationでBlockingがないことを確認する。

## Current design decision

**Shared Type Migration v1 + Plan / Diffを採用済み。**

RFC 0006で比較した3案のうち、Humanはshared Type Migrationを導入しType Editorをthin GUI adapterにする案を選択した。selected-file direct editとdependency-free subsetは本Objectiveのdesign directionとしては採用しない。

採用decisionを具体化するcanonical candidateは次の2文書である。

- [Type Migration v1](specs/type-migration.md) — operation set、dependency resolution、Plan / Diff、source rewrite、stale preflight、authorization、commit / rollback。
- [GUI Type Editor](gui/type-editor/spec.md) — category-specific editor、guided input、Plan / Diff / Apply、dirty-buffer / recovery composition、thin adapter boundary。

両文書はHuman Approvalを受けるまで`Proposed`であり、implementation authorityではない。reviewでBlockingがなければ、次に必要なHuman actionはこの2 specificationの明示approvalである。

## Explicit non-scope

現Objectiveでは次を含めない。

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
- [Type Editor v1 mutation strategy RFC](rfcs/0006-type-editor-mutation-strategy.md) — Accepted design rationale。implementation authorityではない
- [Type Migration v1](specs/type-migration.md) — Proposed。Human Approval前はimplementation authorityではない
- [GUI Type Editor](gui/type-editor/spec.md) — Proposed。Human Approval前はimplementation authorityではない
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
