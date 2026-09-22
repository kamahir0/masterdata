# 仕様変更0028: Production Delivery & Unity Integration v1

Status: Proposed

## Affected Specifications

- [Build pipeline](../specs/build-pipeline.md): Unity `.meta` lifecycle / publish observationのOpen Questionを解消し、generic publisherとUnity integrationのownership boundaryを追加する。
- [GUI Build / Publish](../gui/build-publish/spec.md): Publish resultとUnity import / compile / runtime verificationの状態を混同しないdelivery UXを追加する。
- [Project layout](../specs/project-layout.md): Unity integration package / project bindingがfilesystem semantic identityを作らないことをroutingする。
- 必要なら新しいUnity Integration canonical ownerを追加し、Unity Editor/runtime側のartifact consumption、diagnostics、load lifecycleを所有させる。

## 根拠と分類（Source Evidence and Classification）

### Human evidence

- 2026-09-22 JST、HumanはAdvanced Authoring完了後に次へ進むことを選択し、コード編集を伴う実装はimplementation agentへ委譲する方針を指定した。

### Existing Approved authority

- Product VisionはMasterMemoryを利用するUnity project向けlocal-first authoring / build systemをproduct identityとしている。
- Build pipelineはcanonical artifactとUnity等external publish destinationを分離する。
- C# publish targetはMasterData-managed generated setとunmanaged destination contentを分離し、unmanaged contentを変更してはならない。
- binary targetはexplicit fileだけをowned targetとして扱い、sibling ownershipを広げない。
- multi-target Publishはtarget-local failure domainであり、global atomic commitを保証しない。
- receiptはcanonical artifact-set integrity metadataであり、Unity AssetDatabase stateやreleased compatibility identityではない。
- Build pipelineはUnity `.meta` lifecycleとpublish manifestの連携をMasterData publisherが持つかUnity importerへ委譲するかをOpen Questionとして残している。
- GUI Build / PublishはPublish successがUnity compile / 動作確認成功を含意しないと既に定義している。

## Problem

C# / binaryをUnity projectの`Assets`配下へ安全にpublishできても、Unityでは各assetに`.meta`とAssetDatabase lifecycleが存在する。

ownerを決めずにMasterData publisherがgenerated `.cs` deletionと同時に`.meta`を削除すると、Unity GUID continuityやuser-owned metadataを壊す可能性がある。一方、publisherが`.meta`を完全に無視するだけでは、Unity integrationとして誰がlifecycle、import observation、diagnosticsを持つかが未定義のまま残る。

このchoiceはfilesystem ownership / stale cleanup / Unity package architectureへ影響するためHuman gateである。

## Option A — MasterData publisher owns Unity .meta

MasterData publisherがUnity destinationを認識し、managed generated C# / binaryと対応する`.meta` lifecycleをpublisher manifest内で所有する。

### Advantages

- artifactとUnity metadataのcleanupを単一publisherで制御できる。
- Unity packageなしでもdestinationを完全管理できる可能性がある。

### Costs / risks

- generic filesystem publisherがUnity-specific semanticsを持つ。
- existing unmanaged preservation boundaryを拡張する必要がある。
- Unity GUID generation / persistence contractをMasterData側で定義・実装する必要がある。
- Unityが作成/更新したmetadataとpublisher ownershipが競合し得る。
- monorepo / separate repo / Unity version差へのcouplingが大きい。

## Option B — 推奨: Unity Editor/package owns .meta

MasterData generic publisherはcurrent contractどおりartifact bytesとMasterData-owned publish manifestだけを管理し、`.meta`を生成・更新・削除しない。

Unity Editor/packageがUnity AssetDatabase lifecycleを所有する。generated filesがAssets treeへ現れた後の`.meta` creation/preservation、asset import observation、Unity-specific diagnosticsはUnity integration側の責務とする。

### Advantages

- generic publisherとUnity-specific lifecycleを分離できる。
- `.meta`をunmanaged Unity stateとしてpreserveでき、GUIDをpublisher都合で再生成しない。
- Unity API / version差をUnity package境界へ閉じ込められる。
- existing C# stale-file cleanup contractを変えずに拡張できる。
- non-Unity publish destinationを汚染しない。

### Costs / constraints

- Unity integration packageが必要。
- publish成功とUnity import成功は別phaseとして表示・diagnoseする必要がある。
- publisherがgenerated `.cs`を削除してもorphan `.meta`が一時的に残る可能性があり、そのcleanup policyはUnity側で明示する必要がある。
- Unityが起動していない場合、Publish operationだけでAssetDatabase import完了を保証できない。

