# Unity Integration仕様

Status: Implemented

Domain: Production Delivery & Unity Integration

## 位置付け

この仕様は、canonical MasterData Buildと既存safe Publishのartifact bytesをUnity projectへdeliveryし、Unity Editorのimport/compile observationとUnity applicationのMasterMemory database loadを、MasterData publisherと別phaseで扱うcontractを所有する。canonical YAML、schema/type/reference semantics、artifact receipt、generic publish ownershipは各既存ownerが所有し、このdocumentはそれらをUnity integrationへ接続する境界だけを定義する。

Human decisionは仕様変更0028でOption Bを採用した。Unity Editor/packageがUnity `.meta` lifecycleを所有し、MasterData generic publisherはgenerated C#、explicit binary、MasterData-owned publish manifestだけを所有する。

## Canonical phases

```text
canonical Build
  -> receipt-valid artifact set
  -> existing safe Publish
  -> Unity asset import observation
  -> Unity compile observation
  -> explicit runtime binary load
  -> caller-owned MemoryDatabase
```

Publish successはUnity import、compile、runtime loadのsuccessを意味しない。Unity Editorが未起動または観測していない場合、Unity phaseは`not_observed`/`unknown`であり、generic Publish reportへ混ぜてはならない。

## Normative requirements

### UNITY-DELIVERY-001

MasterData Build、receipt validation、generic Publish、Unity import、Unity compile、およびruntime binary loadは別phaseとして表現しなければならない（MUST）。Publish successだけをUnity-ready、compile success、runtime load successとして表示してはならない（MUST NOT）。

### UNITY-DELIVERY-002

generic publisherのownershipは既存[Build pipeline仕様](build-pipeline.md)のC# managed set、publish manifest、およびexplicit binary fileに限る（MUST）。publisherはUnity `.meta`を生成、更新、削除、manifest登録してはならない（MUST NOT）。C# managed artifactまたはexplicit binaryのupdate/retirement時、隣接`.meta`やunmanaged siblingをbasename一致だけで操作してはならない（MUST NOT）。

### UNITY-DELIVERY-003

Unity package / Editor integrationはAssetDatabase、asset import observation、Unity compiler observation、asset identity/GUID lifecycleを所有しなければならない（MUST）。MasterData publisherはGUIDを生成、再生成、保持するidentity registryを持ってはならない（MUST NOT）。safeなasset identityを確定できないorphan cleanupは削除せずpreserve + structured diagnosticとする（MUST）。

### UNITY-DELIVERY-004

repository packageはUPM-compatibleなversion-controlled `unity/Packages/com.kamahir0.masterdata` layoutを持ち、runtime assemblyとEditor assemblyを分離しなければならない（MUST）。Editor assemblyだけがUnityEditor APIを参照し、runtime assemblyはUnityEditor APIを参照してはならない（MUST NOT）。package registry releaseはv1の前提ではない。

### UNITY-DELIVERY-005

runtime APIはcallerからexplicit binary bytesまたはStreamingAssets relative pathと、generated databaseを生成するfactoryを受け取らなければならない（MUST）。packageはMasterMemory/MessagePack binaryをparse、再実装、独自index化してはならない（MUST NOT）。factoryはconsumerがpinned MasterMemory 3.0.4 / MessagePack 3.1.3とgenerated `MemoryDatabase(byte[])`を接続するために使用する。missing、empty、read、cancellation、factory failureはstructured errorとして返し、empty databaseやhidden singletonへ置換してはならない（MUST NOT）。

### UNITY-DELIVERY-006

runtime databaseはcallerが所有し、reloadはcallerが新しいload/factory callを明示して行う（MUST）。packageはglobal/static database singleton、generated rowへのhidden database injection、implicit Build/Publish、automatic reloadを提供してはならない（MUST NOT）。runtime assemblyはUnityEditorへ依存してはならない（MUST NOT）。

### UNITY-DELIVERY-007

