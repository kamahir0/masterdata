# Build pipeline仕様（Build pipeline）

Status: Approved

## この文書の位置付け

この文書は、MasterData projectのcanonical build artifactを作成する処理と、作成済みartifactを外部へ配置する処理の
境界を定義するApproved canonical specificationである。`BUILD-ARTIFACT-001`から`BUILD-ARTIFACT-005`および
`PUBLISH-001`から`PUBLISH-010`は、Human maintainerの承認を経てcurrent normative contractとなった。この承認とcanonical
documentへの適用は[仕様変更0004](../spec-changes/0004-canonical-artifacts-publish-targets.md)に記録する。
external publish destinationのfilesystem path safetyに関する`PUBLISH-PATH-001`から
`PUBLISH-PATH-010`は、仕様変更0006のHuman Approvalとcanonical applicationによってcurrent
normative contractとなった。
publish-only operationが使用するcanonical artifact-set receiptに関する`ARTIFACT-SET-001`から
`ARTIFACT-SET-008`は、[仕様変更0007](../spec-changes/0007-canonical-artifact-set-receipt.md)のHuman Approvalとcanonical applicationによって
current normative contractとなった。
複数publish targetのall-target preflight、target-local failure、continuation、およびaggregate resultに関する
`PUBLISH-EXEC-001`から`PUBLISH-EXEC-005`は、[仕様変更0008](../spec-changes/0008-multi-target-publish-execution.md)のHuman Approvalと
canonical applicationによってcurrent normative contractとなった。
legacy configurationのhard cutとstructured migration diagnosticの適用は、[仕様変更0005](../spec-changes/0005-legacy-build-path-hard-cut.md)に記録する。

Approvedなdomain semanticsは、[Build Selection仕様](build-selection.md)、[Table / Primary Key / Secondary Key仕様](table-and-keys.md)、
各Type System仕様、[YAML subset仕様](yaml-subset.md)、および[Project layout仕様](project-layout.md)がそれぞれ所有する。
この文書は、これらのdomain semanticsを変更せず、build artifactとpublishのarchitecture boundaryを定義する。`Approved`は
implementation evidenceが揃ったことを意味しない。canonical configuration parser、CLI、canonical artifact builderは実装済みであるが、external
filesystem publisherとartifact-set receipt runtimeは未実装であるため、`PUBLISH-004`以降、`PUBLISH-PATH-*`、および`ARTIFACT-SET-*`のpublish operationに対応するimplementation、tests、fixtures、およびmigrationは別taskで行う。

今回のrefinementでは、project-localなcanonical build artifactsと、Unityなどの外部publish destinationsを別の層として扱う。
このdocumentのApproved contractに対するimplementationは、canonical configuration、CLI、core build plan、canonical artifact builderへ段階的に接続されている。
external publish operationは未実装であり、影響するcanonical specificationのStatus変更とconfiguration contractのreconciliationは、仕様変更0004、0005、および0007に記録する。
external publish path safety、receipt、partial executionのlifecycle recordは、それぞれ仕様変更0006、0007、および0008に記録する。

## 承認されたcanonical model

以下をHuman Approval済みの中心modelとして採用する。

- MasterData projectはproject directory内にcanonical build artifact領域を持つ。
- Unity projectやserver projectなど、project root外を含む外部配置はpublish targetとする。
- monorepoとseparate repositoryのどちらもdeployment topologyとして扱える。compiler semanticsはGit repository topologyに依存しない。
- 1つのcanonical artifactを0個以上のpublish targetへ配布できる。配布先ごとにbuildを繰り返さない。
- canonical artifact領域はMasterData toolが所有するbuild outputであり、user-owned treeへの直接出力とは区別する。
- canonical C#、canonical binary、およびartifact-set receiptは、最後に成功したfull buildに由来するcoherent setとして扱う。
- project directoryのbasenameはconvention上kebab-caseを基本とするが、project identityは`project.id`であり、basenameから導出しない。

上記のapproval記録は仕様変更0004および0007を参照する。implementation evidenceが揃うまで、この文書を`Implemented`へ変更しない。

## Canonical pipeline

canonical buildとpublishを次の二段階に分ける。

```text
resolve project
 -> load config
 -> discover YAML
 -> parse schema/data/type
 -> profile-independent validation
 -> Build Selection
 -> selected logical Table construction
 -> Primary Key / Unique / Reference validation
 -> canonical record ordering
 -> C# generation plan
 -> canonical artifact staging
 -> compile schema-specific .NET builder
 -> build MasterMemory binary
 -> reload/validate binary
 -> hash staged canonical C# and binary
 -> stage artifact-set receipt
 -> publish complete canonical artifact set
 -> optional publish targets
```

`build`はcanonical artifactを作成するoperationであり、外部publishを暗黙に含めない。`publish`は検証済みcanonical artifact setを
外部destinationへ配布する別operationとする。このbuild/publish separationはこの文書のApproved contractである。

Build Selectionとselected datasetに対するconstraint validationの順序は、[Build Selection仕様](build-selection.md)の
`BUILD-SELECT-010`、`BUILD-SELECT-011`、および`BUILD-SELECT-017`に従う。pipelineはselection前のprofile-independent validationと、
selection後のdataset-level validationを混同してはならない。

Rust coreはproject/config解決、YAMLのtyped AST、semantic validation、Type System/Table resolution、BuildPlan、およびcanonical artifact生成に
必要なvalidated modelを担当する。`masterdata-codegen-csharp`はresolved modelからstructured C#をloweringし、MasterMemory binary formatと
Source Generatorのbehaviorは.NET dependencyに残す。`.NET` process invocationは`masterdata-dotnet`に集約し、application serviceはstagingと
artifact publicationを担当する。CLIとTauriはshared workflowを呼び出し、domain semanticsまたは.NET invocationを複製しない。

generated C#のtype、property、constructor parameterのidentifier contractは[C#命名仕様](type-system/csharp-naming.md)が所有する。build pipelineは
source nameのnormalization、transliteration、suffix付与、または自動repairによってこのcontractを置き換えてはならない。

## Normative requirements: canonical build artifacts

### BUILD-ARTIFACT-001

v1のcanonical artifact rootはMasterData project directoryの配下でなければならない（MUST）。default locationは`.masterdata/output/`とする。
canonical rootを設定で上書きする場合も、`artifact_dir`はproject rootを基準とするproject-local relative pathでなければならず（MUST）、
absolute pathまたは`..`によるproject directory外へのescapeをcanonical build destinationとして許可してはならない（MUST NOT）。

canonical artifact rootの場所はGit repositoryのroot、checkout directoryのbasename、Unity projectの場所、またはsource YAMLのdirectoryから
導出してはならない（MUST NOT）。

### BUILD-ARTIFACT-002

canonical artifact rootはMasterData tool-ownedでなければならず（MUST）、canonical buildはそのrootをuser-owned output treeとの共存場所として
扱ってはならない（MUST NOT）。canonical root内に置くものは、canonical C#、canonical binary、および別途承認されたartifact-set receiptなどのtool metadataに限る。