## Option C — No owned Unity integration

現在のgeneric C# / binary publishだけを正式surfaceとし、Unity auto-importと`.meta` behaviorは完全にtool外とする。

### Advantages

- implementation scopeが最小。

### Costs

- Product VisionがUnity向けsystemである一方、delivery/import/runtime integrationが未定義のまま残る。
- Unity固有failureをstructuredに扱えず、production-ready Objectiveを満たしにくい。

## 推奨

Option B。

MasterData publisherはfilesystem artifact publisherとして維持し、Unity-specific stateはUnity packageへ置く。これは既存のshared Rust semantics / adapter separationと同じ責務分離である。

Option B採用後は、同Objective内で次をHumanへ逐次確認せずrefine / implementしてよい。

1. Unity Integration canonical specification。
2. repository内Unity package / Editor integration boundary。
3. generated C# / binaryのsupported Unity destination conventions。
4. `.meta` preserve / orphan cleanup semantics。
5. import / compile / binary-load diagnostics。
6. runtime MemoryDatabase load helper / lifecycle。
7. Desktop delivery status composition。
8. focused Unity/editor-like smoke、existing publisher regression、Candidate / remote CI。

package registry publication、Unity processの自動起動等のexternal irreversible effectはObjective外または別Human gateとする。

## 互換性（Compatibility）

Option Bはexisting generic C# / binary publish semanticsをbreakingに変更しない方向である。`.meta`をpublisher ownershipへ追加しないため、existing destinationのunmanaged content preservationを維持する。

Unity integration packageはadditive surfaceとし、既存projectでpackageを導入しない場合のBuild / Publish behaviorを変更してはならない。

## 受け入れと実装への影響（Acceptance and Implementation Impact）

Human decision後、最低限次をspecify / verifyする。

- publishが`.meta`を作成・更新・削除しないこと。
- Unity側でgenerated C# asset GUIDが通常updateで不必要に変わらないこと。
- stale generated C# deletion後のorphan metadata policy。
- Unity未起動時のPublish successとUnity import unknown stateの分離。
- C# compile error、binary missing/corrupt、MemoryDatabase load failureのdiagnostics。
- same canonical artifact set由来のC# / binaryを可能な範囲で検証し、multi-target partial successをcoherentと誤認しないこと。
- non-Unity C# / binary publish regressionなし。
- Unity packageなしのexisting project behavior不変。

## Human decision

2026-09-22 JST、Human maintainerはOption Bを採用した。

- Unity Editor/packageがUnity `.meta` lifecycleを所有する。
- MasterData generic publisherはgenerated C# / binary artifact bytesとMasterData-owned publish manifestだけを所有する。
- MasterData publisherはUnity `.meta`を生成・更新・削除しない。
- Unity AssetDatabase lifecycle、asset import observation、Unity-specific diagnosticsはUnity integration側が所有する。
- Publish successとUnity import / compile / runtime load successは別phaseとして扱う。

Option A / CはRejected alternativeとする。

package internal layout、Editor window/component naming、runtime helper API、test harness、orphan `.meta` cleanupの安全な具体化は、上記ownership principleと既存Approved authorityから決められる範囲でagent-resolvableとする。new persisted config format、breaking public API/CLI、security/trust boundary、external release/publishが必要になった場合だけHuman gateへ戻す。

## 未解決事項（Open Questions）

Human-gated Open Questionは解消済み。

Implementation前にagentがrefineすべき事項:

- Unity package / repository layout。
- Unity artifact location discoveryを既存publish configurationからどう受け取るか。
- Unity Editor側のorphan `.meta` cleanup policy。
- import / compile / runtime load status model。
- MemoryDatabase load helperとPlay Mode / reload lifecycle。
- Desktop delivery statusのcomposition。

これらはOption Bのownership boundaryを変更しない限りagent-resolvableである。

## レビュー（Review）

Human decisionによりmaterial product forkは解消した。Option Bはexisting generic publisherのunmanaged preservation、target-local ownership、adapter separationと整合する。

次のrefinementではUnity integration canonical owner、package/runtime boundary、failure semantics、verification matrixを確定し、review-specでBlockingなしを確認してからimplementationへ進む。

## 承認記録（Approval Record）

Human decision: Option B, 2026-09-22 JST。
Option A / C: Rejected alternative。
Canonical refinement / implementation: Pending。
