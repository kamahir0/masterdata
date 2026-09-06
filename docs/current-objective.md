# Current Objective

## Role

この文書は、現在のdevelopment priorityを記録する唯一のownerである。

この文書はSpecificationではなく、product/domainのobservable semanticsのauthorityでもない。
Approved semanticsはcanonicalな[仕様](specs/README.md)を参照し、implementation realityはcurrent code、tests、
Git historyをfreshに確認する。ここに書かれたpriorityだけを根拠に、未承認のbehavior、CLI grammar、config key、
protocol、file formatを実装してはならない。

## Objective

現在のHuman priorityは、**build / publish CLI lifecycleを先に完成させる**ことである。

このrepositoryで先に閉じる主要なintegration boundaryは、Approvedな[`CLI-007`](specs/cli.md#cli-007)が定義する
`build --publish` compositionである。buildが成功した場合だけpublishへ進み、standalone `publish`と同じ
receipt-valid canonical artifact setおよびpublish semanticsを利用する方向を、主要CLI lifecycleのcompletionとして扱う。

## Why now

canonical artifact-set receipt、external publisher、standalone CLI publishまでの責務と境界を先に整理してきた。
その上で`build --publish`を完成させると、canonical buildからexternal publishまでの主要CLI workflowが一貫する。

Schema MigrationはApproved semanticsをかなり持つが、現在のHuman priorityではbuild / publish lifecycleのcompletionより
後へ回す。

## Completion boundary

次の既存contractを満たすintegrationとしてcompletionを判定する。

- build成功後だけpublishへ進む。
- build failureではpublishを開始しない。
- build成功時に確定したcanonical artifact setは、後続publish failureだけを理由にrollbackしない。
- publish failure時は、canonical buildが成功していてもcombined CLI operation全体をsuccessとして報告しない。
- 詳細なartifact、receipt、target-local failure、aggregate resultの意味は、[Build pipeline仕様](specs/build-pipeline.md)と
  [CLI surface仕様](specs/cli.md)のownerへ委譲する。

## Explicit non-scope

このobjectiveは、次を今回のpriorityに含めない。

- Schema Migrationのruntime implementation
- GUI/Tauri、Web、Native Hostのfeature implementation
- Approved semantic contract、Requirement ID、Diagnostic Codeの変更
- `build --publish`の未承認なresult serialization、new public grammar、config key、protocol、file formatの発明
- Current Objectiveをfeature status一覧、進捗率、test inventory、implementation inventoryとして運用すること

## Next candidate

次のHuman priority候補は**Schema Migration implementation**である。これは実装順を自動的に承認するものではなく、
priority上のcandidateである。着手時は[Schema Migration v1仕様](specs/schema-migration.md)と関連するApplied
spec-change、ADR、RFCを読み、必要なworkflowとapproval gateを満たす。

## Relevant authorities

- [Product vision](product/vision.md) — product problemとlong-term direction
- [Specification index](specs/README.md) — specification lifecycleとnormative authority
- [Build pipeline specification](specs/build-pipeline.md) — canonical artifact / receipt / publish contract
- [CLI surface specification](specs/cli.md) — `build --publish` composition
- [Specification change 0007](spec-changes/0007-canonical-artifact-set-receipt.md) — artifact-set receiptの適用履歴
- [Specification change 0008](spec-changes/0008-multi-target-publish-execution.md) — multi-target publish executionの適用履歴
- [Schema Migration v1 specification](specs/schema-migration.md) — Next candidateのsemantic authority
- [Specification workflow](contributing/specification-workflow.md) — decisionのdurable routingとapproval lifecycle