Editor observationはcallerが指定したexact generated C# directoryとbinary fileだけを対象としなければならない（MUST）。packageはMasterData YAML/configの再解釈、filesystem全体探索、filename/basename heuristic、`.meta`のwrite/delete、automatic orphan cleanupを行ってはならない（MUST NOT）。artifact missing、import pending/unknown、compile unknown/failureはstable `MASTERDATA-UNITY-*` diagnosticとして区別する。

### UNITY-DELIVERY-008

Editor statusは少なくともC# artifact presence、binary presence、import state、compile state、diagnosticsを表示しなければならない（MUST）。compiler message本文をMasterData diagnosticへ再分類する必要はないが、Unity compiler failureをPublish successへ変換してはならない（MUST NOT）。runtime load statusはEditor compile statusとは別であり、Editor observerがruntime database successを主張してはならない（MUST NOT）。

### UNITY-DELIVERY-009

Desktop Build / Publishはshared application reportのtarget statusとaggregate outcomeを表示し、Unity verificationを`not_observed`として明示しなければならない（MUST）。C# targetまたはbinary targetのpartial failureをUnity-readyとして表示してはならない（MUST）。DesktopはUnity process起動、reverse IPC、pathからUnity projectを推測してはならない（MUST NOT）。

### UNITY-DELIVERY-010

Unity packageはcanonical schema/YAML parser、Type System、Reference resolver、MasterMemory binary implementationを含んではならず（MUST NOT）、view/generated C# property/binary fieldを追加してはならない（MUST NOT）。既存publish target configurationは、例えば`kind = "csharp"`を`Assets/MasterData/Generated`へ、`kind = "binary"`を`Assets/StreamingAssets/masterdata.bytes`へ指定する既存compositionを利用する。新しいMasterData persisted configを追加しない。

### UNITY-DELIVERY-011

package validationはUnity executable/licenseのないrequired CIでも、package manifest、asmdef separation、runtime/editor API boundary、diagnostic identifiers、およびdeterministic source layoutを検証できなければならない（MUST）。実Unity Editor compileが実行できない環境では、未実施を実機成功として報告してはならない（MUST NOT）。

### UNITY-DELIVERY-012

runtime StreamingAssets APIはcallerが`Application.streamingAssetsPath`からresolve可能なrelative pathを指定する形としなければならない（MUST）。v1の推奨compositionは`Assets/StreamingAssets/masterdata.bytes`とrelative name `masterdata.bytes`であるが、packageはpublish target pathをsemantic identityとして管理したり、arbitrary asset discoveryを行ったりしてはならない（MUST NOT）。platform-specific readはUnity supported `UnityWebRequest` boundaryへ委譲する。

## Failure and ownership model

MasterData publish reportはtargetごとの`succeeded`/`failed`/`not_attempted`とaggregate outcomeを返す。Unity Editor statusは`unknown`、`pending`、`observed`、`failed`を独立して返す。runtime loaderはmissing/read/empty/factory/cancellation failureを例外またはstructured resultとして返し、error swallowingをしない。`.meta`のGUIDはUnityが管理し、通常のpublisher updateでは既存bytesがpreserveされる。stale generated C#のretirement後に孤立`.meta`が残る場合、v1 packageは削除せずdiagnostic/warningを表示する。

## Compatibility

既存MasterData projects、generated C#、MasterMemory binary、artifact receipt、generic publish target semanticsへの変更はない。Unity packageのadditive導入なしにexisting behaviorは変わらない。package registry、cross-schema binary interoperability、external save/network contract、artifact signingはこのspecificationの保証外である。

## Verification

publisher testsはC# update、stale generated C# retirement、binary sibling、unmanaged file、`.meta` preservationを検証する。package static checkerはUPM/asmdef boundaryを検証する。runtime/editor sourceは可能な環境でUnity Editor compileを行い、runtime load pathはcaller factoryへ委譲するpure APIとfailure evidenceを検証する。Desktop testはPublish aggregateとUnity `not_observed` compositionを検証する。CIはUnity executableをrequired dependencyにしない。

## Open Questions

なし。runtime loaderのC# identifier、Editor windowのlayout、static checkerの内部module構成はimplementation detailである。新しいpersisted configuration、mandatory platform support、package registry release、Unity process automationが必要になった場合は別changeとする。
