# 仕様変更0028: Production Delivery & Unity Integration v1

Status: Applied

## Affected Specifications

- [Unity Integration仕様](../specs/unity-integration.md): Unity package、Editor observation、runtime load、phase separation、`.meta` ownership、Desktop compositionのcanonical ownerを追加する。
- [Build pipeline仕様](../specs/build-pipeline.md): generic publisherがUnity `.meta`を所有しないことをroutingする。
- [Project layout仕様](../specs/project-layout.md): existing `publish.targets` compositionからUnity destinationを構成できることをroutingする。
- [GUI Build / Publish仕様](../gui/build-publish/spec.md): generic Publish resultとUnity verification未観測を分離表示する。
- [仕様index](../specs/README.md): Unity Integration ownerを追加する。

## Source Evidence

- Human maintainerは2026-09-22 JSTにOption B（Unity Editor/packageがUnity `.meta` lifecycleを所有）を選択した。
- 既存の`masterdata-app` publisherはC# managed-set manifestと明示binary fileだけを所有し、unmanaged file、`.meta`、binary siblingを保護する。既存のpublish testsはそのcontractを証明している。
- canonical build receiptはC#とbinaryのcoherencyをBuild時点で検証するが、Unity import、compile、runtime loadを観測するものではない。
- GUI Build / PublishはBuild、Publish、Unity compile/runtimeを別phaseとして扱う既存contractを持つ。
- MasterMemory 3.0.4 / MessagePack 3.1.3のgenerated C# compile/runtime evidenceは.NET bridgeとcodegen testsにあり、Unity packageがこのimplementationを複製する必要はない。
- current repositoryのCIはUnity Editorを提供しないため、repository checkはpackage boundary/static validationを行い、実Unity compileは利用可能な開発環境で追加evidenceとする。

## Adopted Agent Decisions

1. Unity Integration canonical ownerを`docs/specs/unity-integration.md`とし、packageを`unity/Packages/com.kamahir0.masterdata`へ置く。package registryへのpublishは行わない。
2. 新しいMasterData persisted config、Unity settings asset、GUID registryは追加しない。C#とbinaryのdestinationは既存`[[publish.targets]]`をsource of truthとし、Unity runtime loaderはcallerが明示するStreamingAssets relative pathを受け取る。
3. packageはMasterMemory / MessagePackをbundleまたはupgradeしない。runtime APIは`byte[]`とcaller-supplied factoryを受け取り、consumerが既存generated `MemoryDatabase(byte[])`とpinned dependencyを提供する。
4. Editor surfaceは明示pathに対するread-only AssetDatabase observationとcompile status表示に限定する。packageはYAML parser、schema resolver、publisher、`.meta` write/delete、orphan heuristic cleanupを持たない。
5. runtime surfaceはexplicit binary ownership、async StreamingAssets read、factory delegation、caller-owned databaseを提供する。hidden singleton、binary parser、reload global stateは導入しない。
6. Desktopはgeneric publish aggregateと`unity_verification = not_observed`を表示する。Unity Editorへのreverse connection、process自動起動、Publishからのimplicit Unity verificationは行わない。
7. package manifest/asmdef/source boundaryのstatic validationをrepository `check-all`へ追加し、Unity Editorをrequired CI dependencyにしない。ローカルでは利用可能なUnity Editorのcompile evidenceを取得する。

## Requirements

