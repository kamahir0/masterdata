# Current Objective

## Objective

**Clean-room Rewrite**。`rewrite/clean-room`上で、精製済みProduct Constitution / canonical contracts / independent oracleからMasterDataをfreshに再実装し、Humanがmainへのcutoverを判断できる状態へ到達する。

## Completion boundary

- contractからのarchitecture導出、buildable workspaceとCI、instrumentされたDesktop vertical slice。
- [ordinary authoring baseline](gui/rewrite-baseline.md)、[canonical domain](specs/README.md)、[Desktop contracts](gui/README.md)の実装。
- [Desktop presentation quality](gui/app-shell.md#presentation-quality)とReact / Ant Design baseline、theme / motion / actual visual review。
- source / schema / type mutation、write safety、Migration、CLI、Build / Publish、native MasterMemory delegationとactual consumer。
- [independent oracle](../fixtures/rewrite-oracle/v1/README.md)の全category、[acceptance matrix](rewrite-preparation/acceptance-matrix.md)、source exact bytes、100k capacity。
- [performance contract](gui/performance.md)のdistribution / work counts、macOS arm64 / Windows x64 actual Desktop、fresh Candidate reviewとrequired CI。
- 未解決Blockingなしの**Cutover-ready**。内部milestoneはこのObjectiveのcompletionではない。

## Authority / non-scope

[formal inputs](rewrite-preparation/README.md)、[constitution](rewrite-preparation/constitution.md)、[domain / safety](rewrite-preparation/domain-safety.md)、[handoff](rewrite-preparation/clean-room-handoff.md)。

旧production code / topologyをdefault input、template、copy sourceにしない。mainへのmerge、release、scope外featureの追加は行わない。
