# Current Objective

## Role

この文書は、現在のdevelopment priorityを記録する唯一のownerである。

この文書はSpecificationではなく、product/domainのobservable semanticsのauthorityでもない。
Approved semanticsはcanonicalな[仕様](specs/README.md)を参照し、implementation realityはcurrent code、tests、
Git historyをfreshに確認する。ここに書かれたpriorityだけを根拠に、未承認のbehavior、CLI grammar、config key、
protocol、file formatを実装してはならない。

## Objective

現在のHuman priorityは、**GUI validation vertical sliceを完成させる**ことである。

current GUIはprojectをTauri経由で開いてoverviewを表示できる一方、validationはuser-facing actionとしてまだ利用できない。
次に閉じるintegration boundaryは、Tauri adapterから既存のNative Application Services / shared validation semanticsを利用し、
GUI上でcurrent projectのvalidationを実行して、その既存resultとstructured diagnosticsを確認できる状態にすることである。

このobjectiveは新しいvalidation semanticsを定義しない。GUI presentationを実装するためにApproved authorityからobservable behaviorを
安全に決められない場合は、implementation convenienceで補完せずSpecification GapとしてHuman decisionへ戻す。

## Why now

canonical build、artifact-set receipt、standalone publish、`build --publish`までの主要CLI lifecycleはcurrent implementationで一通り
接続された。次はfoundationをさらに広げるより、既存のshared semanticsを実際のGUI workflowへ接続し、user-visible valueまでの距離を
縮めることを優先する。

[Product vision](product/vision.md)はCLIとGUIが同じvalidation resultを利用できる方向を示しており、
[Runtime hosts仕様](specs/runtime-hosts.md)はTauri Desktopがvalidationを含むdomain/application semanticsを共有し、adapterへsemantic logicを
複製しないことを要求している。current GUI shellとTauri adapterがすでに存在するため、validationは小さなvertical sliceとしてこのboundaryを
実証するのに適している。

Schema MigrationはApproved semanticsを持つ重要な次候補だが、今回はsource-preserving transaction engineを先に積むより、
GUIで既存価値を利用可能にすることを優先する。

## Completion boundary

次の既存contractを満たすintegrationとしてcompletionを判定する。

- GUIからcurrent projectに対するvalidationを明示的に実行できる。
- Tauri adapterはCLI subprocessや独自validatorを使わず、既存のNative Application Service / shared validation semanticsを利用する。
- validation success / failureと、既存のstructured diagnosticsをGUIから確認できる。
- validationを理由にcanonical artifact set、external publish target、publish manifestなどを変更しない。
- current project loading、diagnostic structure、validation semanticsは既存ownerを再利用し、GUI側で第二のdomain implementationを作らない。
- Approved authorityから決められないGUI observable behaviorが必要になった場合は、Specification Gapとして停止する。

## Explicit non-scope

このobjectiveは、次を今回のpriorityに含めない。

- table / record editor、cell editing、source write、save UX
- Schema Migration runtime implementation
- GUIのbuild / publish wiring
- Standalone Web、Connected Web、Native Hostのfeature implementation
- new validation semantics、new Diagnostic Code、Requirement ID変更
- 未承認なGUI result schema、protocol、config key、file format、CLI grammarの発明
- production-gradeなtable editorやdesign-system全面刷新
- Current Objectiveをfeature status一覧、進捗率、test inventory、implementation inventoryとして運用すること

## Next candidate

次のHuman priority候補は**Schema Migration implementation**である。これは実装順を自動的に承認するものではなく、
priority上のcandidateである。着手時は[Schema Migration v1仕様](specs/schema-migration.md)と関連するApplied
spec-change、ADR、RFCを読み、必要なworkflowとapproval gateを満たす。

GUI validation完了後に、product experience上の次vertical sliceを先に進める価値が高いとHumanが判断した場合は、
このcandidateを自動昇格させずCurrent Objectiveを改めて選定する。

## Relevant authorities

- [Product vision](product/vision.md) — product problem、GUI/CLI shared validationの方向性
- [Specification index](specs/README.md) — specification lifecycleとnormative authority
- [Runtime hosts specification](specs/runtime-hosts.md) — Tauri / shared domain/application semanticsのboundary
- [CLI surface specification](specs/cli.md) — source-derived validationのno-mutation boundaryと既存CLI mapping
- current `NativeApplicationService::validate` implementation / tests — implementation realityとshared validation entrypoint
- current `apps/gui` frontend / Tauri adapter — GUI implementation reality
- [Schema Migration v1 specification](specs/schema-migration.md) — Next candidateのsemantic authority
- [Specification workflow](contributing/specification-workflow.md) — decisionのdurable routingとapproval lifecycle