- `UNITY-DELIVERY-001` — canonical Build、receipt-valid artifact set、existing safe Publish、Unity import、Unity compile、runtime loadを別phaseとして扱い、Publish successだけからUnity readinessを推測しない。
- `UNITY-DELIVERY-002` — generic publisherはC# managed set、publish manifest、explicit binary fileだけを所有し、`.meta`を生成・更新・削除・manifest登録してはならない。stale generated artifact retirementでも隣接`.meta`を削除してはならない。
- `UNITY-DELIVERY-003` — Unity Editor/packageがAssetDatabase import observation、Unity compiler observation、asset identity/GUID lifecycle、safe orphan handlingを所有する。安全にidentityを確定できないcleanupはpreserve + diagnosticとする。
- `UNITY-DELIVERY-004` — Unity packageはversion-controlled UPM-compatible layout、runtime/editor assembly分離、stable package diagnosticsを提供し、package registry publishを要求しない。
- `UNITY-DELIVERY-005` — runtime packageはMasterMemory binaryをparseまたは再実装せず、explicit bytes/factory APIとStreamingAssets async APIを通じてconsumerのexisting generated `MemoryDatabase` constructionへ委譲する。読み込み失敗、empty/missing binary、factory failure、cancellationはerrorとして返し、黙ってempty databaseへ置換してはならない。
- `UNITY-DELIVERY-006` — runtime packageはUnityEditor APIを参照せず、hidden global database singleton、hidden database injection、implicit Build/Publish、automatic reloadを提供してはならない。callerがdatabase lifecycleとreloadを所有する。
- `UNITY-DELIVERY-007` — Editor observationはcallerが渡したexact C# directoryとbinary fileを対象とし、filesystem全体探索、basename heuristic、MasterData config再解釈を行ってはならない。artifact missing、import pending/unknown、compile unknown/failureをstructured Unity diagnosticsとして区別する。
- `UNITY-DELIVERY-008` — Unity Editorが起動していないPublishはPublish successとUnity import/compile/runtime `not_observed`を別々に表現する。C# targetまたはbinary targetのpartial failureはUnity-readyとして表示してはならない。
- `UNITY-DELIVERY-009` — Desktop Build / Publishはshared application reportを表示し、Unity statusをReact独自のpath heuristicやUnity process起動で生成してはならない。Build、preview、Confirm、Publishの既存explicit workflowとno implicit side effectsを維持する。
- `UNITY-DELIVERY-010` — package sourceはcanonical YAML、schema/type/reference semantics、MasterMemory binary internalsを含まず、generated C# surfaceを変更せず、existing non-Unity publish behaviorを保持する。
- `UNITY-DELIVERY-011` — package/runtime/editor status ordering、diagnostic ordering、static validationはdeterministicで、repository CIでUnity executable/licenseがなくても検証可能でなければならない。実Unity compile evidenceが得られない環境では未実施として報告し、実機検証済みと主張してはならない。
- `UNITY-DELIVERY-012` — Unity runtime path conventionとしてcallerが`Application.streamingAssetsPath`からresolveできるrelative pathを指定し、v1では既存exampleの`Assets/StreamingAssets/masterdata.bytes` compositionを推奨する。packageはpublish target pathをsemantic identityとして所有せず、arbitrary discoveryを行わない。

## Compatibility / Non-scope

このchangeはadditiveであり、既存`masterdata.toml`、publish target kind、artifact receipt、C# generator、MasterMemory binary、generated Reference helperのpublic shapeを変更しない。既存projectはUnity packageを導入しない限り従来のBuild/Publish behaviorを保持する。対象外はpackage registry publish、Unity process自動起動、remote control/IPC、GUID/stable identity registry、MasterMemory binary parser、cross-schema binary guarantee、save/network compatibility、artifact signing、Web productである。

Unity `.meta`はpublisherのmanaged setではなくUnity asset lifecycleである。通常updateでexisting `.meta` bytesをpreserveし、stale generated C# retirementで`.meta`が残ることを許容する。v1 packageはorphan cleanupを自動実行せず、AssetDatabaseが安全なidentityを観測できる将来adapterの余地だけを残す。

## Acceptance evidence

canonical ownerのVerification節に従い、publisher stale retirement / C# update / binary sibling `.meta` preservation、non-Unity regression、Desktop aggregate/status、runtime API failure/success、asmdef boundary/static package validation、利用可能な環境でのUnity Editor compileを確認する。Unity runtime codeはcaller-provided factoryを通じてMasterMemory 3.0.4 / MessagePack 3.1.3へ委譲し、package自身のdependency upgradeを行わない。

## Open Questions

なし。package内部のC# identifier、IMGUI layout、static checkerのmodule名はimplementation detailでありHuman gateではない。新しいpersisted MasterData config、public CLI、package registry release、mandatory platform supportが必要になった場合は別のspecification change / Human gateへ戻す。

## Approval / Application Record

Approval mode: Agent-autonomous. `refine-spec` と`review-spec`のfresh reviewでBlockingおよびHuman gateがないこと、既存config/publisher/runtime contractを壊さないadditive changeであることを確認し、2026-09-22 JSTにUnity Integration canonical ownerへ適用した。Human-selected Current Objective: Production Delivery & Unity Integration、Human decision: Option B。