current `build.output`配下のmanaged/unmanaged file coexistenceは旧implementationのbehaviorであり、canonical root ownershipの代替ではない。
canonical configuration implementationは旧configurationを受理せず、既存legacy artifactを自動削除または自動移行しない。canonical outputはcomplete rootとして
stagingし、successful build後にcoherentな`csharp/`、`masterdata.bytes`、およびreceiptのsetを公開する。receiptを含むcomplete setの外側でmetadataだけを更新してはならない（MUST NOT）。

### BUILD-ARTIFACT-003

v1のcanonical layoutは次のとおりでなければならない（MUST）。

```text
<project root>/
├─ masterdata.toml
├─ schemas/
├─ data/
└─ .masterdata/
   ├─ output/
   │  ├─ csharp/
   │  │  ├─ Item.g.cs
   │  │  └─ Enemy.g.cs
   │  ├─ masterdata.bytes
   │  └─ .masterdata-artifact-set.json
   └─ cache/
```

v1ではbinaryを`output/masterdata.bytes`へ置き、receiptを`output/.masterdata-artifact-set.json`へ置く。`output/binary/`や`output/metadata/`への分割はこのspecificationで先取りしない。

### BUILD-ARTIFACT-004

canonical C#、canonical binary、およびartifact-set receiptは、同一のvalidated build resultから作成されたcoherent artifact setでなければならない（MUST）。
buildが成功した場合はcompleteなcurrent setを公開し、前回buildで不要になったcanonical generated fileを残してはならない（MUST）。
build失敗時は、最後のcoherent canonical setを利用不能にするpartial setを公開してはならない（MUST NOT）。

canonical artifactの作成は、raw YAMLをauthorityとするvalidation、Type System/Table resolution、Build Selection、必要なconstraint validation、
canonical ordering、およびC#/.NET validationを経た後に行う。canonical artifact自体をsource of truth、semantic schema hash、cache key、または
released compatibility identityとして扱ってはならない（MUST NOT）。

### BUILD-ARTIFACT-005

canonical buildはcanonical artifactだけを作成し、configured external publish targetを暗黙に更新してはならない（MUST NOT）。
`publish`はcanonical artifact setを入力とする別operationでなければならない（MUST）。

`build --publish`のような統合UXは将来候補であり、このrequirementはそのcommandの存在またはexit semanticsを確定しない。

## Normative requirements: canonical artifact-set receipt

### ARTIFACT-SET-001

successfulなnon-dry-run full buildは、canonical C#、canonical binary、およびartifact-set receiptを、1つのcoherent canonical artifact setとして
公開しなければならない（MUST）。receiptだけを先に、またはC#とbinaryのpublication後に別操作として更新してはならない（MUST NOT）。通常のI/O
failureでは、成功時はcompleteなNEW C#、NEW binary、matching NEW receiptを、失敗時はprevious complete C#、binary、matching receiptを維持する。
filesystem crashまたはpower-loss時のglobal atomicityは、このrequirementでは保証しない。

### ARTIFACT-SET-002

receiptのv1 reserved filenameは`.masterdata-artifact-set.json`とする。receiptは少なくとも次のsemantic fieldsを持たなければならない（MUST）。

```json
{
  "version": 1,
  "project_id": "game.masterdata",
  "hash_algorithm": "sha256",
  "csharp": [
    { "path": "Enemy.g.cs", "hash": "..." },
    { "path": "Item.g.cs", "hash": "..." }
  ],
  "binary": {
    "path": "masterdata.bytes",
    "hash": "..."
  }
}
```

v1の`version`は`1`、`hash_algorithm`は`sha256`でなければならない（MUST）。hashは対象artifact fileのexact byte sequenceに対するSHA-256であり、
lowercase 64桁hexをcanonical representationとする。JSON whitespaceやproperty serialization orderはsemantic contractとしない。receiptはtimestamp、
random UUID、absolute checkout path、Git commit、machine hostname、またはcurrent cwdを必須identity fieldとして持ってはならない（MUST NOT）。C# pathをdeterministic orderで
記録するため、同じ`project.id`と同じcanonical artifact bytesから生成されるreceiptのsemantic contentはdeterministicでなければならない（MUST）。
`version`はreceipt JSON formatのversionであり、MasterMemory binary format、builder protocol、project semantic version、またはreleased compatibility versionを表してはならない（MUST NOT）。

### ARTIFACT-SET-003

receiptの`csharp[].path`はcanonical artifact rootの`csharp/`からのrelative file pathでなければならず（MUST）、non-empty、directory外へescapeしない、
deterministic order、filesystem namespace上のunique pathでなければならない（MUST）。absolute path、`..`、target-directory relative path、process cwd
relative pathを受理してはならない（MUST NOT）。receipt representationではpath separatorに`/`を使用し、platform filesystem pathへの変換はadapterが行う。

receiptはcurrent canonical C# file setの全fileと`masterdata.bytes`を記述し、それぞれのexact bytesをhashしなければならない（MUST）。binaryのreceipt pathはv1では
`masterdata.bytes`に固定し、arbitrary binary destinationや`build.binary_output`を復活させてはならない（MUST NOT）。receipt対象はexpected regular fileでなければならず、
symlink、directory、special filesystem objectをartifactとしてhashしてはならない（MUST NOT）。
source hash、`schema_source_content_hash`、semantic schema hash、またはcache keyをv1 receiptの必須fieldやpublish eligibility keyへ昇格させてはならない（MUST NOT）。

### ARTIFACT-SET-004

publish operationは、いずれかのexternal targetをpreflightまたはmutationする前に、canonical receiptとartifact setを検証しなければならない（MUST）。
最低限、receiptの存在とregular-file type、supported version、valid shape、current `project.id`との`project_id`一致、supported hash algorithm、全C# pathの
安全性、receiptとactual `csharp/` file setの完全一致、全C# hash、固定binary path、binaryの存在・regular-file type・hash、およびcanonical root entryのexpected typeを確認する。
全検証に成功した場合だけ、setをpublish-eligibleとして扱う。receipt自身はC#またはbinary publish targetへcopyせず、検証済みC# setと`masterdata.bytes`だけを各targetへ配布する。

receiptのproject identityは`project.id`だけであり、directory basename、absolute checkout path、Git repository、current cwd、`project.name`、または`project.version`を
identityへbindしてはならない（MUST NOT）。directoryを移動しても`project.id`が一致する限りreceipt validityを失わせず、`project.id`が変更された場合はmismatchとしてrejectする。
publish targetのkindまたはpathもreceipt identityへbindしてはならず、build後のtarget追加・変更だけでreceiptをinvalidにしてはならない（MUST NOT）。

receipt validationはYAMLの再parse、semantic validation、C#再生成、.NET builder、またはimplicit buildを開始してはならない（MUST NOT）。receipt validation failureは、missing、invalid、
mismatch、project mismatchなどのreason、canonical artifact rootまたはreceipt path、および関連Requirement IDを識別できるstructured diagnosticを返さなければならず（MUST）、
external target parent creation、C# target mutation、publish manifest mutation、stale deletion、binary replacementを開始してはならない（MUST NOT）。
canonical root直下には`csharp/` directory、`masterdata.bytes`、およびreceiptだけをexpected entryとして許可し、それ以外のunexpected entryまたはtypeはrejectしなければならない（MUST）。

### ARTIFACT-SET-005

