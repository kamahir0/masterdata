# Current Objective

## Role

この文書はHuman-selectedなcurrent priorityとDONE boundaryのownerである。observable semanticsはcanonical specification、resume地点は[Development State](execution-state.md)、implementation realityはcode / tests / Gitで確認する。

## Objective

**Standalone Webで、明示的に許可されたlocal workspaceを開き、共有frontendとRust semanticsを使って既存sourceの閲覧・編集・検証・保存を完結させ、静的配布可能な成果物をverification済みcandidateへ到達させる。**

## Completion slices

### Browser workspaceとshared semantics

- [Runtime hosts](specs/runtime-hosts.md) `RUNTIME-HOST-001`, `RUNTIME-HOST-003`, `RUNTIME-HOST-005`, `RUNTIME-HOST-007`, `RUNTIME-HOST-011..013`をStandalone Webで実現する。
- Browser workspaceのpermission、source read/write、logical path、保存競合、失敗時のobservable behaviorをspecification workflowで確定する。
- DesktopとWebのfrontend / application semanticsを共有し、host固有のI/Oをadapterへ分離する。

### Authoring workflow

- 許可された既存workspaceでExplorerからsourceを選び、Data / Table / Typeを編集・検証・保存できる。
- capabilityのないnative Build / Publishは事前に利用不可とわかる。

### Verification

- 共有semanticsのcross-host regression、Browser実操作、Desktop回帰、WASM、repository checksを確認する。
- static bundleを作り、exact Candidateのfresh verificationとrequired remote CI reconciliationを完了する。

## Explicit non-scope

- Native Host、Connected Web、loopback transport、browser内の.NET / MasterMemory binary build。
- Webからのnative Build / Publish、実サイト公開・release。
- Reference完成、expression / computed / programmable view。
- Git操作のproduct機能化。Project / sourceの新規作成は今回のcompletion requirementとしない。
- Approved specificationにないobservable behaviorをimplementation convenienceで追加すること。

## Audit

このObjectiveは2026-09-21 JSTにHumanが選択した。前ObjectiveのP4 completionはGit historyで追跡する。
