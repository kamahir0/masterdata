# Current Objective

## Role

この文書は、現在のdevelopment priorityを記録する唯一のownerである。

この文書はSpecificationではなく、product/domainのobservable semanticsのauthorityでもない。
Approved semanticsはcanonicalな[仕様](specs/README.md)を参照し、implementation realityはcurrent code、tests、
Git historyをfreshに確認する。ここに書かれたpriorityだけを根拠に、未承認のbehavior、CLI grammar、config key、
protocol、file formatを実装してはならない。

## Objective

現在のHuman priorityは、**GUI canonical build vertical sliceを完成させる**ことである。

current GUIはproject overviewとshared validationをuser-facing actionとして利用でき、Tauri adapterには既存の
`NativeApplicationService::build`を呼び出すbuild commandも存在する。一方、frontendのBuild controlはまだ無効であり、
canonical buildのresultをGUIから実行・確認できない。

次に閉じるintegration boundaryは、既存のNative Application Services / canonical build semanticsをそのまま利用し、
GUI上でcurrent projectのfull canonical buildを明示的に実行して、そのsuccess / failureと既存resultを確認できる状態にすることである。

このobjectiveは新しいbuild semantics、artifact format、publish semanticsを定義しない。GUI presentationを実装するためにApproved authorityから
observable behaviorを安全に決められない場合は、implementation convenienceで補完せずSpecification GapとしてHuman decisionへ戻す。

## Why now

canonical build、artifact-set receipt、standalone publish、`build --publish`までの主要Native / CLI lifecycleはcurrent implementationで一通り
接続され、GUI validation vertical sliceもshared application semanticsを再利用する形で完了した。

次は新しいfoundationを広げる前に、すでに実装済みのcanonical build capabilityをGUI workflowへ接続することで、user-visible valueを短い距離で
増やせる。current Tauri adapterにはshared build commandが存在し、frontendにもBuild controlがあるため、validationに続く小さなvertical sliceとして
architecture boundaryを再利用できる。

[Build pipeline仕様](specs/build-pipeline.md)はcanonical buildとexternal publishを分離し、[Runtime hosts仕様](specs/runtime-hosts.md)はTauri Desktopが
Native Application Servicesを共有してadapterへbuild semanticsを複製しないことを要求している。このため、GUI Buildは新しいdomain behaviorを発明せず、
既存contractをproduct surfaceへ露出するintegration workとして進められる。

Schema MigrationはApproved semanticsを持つ重要な次候補であり、authoring systemへ進むための主要foundationである。ただし今回はsource-preserving
transformation / transaction engineを先に構築するより、既存build capabilityをGUIから利用可能にしてNative product workflowをもう一段閉じることを優先する。

## Completion boundary

次の既存contractを満たすintegrationとしてcompletionを判定する。

- GUIからcurrent projectに対するfull canonical buildを明示的に実行できる。
- Tauri adapterはCLI subprocessや独自builderを使わず、既存の`NativeApplicationService::build` / shared build semanticsを利用する。
- build中、build success、build failureをGUIから区別して確認できる。
- success時に、少なくともcanonical artifact root、canonical C# output、canonical binary outputをGUIから確認できる。
- failure時は既存のstructured diagnosticをGUIから確認できる。
- GUIの通常buildはexternal publish targetを暗黙に更新しない。
- current project loading、diagnostic structure、build semantics、artifact ownershipは既存ownerを再利用し、GUI側で第二のdomain / build implementationを作らない。
- Approved authorityから決められないGUI observable behaviorが必要になった場合は、Specification Gapとして停止する。

## Explicit non-scope

このobjectiveは、次を今回のpriorityに含めない。

- standalone publishまたはGUI publish wiring
- `build --publish`相当のGUI composition
- Build Profile選択UX
- table / record editor、cell editing、source write、save UX
- Schema Migration runtime implementation
- Standalone Web、Connected Web、Native Hostのfeature implementation
- Generated C# Preview / explicit C# exportの新しいpublic UX
- new build / publish semantics、new Diagnostic Code、Requirement ID変更
- 未承認なGUI result schema、protocol、config key、file format、CLI grammarの発明
- production-gradeなtable editorやdesign-system全面刷新
- Current Objectiveをfeature status一覧、進捗率、test inventory、implementation inventoryとして運用すること

## Next candidate

次のHuman priority候補は**Schema Migration implementation**である。これは実装順を自動的に承認するものではなく、
priority上のcandidateである。着手時は[Schema Migration v1仕様](specs/schema-migration.md)と関連するApplied
spec-change、ADR、RFCを読み、必要なworkflowとapproval gateを満たす。

GUI canonical build完了後は、authoring systemへ進むためにSchema Migrationを優先候補とする。ただしproduct experience上、
別のuser-visible vertical sliceを先に進める価値が高いとHumanが判断した場合は、このcandidateを自動昇格させずCurrent Objectiveを改めて選定する。

## Relevant authorities

- [Product vision](product/vision.md) — local-first productとGUI / CLI shared semanticsの方向性
- [Specification index](specs/README.md) — specification lifecycleとnormative authority
- [Build pipeline specification](specs/build-pipeline.md) — canonical build、artifact set、build / publish separation
- [Runtime hosts specification](specs/runtime-hosts.md) — Tauri / Native Application Services / shared semanticsのboundary
- [CLI surface specification](specs/cli.md) — build OperationとCLI surfaceの分離、build / publish compositionの既存contract
- current `NativeApplicationService::build` implementation / tests — implementation realityとshared native build entrypoint
- current `apps/gui` frontend / Tauri adapter — GUI implementation realityと既存build command / disabled Build control
- [Schema Migration v1 specification](specs/schema-migration.md) — Next candidateのsemantic authority
- [Specification workflow](contributing/specification-workflow.md) — decisionのdurable routingとapproval lifecycle