current YAMLまたはsource treeがlast successful build以降に変更されたことだけを理由に、validなreceipt付きcanonical artifact setのpublishをrejectしてはならない（MUST NOT）。
publishはcurrent sourceのfreshnessをparse、hash、validate、compareしてはならず、source変更を理由にimplicit buildしてはならない（MUST NOT）。YAMLは引き続きbuild時の
canonical source of truthであるが、publish operationのauthorityはlast successful publish-eligible canonical artifact setである。source YAMLが削除またはrenameされても、
current project configをloadでき、receipt validationに成功する限り、そのsetはpublish可能である。

### ARTIFACT-SET-006

receiptがないcanonical rootはpublish-eligibleとみなしてはならない（MUST）。`csharp/`と`masterdata.bytes`だけが存在するpre-receipt setを、publisherはenumerate、
hash、receipt生成によってautomatic adoptしてはならない（MUST NOT）。missing receipt、malformed JSON/shape、unsupported versionまたはhash algorithm、missing required field、
invalid/escaping path、duplicate path、invalid hash、project id mismatch、receipt自身のsymlink/directory/special type、またはartifact setのmissing/extra/tampered entryは
rejectしなければならない（MUST）。

reject時はautomatic repair、migration、inference、stale deletionを行わず、canonical artifact set、receipt、およびexternal destinationを変更してはならない（MUST）。
missingまたはlegacy receiptには、`masterdata build`で新しいcoherent setを生成するguidanceを返す。receipt導入以前のartifactにcompatibility grace periodは設けない。

### ARTIFACT-SET-007

receiptはcanonical artifact-set consistency、accidental/manual artifact driftの検出、missing/extra fileの検出、byte mismatch、およびproject identity mismatchを保証する
integrity receiptである。SHA-256を使用してもreceipt自身が署名されていないため、malicious actorによるartifactとreceiptの同時改変、producer authentication、
supply-chain attestation、remote provenance、artifact signing、またはreleased compatibilityを防止・証明するsecurity authenticity contractではない（MUST NOT）。

receiptはYAML semantics、schema/Table/Type/Reference identity、source of truth、semantic schema hash、compiled builder cache key、incremental compiler identity、または
released compatibility identityとして扱ってはならない（MUST NOT）。

### ARTIFACT-SET-008

publish-valid receiptを生成または更新できるoperationは、complete canonical C# setとvalidated canonical binaryを同じbuild resultとして生成したfull buildに限る（MUST）。
pipeline途中で停止するschema-only、generate-only、validate-only等のoperationは、complete C#とbinaryを生成していない限りreceiptを発行または更新してはならない（MUST NOT）。
これによりNEW C#、OLD binary、または一部だけ更新されたapparently valid receiptを作ってはならない。

`masterdata build --dry-run`はreceiptをcreate、update、delete、invalidateしてはならない（MUST NOT）。C# generation failure、.NET compile failure、DatabaseBuilder failure、
reload validation failure、receipt generation failure、またはcanonical root publication failure時は、通常のI/O failure semanticsの範囲でprevious complete setとmatching
receiptを保持し、receipt generation failureをsuccessとして扱ってはならない（MUST NOT）。

receipt validation後は、`PUBLISH-PATH-001`から`PUBLISH-PATH-010`のall-target preflightへ進み、receipt invalid時にそのpreflightまたはdestination mutationへ到達してはならない。

## Artifact-set receipt acceptance matrix

このmatrixは`ARTIFACT-SET-001`から`ARTIFACT-SET-008`に対する将来のimplementation evidenceであり、現時点のtest passを表さない。すべてpending implementationである。

| Requirement | Planned evidence（pending implementation） | Observation |
| --- | --- | --- |
| ARTIFACT-SET-001 | `full_build_writes_matching_artifact_receipt`; `failed_build_preserves_previous_receipt_and_set` | C#、binary、receiptをwhole-root setとして扱い、通常のI/O failureで旧setを保持する。 |
| ARTIFACT-SET-002 | `artifact_receipt_has_v1_shape_and_sha256_hashes`; `artifact_receipt_is_deterministic` | version、project id、hash algorithm、deterministic representationを確認する。 |
| ARTIFACT-SET-003 | `artifact_receipt_covers_complete_csharp_and_binary_set`; `artifact_receipt_rejects_unsafe_paths` | all C# path、固定binary path、separator、duplicate/escapeを確認する。 |
| ARTIFACT-SET-004 | `publish_validates_receipt_before_external_mutation`; `publish_rejects_tampered_artifact_set` | receipt、actual set、hash、project id、entry typeを検証し、B8 preflightより前に停止する。 |
| ARTIFACT-SET-005 | `publish_accepts_last_receipt_after_current_yaml_change`; `publish_does_not_rebuild_current_sources` | source change、source deletion、publish target変更でreceipt validityを不必要に失わせない。 |
| ARTIFACT-SET-006 | `publish_rejects_missing_or_malformed_receipt_without_mutation`; `publish_rejects_legacy_pre_receipt_set` | automatic adoption/repair/migrationを行わず、旧destinationを変更しない。 |
| ARTIFACT-SET-007 | `artifact_receipt_does_not_claim_authenticity_or_cache_identity` | consistency/integrityと署名・provenance・cache semanticsを区別する。 |
| ARTIFACT-SET-008 | `dry_run_leaves_receipt_untouched`; `partial_build_does_not_issue_receipt`; `receipt_generation_failure_preserves_previous_set` | complete full buildだけがreceiptを発行し、failure時にreceiptだけ更新しない。 |

receiptはexternal C# destinationの`.masterdata-publish-manifest.json`とは異なるmetadataであり、前者を後者のownership sourceとして扱ってはならない。

## Normative requirements: publish targets

### PUBLISH-001

publish targetはcanonical artifactのdistribution destinationを表し、projectは0個以上のtargetを持つことができる（MAY）。v1のconfiguration
surfaceはtarget単位の次の形をcanonical configuration shapeとする。

```toml
[[publish.targets]]
kind = "csharp"
path = "../unity/Assets/MasterData/Generated"

[[publish.targets]]
kind = "binary"
path = "../unity/Assets/StreamingAssets/masterdata.bytes"

[[publish.targets]]
kind = "binary"
path = "../dedicated-server/data/masterdata.bytes"
```

v1のtarget kindは`csharp`と`binary`だけとする。未定義のfuture kindをこのspecificationで仕様化しない。1つのcanonical artifactを複数targetへpublishでき、
targetごとにschema parse、semantic validation、C# generation、またはMasterMemory buildを繰り返してはならない（MUST NOT）。

### PUBLISH-002

relativeなpublish target pathはMasterData project rootを基準にresolveしなければならない（MUST）。process current working directory、source root、
checkout directoryのbasenameをrelative pathの基準にしてはならない（MUST NOT）。

このruleにより、次のmonorepo構成をcwdに依存せず表現できる。

```text
workspace/
├─ master-data/
│  └─ masterdata.toml
└─ unity/
   └─ Assets/
```

`master-data/masterdata.toml`から`../unity/...`を指定する。absolute publish pathも許可し、configured absolute filesystem destinationとして扱う。
relative targetとabsolute targetのいずれも、`PUBLISH-PATH-001`から`PUBLISH-PATH-010`のfilesystem safety validationを受ける。
parent creation、symlink、path containmentの詳細は、同requirementsが所有する。

