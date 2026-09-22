# Current Objective

## Role

この文書はHuman-selectedなcurrent priorityとDONE boundaryのownerである。observable semanticsはcanonical specification、resume地点は[Development State](execution-state.md)、implementation realityはcode / tests / Gitで確認する。

## Objective

**Production Delivery & Unity Integrationをproduction-readyにし、canonical MasterData Build/PublishからUnity packageのEditor import observation、compile status、runtime MasterMemory loadまで、責務分離されたdelivery workflowを一貫して利用できる状態へ到達する。**

## Completion slices

- [Unity Integration仕様](specs/unity-integration.md)に従うUPM-compatible repository package、runtime/editor assembly separation、explicit path/status API。
- generic publisherのC# manifest / explicit binary ownership、`.meta`/GUID lifecycleのUnity側委譲、stale retirement preservation。
- Publish aggregate、Unity import/compile/runtime loadのphase separation、Desktopでの`not_observed`表示。
- caller-supplied factoryによるMasterMemory 3.0.4 / MessagePack 3.1.3 runtime load、missing/corrupt/factory failure semantics。
- focused publisher/package/Desktop regressions、package static validation、available environmentのUnity compile evidence、fresh review、exact Candidate、required remote CI reconciliation。

## Explicit non-scope

- package registry publish、Unity process自動起動、reverse control/IPC、generated artifact auto-commit。
- MasterMemory binary parser再実装、cross-schema binary compatibility、save/network compatibility、artifact signing。
- Unity GUID/stable identity registry、publisherによる`.meta` cleanup、YAML/schema resolverのpackage複製。
- Web product revival、unrelated Desktop polish、および既存Advanced Authoring/Reference semanticsの再設計。

## Audit

2026-09-22 JST、Advanced Authoring完了後のHuman priorityとしてProduction Delivery & Unity Integrationを開始した。仕様変更0028でHumanはOption B（Unity Editor/packageが`.meta` lifecycleを所有）を選択した。PUBLISH ownership、receipt、generated C#、binary contractを再利用し、Unity import/compile/runtimeを別phaseに保つ。
