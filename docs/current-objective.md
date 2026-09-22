# Current Objective

## Role

この文書はHuman-selectedなcurrent priorityとDONE boundaryのownerである。observable semanticsはcanonical specification、resume地点は[Development State](execution-state.md)、implementation realityはcode / tests / Gitで確認する。

## Objective

**Production Delivery & Unity Integrationをproduction-readyにし、MasterDataでBuildしたcoherent C# / MasterMemory binaryをUnity projectへ安全に届け、Unity Editor / runtimeが明示的なownershipとdiagnosticsの下でimport・検証・利用できる状態へ到達する。**

## Completion slices

### Unity ownership and package boundary

- Unity固有のasset lifecycle、特にgenerated C# / binaryに付随する`.meta` ownershipを明示し、generic MasterData publisherのmanaged/unmanaged contractと矛盾しない。
- Unity Editor/runtime integrationを既存Build / Publish semanticsのconsumerとして設計し、raw YAML parsing、MasterMemory binary format再実装、独自schema semanticsをUnity側へ複製しない。
- Unity package / integration surfaceのinstall/update boundary、supported artifact locations、failure/diagnostic behaviorを定義する。

### Delivery workflow

- canonical artifact setからUnity projectへのC# / binary deliveryをexisting receipt / publish path safety / target-local rollback contract上で構成する。
- Unity側はpublish後のasset import / compile / binary loadを観測・検証し、MasterData publish successとUnity compile/runtime successを混同しない。
- partial publish、stale artifact、missing binary、compile failure、asset import failureをstructuredかつrecoverableに扱う。
- Desktop Build / Publish UXからUnity delivery状態を理解できる範囲をspecify / implementし、hidden auto-build / auto-publishを導入しない。

### Unity runtime usability

- Unity application codeがMasterMemory binaryを明示的にloadし、generated types / MemoryDatabaseを既存MasterMemory contractで利用できるintegration pathを提供する。
- binary source/path、initialization lifecycle、reload/Editor Play Mode境界、missing/corrupt artifactのfailure semanticsを定義する。
- generated codeとbinaryが同じMasterData build resultに由来することを可能な範囲で検証し、保証できない状態をcoherentと推測しない。

### Verification

- Unity-facing package/integration、publisher boundary、Editor/runtime smokeをfocused evidence化する。
- existing non-Unity publish target、canonical artifact、Build/Publish semanticsをregressさせない。
- fresh review、repository checks、exact Candidate、required remote CI reconciliationまで完了する。

## Adopted Unity ownership boundary

2026-09-22 JSTのHuman decisionにより、Unity Editor/packageが`.meta` lifecycleを所有するOption Bを採用した。

MasterData generic publisherはgenerated C# / binary artifact bytesと自身のpublish manifestだけを管理し、Unity `.meta`を生成・更新・削除しない。Unity AssetDatabase lifecycle、asset import observation、Unity-specific diagnosticsはUnity integration側が所有する。Publish successとUnity import / compile / runtime load successを混同しない。

## Explicit non-scope

- MasterData publisherによるUnity AssetDatabase APIの直接操作。
- Unity processの自動起動・remote controlを通常Publish success条件にすること。
- package registryへのrelease / publish。
- generated C#またはbinaryをGitへ自動commitすること。
- MasterMemory binary formatのUnity側再実装。
- cross-schema binary compatibility guarantee。
- external save/network compatibility。
- artifact signing / producer authentication。
- Web / Browser surfaceの再導入。

## Audit

2026-09-22 JST、Advanced Authoring / Computed View完了後、Humanは次Objectiveへ進むことを選択した。コード編集を伴う実装はimplementation agentへ指示書で委譲する。Product VisionがUnity project向けlocal-first systemを明示し、Build / PublishがUnity等のexternal destinationを既に扱うため、次の大きなproduct outcomeとしてProduction Delivery & Unity Integrationを開始した。同日、仕様変更0028のHuman gateではOption B（Unity Editor/packageが`.meta` lifecycleを所有）を採用した。