### PUBLISH-003

publishは、Rustのvalidated/resolved modelから作成され、canonical artifact setとして検証済みのinputだけを使用しなければならない（MUST）。
raw YAML、raw DataDocument、source file order、既存の任意generated C#をpublishのauthorityとして使用してはならない（MUST NOT）。

publish-only operationは、`ARTIFACT-SET-001`から`ARTIFACT-SET-008`に従って検証されたcanonical artifact-set receiptを使って、
last successful build resultをpublish-eligibleと確立しなければならない（MUST）。publishはcurrent YAMLのfreshnessを再検証するためにimplicit buildを開始してはならない（MUST NOT）。
receiptのtrust scope、source freshness、semantic schema hashとの関係は同requirementsが所有する。

### PUBLISH-004

C# publish targetはmanifest-based ownershipを使用しなければならない（MUST）。publish target directory全体をMasterData tool-ownedと
みなしてはならない（MUST NOT）。manifestはtarget directoryの配下に置き、v1のreserved filenameは
`.masterdata-publish-manifest.json`とする。

manifestは、直前のsuccessful publishでMasterData publisherが所有したrelative file pathsを識別するownership metadataに限る。manifestから
schema semantics、generated C#のauthority、semantic schema hash、cache key、またはreleased compatibility identityを復元してはならない（MUST NOT）。
manifestが存在しない初回publishでは、previous managed setは空として扱い、destinationに既にある他のentryを拡張子やcontentからmanagedと推測してはならない（MUST NOT）。
現行implementationにあるlegacy markerは旧artifact migrationの入力候補に限り、新しいpublish targetのownership sourceへ自動昇格させない。

manifestのv1 contentは、少なくともformat versionとmanaged relative pathsを持たなければならない（MUST）。conceptual shapeは次のとおりである。

```json
{
  "version": 1,
  "files": [
    "Enemy.g.cs",
    "Item.g.cs"
  ]
}
```

`files`はtarget directoryからのrelative file pathであり、directory外へescapeしてはならない（MUST NOT）。pathのdeterministic orderingを使用する。
manifest path自体はpublisher-reserved pathであり、generated C# fileがmanifest pathとcollisionしてはならない（MUST NOT）。

### PUBLISH-005

C# publish成功後のmanaged file setは、そのtargetのmanifestに記録されたcurrent setと一致しなければならない（MUST）。publisherは前回manifestの
managed pathsとcanonical C# artifact setの差分を、次のように扱う。

- addition: current canonical pathを追加する。
- update: current canonical contentで同じmanaged pathを更新する。
- removal: 前回managedだがcurrent setにないpathをstale managed artifactとしてretireする。
- rename: 旧managed pathをretireし、新pathを追加する。

例えば前回manifestが`Item.g.cs`、`Enemy.g.cs`、`ItemId.g.cs`を含み、current canonical setが`Item.g.cs`と`Enemy.g.cs`だけなら、
`ItemId.g.cs`はsuccessful publish後に存在してはならない（MUST NOT）。publisherはtarget directory全体を空にして再copyするのではなく、
前回managed setとcurrent setの差分だけを管理しなければならない（MUST）。
この差分管理は、`ItemId.g.cs.meta`のようにmanifestにない隣接entryをstale managed artifactとして扱わない。

### PUBLISH-006

manifestに記録されていないregular fileまたはdirectoryはunmanaged user contentとして扱い、MasterData publisherはsilent deleteまたはsilent
overwriteしてはならない（MUST NOT）。特に`Item.g.cs.meta`、`UserNotes.txt`、`SomeEditorUtility.cs`のようなfileは、manifestに記録されていない限り
MasterData ownership外である。Unityの`.meta` lifecycleを、generated C#とのbasenameが一致することだけを理由にMasterData側で管理してはならない（MUST NOT）。

current canonical C# pathが既存unmanaged entryとfilesystem上で衝突する場合、publishはcollision errorで停止しなければならない（MUST）。
publisherはそのentryを自動adopt、automatic rename、suffix付与、またはgenerated fileによるoverwriteで解決してはならない（MUST NOT）。
file/file、file/directory、directory/file、nested pathなど、next managed setを安全にmaterializeできないpath shapeは同じcollisionとして扱う。

### PUBLISH-007

C# publishの概念pipelineは次のとおりである。

```text
load valid canonical C# artifact set
 -> inspect previous publish manifest
 -> classify previous managed paths
 -> inspect destination unmanaged entries
 -> reject NEW managed vs unmanaged path collisions
 -> prepare complete next managed set
 -> retire previous-managed minus current-generated paths
 -> write next manifest
 -> publish target
```

staging、set switch、rollback、または別のmechanical strategyを使用してよいが、user-owned contentを巻き込むdirectory-wide delete/re-copyを
ownership policyとして導入してはならない（MUST NOT）。

### PUBLISH-008

C# publish targetについて、通常のI/O failureに対する目標状態は次のとおりである。

```text
Ok
 -> complete current managed C# set
 -> stale previous-managed paths absent
 -> unmanaged user content preserved
 -> manifest describes current managed set

Err
 -> previous managed publish set remains usable
 -> unmanaged user content is not silently destroyed
```

このrequirementはfilesystem crashまたはpower-loss時に複数fileを同時atomic commitできることを保証しない。cross-file atomicity、retry、rollbackの
実装strategyはtarget-local failureについて`PUBLISH-EXEC-003`、複数targetのcontinuationとaggregate resultについて
`PUBLISH-EXEC-002`および`PUBLISH-EXEC-005`に従う。successful targetを後続targetのfailureだけを理由に戻すcross-target rollbackや、
複数destinationを同時にcommitするglobal atomicityは保証しない。実装が保証していない同時atomic commitをproduct contractとして説明してはならない（MUST NOT）。

### PUBLISH-009

`kind = "binary"` targetはconfigured explicit file 1個だけをpublisher-ownedとして扱わなければならない（MUST）。同じconfigured target fileに既存の
binaryがある場合、そのfileを新しいcanonical binaryでreplaceできる方向とする。binary publisherはparent directory、sibling file、隣接する`.meta`、または
同じdirectoryの他のentryへownershipを拡張してはならない（MUST NOT）。

binary publishはC# manifestをownership metadataとして使用してはならない（MUST NOT）。binary targetのparent creation、既存fileの具体的なreplace
mechanism、およびsymlink/path safetyは別途定義する。execution-time failureが発生した場合は、既存regular fileがあればそのprevious usable
stateを保持し、initial publishでtargetが存在しなければpartialまたはcorruptなconfigured target pathを公開してはならない（MUST NOT）。
このtarget-local failure safetyと、他targetを継続するaggregate semanticsは`PUBLISH-EXEC-003`から`PUBLISH-EXEC-005`に従う。

### PUBLISH-010

publish targetのpathは、canonical artifactの生成元であるMasterData projectまたはGit repositoryの構造を推測して解決してはならない（MUST NOT）。
monorepoではproject rootから隣接repositoryへrelative pathを指定でき、separate repositoryでは適切なabsoluteまたはrelative filesystem pathを
使用できる方向とする。compilerはrepository topology自体をsemantic inputにしない。

## Normative requirements: publish path safety

### PUBLISH-PATH-001

publish targetのpath比較は、文字列のlexical equalityではなく、targetが存在するdestination
filesystemのnamespaceとentry/object identityを基準にしなければならない（MUST）。filesystemが
同じentry/objectまたは重複namespaceとして扱うpathは、同一pathまたはcollisionとして扱わなければ
ならない（MUST）。既存pathのcanonicalized prefix、filesystem identity、symlink alias、
case-sensitiveまたはcase-insensitiveなdirectory lookup、およびfilesystemが同一entryとして扱う
Unicode spellingをこの判定に含める。

このruleは、少なくとも次の比較へ同じ意味で適用する。

- current generated C# pathとexisting unmanaged entry
- manifest pathとdestination entry、manifest path同士、およびreserved manifest path
- binary targetとcurrent generated C#、manifest、existing unmanaged entry
- target同士のownership region
- publish targetとproject、`masterdata.toml`、source root、canonical artifact root、cache

publisherは`path.to_lowercase()`、ASCII-only normalization、NFCだけ、またはOS名だけを、全
filesystemに通用するequivalence algorithmとして仕様化してはならない（MUST NOT）。同一entryか
どうかをdestination filesystemから確認できる場合、その結果を優先する。確認できない場合の
fail-closedは`PUBLISH-PATH-002`に従う。

### PUBLISH-PATH-002

まだ存在しないpublish targetまたはgenerated pathを、存在しないという理由だけでunsafeと判定しては
ならない（MUST NOT）。publisherは、必要に応じてlongest existing ancestor/prefixをdestination
filesystem上で解決し、残りのmissing tailを保持したpath namespaceとしてcollisionとcontainmentを
判定しなければならない（MUST）。このmechanical strategyはcanonical specificationで特定の
Rust crateまたはOS APIに固定しない。

existing prefix、missing tail、case behavior、symlink、またはnamespaceの安全性を証明できない
場合、publisherはstructured path/config errorでrejectしなければならず（MUST）、destinationへ
mutationを開始してはならない（MUST）。通常のsafeな未存在targetは、検証済みのparent creation
policyに従ってsupportしてよい（MAY）。

### PUBLISH-PATH-003

C# target rootまたはtarget rootへ到達する既存ancestor componentがsymlinkである場合、publishを
開始してはならない（MUST NOT）。publisherはconfigured targetのspellingからsymlink先のtreeを
自動的にownershipしたと推測してはならない。

C# target rootが存在しない場合は、`PUBLISH-PATH-002`および`PUBLISH-PATH-009`のpreflight後に
必要なdirectoryを作成してよい。existing target rootはreal directoryでなければならず（MUST）、
file、symlink、またはspecial filesystem objectの場合はrejectしなければならない（MUST）。
target directory全体をdeleteして再作成するownership strategyは導入してはならない（MUST NOT）。

target内のunmanaged symlinkはuser-owned contentとして扱い、publisherはfollow、delete、または
symlink先のoverwriteをしてはならない（MUST NOT）。current generated pathまたはmanaged pathが
そのsymlinkとfilesystem上でcollisionする場合はrejectする。

### PUBLISH-PATH-004

C# targetのmanaged pathは`PUBLISH-004`のmanifestに記録されたrelative file pathとreserved
manifest fileに限る。manifest外のregular file、directory、`.meta`、およびsymlinkはunmanaged
contentとして保持し、current generated pathがfilesystem上でそのentryとcollisionする場合は
rejectしなければならない（MUST）。publisherはunmanaged entryをautomatic adoption、rename、
suffix付与、delete、またはoverwriteによって解決してはならない（MUST NOT）。

previous manifestに記録されたmanaged pathをretireまたはupdateする場合、destinationの対象は
expectedなregular fileでなければならない（MUST）。managed pathの現在entryがdirectory、symlink、
またはspecial filesystem objectへ置換されている場合、manifest記載だけを根拠に強制delete、
recursive removal、symlink followをしてはならず、ownership mismatchとしてrejectする。

manifest pathとcurrent generated pathがcaseまたはUnicodeを含む異なるspellingでも、destination
filesystemが同じentryとして扱うなら、同じmanaged slotまたはcollisionとして扱う。文字列だけで
stale removalとadditionを別pathだと判断してはならない。

### PUBLISH-PATH-005

manifestが存在しない初回publishではprevious managed setを空として扱う、という`PUBLISH-004`の
ruleを維持する。既存manifestが次のいずれかに該当する場合、publisherはownershipを確定できない
ためpublishをrejectしなければならない（MUST）。

- JSON shape、format version、relative path、path containmentが不正である。
- 未対応のmanifest versionである。
- `files`にfilesystem上同一entryへresolveするduplicate aliasがある。
- manifest自身がregular fileではなく、symlink、directory、またはspecial objectである。

malformedまたはambiguous manifestを削除、修復、merge、再採用してはならない（MUST NOT）。reject
時はstale managed fileを削除せず、unmanaged contentとmanifestを変更してはならない（MUST NOT）。
`.masterdata-publish-manifest.json`はpublisher-reserved pathであり、current generated C# path、
binary target、または別のreserved artifactがdestination filesystem上でこのpathとcollisionする
場合はrejectする。

### PUBLISH-PATH-006

`kind = "binary"` targetのpublisher ownershipはconfigured explicit fileそのものに限る。target
entryが存在しない場合は、safeなexisting ancestorのpreflight後に作成してよい（MAY）。targetが
existing regular fileの場合はcanonical binaryでreplaceしてよい（MAY）。targetがdirectory、
symlink、またはspecial filesystem objectの場合、publisherはrejectしなければならない（MUST）。
targetまでの既存ancestor componentがsymlinkの場合もrejectしなければならない（MUST）。

binary publisherはC# manifestをownership metadataとして使用してはならず（MUST NOT）、parent
directory、sibling file、隣接する`.meta`、または他のdirectory entryを削除・rename・overwriteして
はならない（MUST NOT）。既存regular fileをreplaceできることは、そのparentやsiblingのownershipを
意味しない。

### PUBLISH-PATH-007

publish targetは、destination filesystem上で次のMasterData critical pathと、publisherがその
protected regionを管理・置換できるequalまたはancestor/descendant overlapをしてはならない（MUST）。

- MasterData project rootおよび`masterdata.toml`
- configured source rootsとそのtree
- canonical artifact rootとそのtree
- build cacheとそのtree

この判定には`PUBLISH-PATH-001`のfilesystem equivalenceを適用する。project rootそのもの、または
project rootを含む外側のdirectoryをtargetにする場合はrejectするが、project root内の独立した
`dist/`のようなsafeなdirectoryまで一律に禁止しない。source root、canonical artifact root、cache
については、targetがそのregionの内側・外側のどちらからでもregionを管理・置換できる場合を
rejectする。`masterdata.toml`自身、source YAML、canonical C#、canonical binary、またはcacheを
binary explicit targetにすることも許可しない。

### PUBLISH-PATH-008

複数publish targetはindependentなownership regionとして扱う。target pathがfilesystem上で
equivalent、ancestor/descendant、またはその他のnamespace overlapとなるconfigurationは、
target mutation開始前にrejectしなければならない（MUST）。少なくとも次を含む。

- 同じbinary fileを指すduplicate targetまたはcase/Unicode alias
- C# target内にbinary targetがある構成
- C# target同士のnested overlap
- C# targetとbinary targetが同じentryまたはnamespaceを共有する構成

target kindが同じか異なるかにかかわらずownership regionのoverlapを許可してはならない（MUST NOT）。
target graphのcollisionをoperation orderで解決したり、一方のtargetをautomaticにsubdirectory
ownershipへ変更したりしてはならない（MUST NOT）。

### PUBLISH-PATH-009

relative publish target pathは引き続きMasterData project rootを基準にresolveしなければならない
（MUST）。process current working directory、source root、checkout directory basenameをbaseに
してはならない。absolute publish target pathも許可し、configured absolute filesystem destination
として扱わなければならない（MUST）。relative targetとabsolute targetのいずれも、
`PUBLISH-PATH-001`から`PUBLISH-PATH-008`のsafety checkを受けなければならない（MUST）。

target rootまたはbinary targetのmissing parent directoryは、既存prefix、symlink、protected
path、target graphのpreflightが成功した後に限り作成してよい（MAY）。新規directoryの作成は、
そのparentやsiblingsのownershipをpublisherへ移さない。preflight前にparentを作成してはならない
（MUST NOT）。

### PUBLISH-PATH-010

既知のpath collision、ownership ambiguity、unexpected filesystem type、protected path overlap、
symlink policy違反、およびtarget graph collisionは、いずれかのpublish destinationをmutation
する前に検出してrejectしなければならない（MUST）。0..N targetを全てpreflightしてから、必要な
parent creation、C# file operation、manifest update、binary replacementを開始する。

preflight後にfilesystemが変更された場合、publisherは各mutation時点でunexpected symlink/type
changeを安全側にrejectし、危険なfollow、recursive delete、またはpartial overwriteを行っては
ならない（MUST NOT）。このruleは外部processやuserとのraceをOS-level handleで完全排除すること、
crash/power-loss時のcross-target atomicityを自動的に保証するものではない。通常のI/O failureに
おけるtarget-local failureと複数targetのexecution semanticsは`PUBLISH-EXEC-001`から
`PUBLISH-EXEC-005`に従う。

## Normative requirements: multi-target publish execution

publishは、同じreceipt-valid canonical artifact setを全targetの入力として、次の3 phaseで実行する。

```text
Phase 1: canonical artifact-set receipt validation
    -> invalid: Err, no destination mutation
Phase 2: ALL configured target preflight
    -> any invalid: Err, no target attempted or mutated
Phase 3: target execution
    -> every configured target is attempted
    -> target-local success or failure/rollback
    -> per-target results are aggregated
```

Phase 1は`ARTIFACT-SET-004`から`ARTIFACT-SET-006`、Phase 2は`PUBLISH-PATH-001`から
`PUBLISH-PATH-010`をownerとする。Phase 3とpartial successのaggregate semanticsは、次のrequirementsをownerとする。

### PUBLISH-EXEC-001

receipt validationに成功した後、publisherはconfigured `publish.targets`の全targetに対して
`PUBLISH-PATH-001`から`PUBLISH-PATH-010`のpreflightを実行しなければならない（MUST）。
collision、protected path overlap、symlink violation、manifest ambiguity、ownership ambiguity、
unsupported filesystem entry type、target graph overlap、またはその他のpreflight errorが1件でも
ある場合、publishはoverall `Err`を返さなければならず（MUST）、いずれのtargetもattemptまたは
mutationしてはならない（MUST NOT）。target parentの作成、C# manifestの更新、stale managed fileの
削除、binary fileのreplaceも開始してはならない（MUST NOT）。

preflight failureではexecution phaseが開始されない。per-target structured resultが
`not_attempted`を表現できる場合、その状態を使用してよい（MAY）。receipt validation failureも
このpreflight境界より前のno-mutation failureとして扱う。

### PUBLISH-EXEC-002

all-target preflightが成功した後、publisherはconfigured targetを全てattemptしなければならない
（MUST）。あるtargetのexecution-time failureを理由に、残りのindependent targetをskipしてはならない
（MUST NOT）。targetの実行順序はdeterministicなattempt順序およびdiagnostic/report順序として使用
してよい（MAY）が、`publish.targets`のregistration orderはsuccess dependency graphを意味しては
ならない（MUST NOT）。target Bがtarget Aのsuccessを待つ、またはtarget Aのfailureを理由にBを
`not_attempted`とする意味を導入してはならない（MUST NOT）。

全targetは同じvalidated canonical artifact-set receiptを入力として扱い、targetごとにYAMLを再parse、
semantic validation、C# generation、.NET build、またはimplicit buildを実行してはならない（MUST NOT）。
preflight後のTOCTOUによりtargetのfilesystem type、symlink、collisionまたはownership safetyが
invalidになった場合は、そのtargetのexecution failureとして扱い、残りのtargetのattemptを継続する。

### PUBLISH-EXEC-003

各publish targetは独立したfailure domainであり、execution-time failureが発生したtargetは、その
target自身のprevious usable publish stateを保持または安全にrollbackしなければならない（MUST）。

- C# targetのsuccessはcurrent managed C# setとcurrent manifestをcoherentに公開し、failureはprevious
  managed setをusableな状態に保持し、unmanaged contentを変更してはならない（MUST）。
- binary targetのsuccessはconfigured explicit fileへNEW binaryを公開し、既存regular fileのreplace
  failureではprevious fileをusableな状態に保持しなければならない（MUST）。initial publishでprevious
  fileが存在しない場合、failure後にpartialまたはcorruptなconfigured target pathを残してはならない
  （MUST NOT）。
- target-local rollbackまたはprevious stateの保持自体に失敗した場合、publisherはrollback failureを
  structured diagnosticとして報告し、targetまたはoverall operationをsuccessとして扱ってはならない
  （MUST NOT）。

publisherは、あるtargetのfailureから別targetのownershipを推測したり、別targetのmanifestまたは
fileを修復したりしてはならない（MUST NOT）。

### PUBLISH-EXEC-004

あるtargetがsuccessした後に別targetがfailureしても、後続targetのfailureだけを理由に、既にsuccess
したtargetをprevious stateへrollbackしてはならない（MUST NOT）。partial successは、successful
targetのNEW usable stateを保持したままoverall operationがfailureとなる状態を許可する。
publisherは複数filesystem、repository、volumeをまたぐglobal atomic commit、cross-target transaction
coordinator、または2-phase commitをv1 contractとして保証してはならない（MUST NOT）。

### PUBLISH-EXEC-005

publisherは全targetのattempt完了後にpublish resultをaggregateしなければならない（MUST）。resultは
少なくともtargetごとに`succeeded`または`failed`をstructuredに識別できなければならない（MUST）。
preflight failureでexecution phaseが開始されなかった場合は、必要に応じてtargetを`not_attempted`
として表現してよい（MAY）。execution phase開始後は、先行targetのfailureだけを理由に後続targetを
`not_attempted`としてはならない（MUST NOT）。

- 全targetがsuccessした場合、publishはoverall `Ok`を返す。
- 1件以上のtargetがexecution failure、rollback failure、またはその他のfailureになった場合、他の
  targetがsuccessしていてもpublishはoverall `Err`を返す。
- validなreceiptと0 targetの場合、publishはtarget mutationを伴わないsuccessful no-opとしてoverall
  `Ok`を返す。

partial success後の再実行は、hidden transaction logを前提とせず、current destination stateとtarget-local
manifestを再評価して安全に同じreceipt-valid artifact setへ収束できる方向でなければならない（MUST）。
unchanged fileをskipするかどうかなどのperformance optimizationはこのrequirementで固定しない。

## Filesystem pathの例

以下はpath文字列の比較結果ではなく、実際のdestination filesystemが返すnamespace/identity結果に
基づいて判定する。

- case-insensitiveなfilesystemで`Generated/Item.g.cs`と`generated/item.g.cs`が同じentryになる
  場合、C# generated pathとunmanaged entry、またはtarget同士のcollisionとしてrejectする。case-
  sensitiveなfilesystemで別entryになる場合にまで一律rejectしない。
- `Café.g.cs`と`Café.g.cs`がdestination filesystem上で同一entryになる場合はcollisionとしてreject
  する。filesystemが別entryとして扱う場合、MasterDataが文字列をNFC等へ変換して別意図を作っては
  ならない。
- `../unity/NewGenerated`のようにtarget自体が未存在でも、既存ancestorが安全に解決でき、missing
  tailのnamespaceにcollisionがない場合は、preflight後のparent creationを許可できる。
- target rootまたは既存ancestorがsymlinkである場合、symlink先をtarget namespaceと推測せずrejectする。

## Publish path safety acceptance matrix

このmatrixは`PUBLISH-PATH-001`から`PUBLISH-PATH-010`に対する将来のimplementation evidenceであり、
現時点のtest passを表さない。external filesystem publisherの実装時に、各testを追加または
同等のevidenceへ置き換える（pending implementation）。

| Requirement | Planned evidence（pending implementation） | Observation |
| --- | --- | --- |
| PUBLISH-PATH-001 | `publish_path_rejects_filesystem_equivalent_unmanaged_collision`; `publish_path_rejects_equivalent_binary_targets`; `publish_path_handles_case_sensitive_and_insensitive_volumes` | destination filesystemの実際のnamespaceをprobeし、OS名だけで分岐しない。 |
| PUBLISH-PATH-002 | `publish_path_accepts_safe_missing_target_tail`; `publish_path_rejects_unresolvable_identity_without_mutation` | existing prefix、missing tail、case behaviorの判定不能をfail closedで確認する。 |
| PUBLISH-PATH-003 | `publish_path_rejects_csharp_target_symlink`; `publish_path_rejects_csharp_ancestor_symlink`; `publish_path_never_follows_unmanaged_symlink` | target root/ancestorとtarget内symlinkを分けて確認する。 |
| PUBLISH-PATH-004 | `publish_path_rejects_case_or_unicode_equivalent_unmanaged_csharp_entry`; `publish_path_rejects_managed_path_type_change`; `publish_path_preserves_meta_and_unmanaged_files` | file/file、file/directory、directory/file、nested pathを含める。 |
| PUBLISH-PATH-005 | `publish_path_rejects_malformed_manifest_without_mutation`; `publish_path_rejects_manifest_alias_duplicates`; `publish_path_rejects_reserved_manifest_alias_collision` | missing manifestの初回publishはempty managed setとして確認する。 |
| PUBLISH-PATH-006 | `publish_path_rejects_binary_symlink_or_directory`; `publish_path_replaces_explicit_regular_file`; `publish_path_preserves_binary_siblings` | explicit fileだけがpublisher-ownedであることを確認する。 |
| PUBLISH-PATH-007 | `publish_path_rejects_source_canonical_cache_and_config_overlap`; `publish_path_allows_safe_project_local_dist` | protected path aliasとsafeなproject-local destinationを区別する。 |
| PUBLISH-PATH-008 | `publish_path_rejects_nested_csharp_targets`; `publish_path_rejects_csharp_binary_overlap`; `publish_path_rejects_target_aliases` | missing target同士もdestination namespaceで比較する。 |
| PUBLISH-PATH-009 | `publish_path_resolves_relative_target_from_project_root`; `publish_path_accepts_absolute_target`; `publish_path_creates_missing_parent_after_preflight` | cwd変更とrelative/absolute resolutionを分離して検証する。 |
| PUBLISH-PATH-010 | `publish_path_validates_all_targets_before_mutation`; `publish_path_rejects_type_change_during_mutation`; `publish_path_preserves_previous_destination_on_preflight_error` | crash durabilityはこのmatrixに含めない。execution-time failureとpartial retryは`PUBLISH-EXEC-*`のmatrixで確認する。 |

## Publish execution acceptance matrix

このmatrixは`PUBLISH-EXEC-001`から`PUBLISH-EXEC-005`に対する将来のimplementation evidenceであり、
現時点のtest passを表さない。external filesystem publisherの実装時に、各testを追加または同等の
evidenceへ置き換える（pending implementation）。

| Requirement | Planned evidence（pending implementation） | Observation |
| --- | --- | --- |
| PUBLISH-EXEC-001 | `preflight_failure_mutates_no_targets` | receipt validation後の全target preflightで1件でも失敗した場合、parent creation、manifest更新、binary replacementを含むmutationがない。 |
| PUBLISH-EXEC-002 | `execution_failure_continues_to_later_targets`; `toctou_failure_on_one_target_does_not_skip_others` | execution phase開始後は先行targetのfailureによって後続targetをskipせず、同じreceiptを入力としてattemptする。 |
| PUBLISH-EXEC-003 | `execution_failure_rolls_back_only_failed_target`; `binary_failure_preserves_previous_file`; `initial_binary_failure_does_not_publish_partial_file`; `csharp_failure_preserves_previous_managed_set` | C# managed set/unmanaged contentとbinary explicit fileのtarget-local safety、rollback failureのErrを確認する。 |
| PUBLISH-EXEC-004 | `successful_target_is_not_rolled_back_by_later_failure` | 先行success targetのNEW usable stateを、後続target failureだけを理由に戻さない。global atomicityを主張しない。 |
| PUBLISH-EXEC-005 | `publish_returns_error_after_partial_success`; `publish_reports_per_target_status`; `retry_after_partial_success_converges`; `zero_targets_is_successful_noop` | all succeededのみOk、1件以上のexecution failureはErr、targetごとのstatusと再実行収束を確認する。 |

## Approved configuration contract

project marker、source discovery、およびproject metadataのownerは[Project layout仕様](project-layout.md)である。artifact settingsとpublish
target settingsは、この文書の`BUILD-ARTIFACT-*`および`PUBLISH-*`がownerとなる。以下は承認されたconfiguration shapeである。

```toml
[project]
id = "game.masterdata"
name = "Game Master Data"
version = "0.1.0"

[sources]
roots = ["sources"]

[build]
artifact_dir = ".masterdata/output"
cache = ".masterdata/cache"

[[publish.targets]]
kind = "csharp"
path = "../unity/Assets/MasterData/Generated"

[[publish.targets]]
kind = "binary"
path = "../unity/Assets/StreamingAssets/masterdata.bytes"
```

`artifact_dir`はcanonical build rootを指定し、`publish.targets`はcanonical artifactの外部destinationだけを指定する。`build.binary_output`を
canonical binaryの任意destinationとして再利用しない。target kindを増やす場合は、別途仕様化する。current parser/modelはこのshapeを受理するが、
external publish operationはまだ実行しない。

## Legacy configuration hard cutと手動migration

canonical configuration implementationは`build.output`と`build.binary_output`を即時hard cutで受理してはならない（MUST NOT）。これらを
compatibility alias、warning-only、またはautomatic migrationとして受理してはならない。拒否時は
[Project layout仕様](project-layout.md)の`PROJECT-CONFIG-004`および`PROJECT-CONFIG-005`に従い、structured migration diagnosticを返し、
build/publish作業とfilesystem artifactの変更を開始してはならない。

旧configurationから新configurationへの移行は、ユーザーが旧pathの意図を確認して明示的に設定する手動migrationである。mappingは自動変換規則ではなく、
次のように責務を選び直すためのnon-normative guidanceである。

| 現行設定 | target modelでの扱い | 備考 |
| --- | --- | --- |
| `build.output` | project-local canonical artifactなら`build.artifact_dir`、外部C#配置なら`kind = "csharp"` publish targetへ、ユーザーが明示的に置き換える | 旧pathがcanonicalかexternal destinationかをtoolは推測しない |
| `build.binary_output` | canonical binaryは`.masterdata/output/masterdata.bytes`、外部配置は`kind = "binary"` publish targetへ、ユーザーが明示的に置き換える | 旧pathの責務をtoolは推測しない |
| `build.cache` | `.masterdata/cache`などのcache設定として維持 | canonical outputと混在させない |

旧`.masterdata/generated`、旧binary、外部Generated directory、Unity Assets等の既存artifactは、このconfiguration rejectionによってmove、delete、renameしてはならない。
`schema_source_content_hash`をsemantic artifact identityやbuilder cache keyへ昇格させてはならない。

## Project identityとdirectory naming

projectは引き続き`masterdata.toml`と`project.id`によって識別する。directory basenameはinitやrepository運用上のpresentation conventionとしてkebab-caseを
基本にできるが、table、type、artifact、またはpublish targetのidentityを決めない。checkout directoryをrenameしても`project.id`が変わらない限り
project identityは変わらない。

この整理は、filesystem locationにsemantic meaningを与えない[ADR 0004](../adr/0004-file-location-has-no-semantic-meaning.md)および
[Project layout仕様](project-layout.md)のcurrent identity ruleを変更しない。

## B7 / B8との関係

旧モデルでは、generated C# directoryの配下にbinaryを置く任意cross-placementが、generated file ownershipとbinary ownershipを同じpathから推測する
必要を生じさせていた。Approved canonical modelでは、`.masterdata/output/`全体をtool-owned rootとし、その直下に`csharp/`と`masterdata.bytes`を分離する。
canonical binaryをC# publish destinationへ混在させないため、B7のownership ambiguityはarchitecture上解消される。legacy configurationはhard cutで拒否され、
旧artifactの自動移行は行わない。

外部publish destinationのfilesystem-equivalent path、case equivalence、Unicode spelling、symlink、path containment、およびtarget間overlapは、
`PUBLISH-PATH-001`から`PUBLISH-PATH-010`によって、destination filesystemのnamespaceとidentityを基準に検証する。これはB8のpath-safety
contractを閉じるが、ASCII lowercase、NFC-only normalization、またはOS名によるcase ruleを実装方式として仕様化するものではない。

## 責務境界と非目標

- `masterdata-core`: project/config解決、typed YAML AST、Type System/Table resolution、Build Selection、record/constraint validation、canonical ordering、
  normalized semantic model。
- `masterdata-codegen-csharp`: resolved modelからのC# generation planとcanonical C# artifact materialization。raw YAMLからsemanticを推論しない。
- `masterdata-dotnet`: internal normalized protocol、.NET process invocation、schema-specific builder、MasterMemory/MessagePack compile、DatabaseBuilder、
  MemoryDatabase reload validation。
- `masterdata-app`: build/publish orchestration、staging、canonical artifact publication、将来のpublish target adapter。
- CLI/Tauri: application serviceを呼び出すadapter。YAML semantics、filesystem discovery、または.NET process invocationを複製しない。

このspecificationでは、Reference、named Build Profile adapter、GUI、Unity importer、semantic schema hash、builder cache/reuse、cache eviction、
released-schema binary compatibility、exact binary bytes identity、artifact signing/attestation、Git provenance、publish filesystem copy、CLI command追加、config parser変更を
実装または確定しない。

## Open Questions（未解決事項）

今回のHuman Approvalによって、project-local canonical output、canonical root ownership、C# manifest-based ownership、stale managed file retirement、
unmanaged preservation/collision、binary explicit-file ownership、`[[publish.targets]]` syntax、relative target pathのproject-root base、複数targetの方向、
monorepo/separate repositoryの想定、directory basenameとproject identityの分離、B8の`PUBLISH-PATH-001`から`PUBLISH-PATH-010`、canonical
artifact-set receiptの`ARTIFACT-SET-001`から`ARTIFACT-SET-008`、および複数targetのexecution semanticsである
`PUBLISH-EXEC-001`から`PUBLISH-EXEC-005`はApproved contractとなった。
以下だけを未解決として残す。

- `masterdata build --publish`を提供するか。提供する場合、canonical build成功とpublish失敗をどのようにCLI resultへ表すか。
- semantic schema hash、builder cache key、released-schema binary compatibilityをreceiptと独立したspecificationで定義するか。
- artifact signing、producer authentication、supply-chain attestation、remote provenanceを将来導入する必要があるか。
- Unity `.meta` lifecycleとpublish manifestの連携をMasterData publisherが持つか、Unity importerへ委譲するか。
- generated .NET projectのownership、cache eviction、Unity asset importがartifact publicationをどう観測するか。
- source discoveryでsymlinkをfollowまたはignoreするproduct-level policy。current traversal guardはcycle防止のためsymlink entryをfollowしない。

- canonical layoutを将来`output/binary/`や`output/metadata/`へ分割する必要があるかは、v1 layoutを承認する際に再確認する。

## Statusと実装方針

この文書の`Status: Approved`は、仕様変更0004、0005、0006、0007、および0008に記録されたHuman Approvalとcanonical applicationを反映する。implementation evidenceが揃う前に
`Implemented`へ変更しない。

canonical configuration implementationは`artifact_dir`をproject-local rootとして検証し、complete artifact rootをbuildする。external publish targetのfilesystem
operation、receipt runtime、および`PUBLISH-EXEC-001`から`PUBLISH-EXEC-005`のtarget executionは未実装であり、
`PUBLISH-PATH-001`から`PUBLISH-PATH-010`とともにfuture publisherのpreflight/execution contractである。このdocumentのStatusは`Approved`のまま維持する。
legacy configurationの受理、alias、warning-only、automatic migrationは禁止され、legacy artifactのmove/delete/renameは行わない。
