# Build pipeline仕様（Build pipeline）

Status: Approved

## この文書の位置付け

この文書は、MasterData projectのcanonical build artifactを作成する処理と、作成済みartifactを外部へ配置する処理の
境界を定義するApproved canonical specificationである。`BUILD-ARTIFACT-001`から`BUILD-ARTIFACT-005`および
`PUBLISH-001`から`PUBLISH-010`は、Human maintainerの承認を経てcurrent normative contractとなった。この承認とcanonical
documentへの適用は[仕様変更0004](../spec-changes/0004-canonical-artifacts-publish-targets.md)に記録する。
legacy configurationのhard cutとstructured migration diagnosticの適用は、[仕様変更0005](../spec-changes/0005-legacy-build-path-hard-cut.md)に記録する。

Approvedなdomain semanticsは、[Build Selection仕様](build-selection.md)、[Table / Primary Key / Secondary Key仕様](table-and-keys.md)、
各Type System仕様、[YAML subset仕様](yaml-subset.md)、および[Project layout仕様](project-layout.md)がそれぞれ所有する。
この文書は、これらのdomain semanticsを変更せず、build artifactとpublishのarchitecture boundaryを定義する。`Approved`は
implementation evidenceが揃ったことを意味しない。現行のconfiguration parser、CLI、filesystem publisherはlegacy contractを
実装しているため、対応するimplementation、tests、fixtures、およびmigrationは別taskで行う。

今回のrefinementでは、project-localなcanonical build artifactsと、Unityなどの外部publish destinationsを別の層として扱う。
このcanonical applicationは、configuration parser、CLI、またはproduction implementationを変更しない。影響するcanonical specificationのStatus変更と
configuration contractのreconciliationは、仕様変更0004および0005に記録する。

## 承認されたcanonical model

以下をHuman Approval済みの中心modelとして採用する。

- MasterData projectはproject directory内にcanonical build artifact領域を持つ。
- Unity projectやserver projectなど、project root外を含む外部配置はpublish targetとする。
- monorepoとseparate repositoryのどちらもdeployment topologyとして扱える。compiler semanticsはGit repository topologyに依存しない。
- 1つのcanonical artifactを0個以上のpublish targetへ配布できる。配布先ごとにbuildを繰り返さない。
- canonical artifact領域はMasterData toolが所有するbuild outputであり、user-owned treeへの直接出力とは区別する。
- project directoryのbasenameはconvention上kebab-caseを基本とするが、project identityは`project.id`であり、basenameから導出しない。

上記のapproval記録は仕様変更0004を参照する。implementation evidenceが揃うまで、この文書を`Implemented`へ変更しない。

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
扱ってはならない（MUST NOT）。canonical root内に置くものは、canonical C#、canonical binary、およびartifact setを識別するために別途承認された
tool metadataに限る方向とする。

current `build.output`配下のmanaged/unmanaged file coexistenceは、現行implementationに残るlegacy behaviorであり、canonical root ownershipの代替ではない。
canonical configuration implementationは旧configurationを受理せず、既存legacy artifactを自動削除または自動移行しない。

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
   │  └─ masterdata.bytes
   └─ cache/
```

v1ではbinaryを`output/masterdata.bytes`へ置く。`output/binary/`や`output/metadata/`への分割はこのspecificationで先取りしない。

### BUILD-ARTIFACT-004

canonical C#とcanonical binaryは、同一のvalidated build resultから作成されたcoherent artifact setでなければならない（MUST）。
buildが成功した場合はcompleteなcurrent setを公開し、前回buildで不要になったcanonical generated fileを残してはならない（MUST）。
build失敗時は、最後のcoherent canonical setを利用不能にするpartial setを公開してはならない（MUST NOT）。

canonical artifactの作成は、raw YAMLをauthorityとするvalidation、Type System/Table resolution、Build Selection、必要なconstraint validation、
canonical ordering、およびC#/.NET validationを経た後に行う。canonical artifact自体をsource of truth、semantic schema hash、cache key、または
released compatibility identityとして扱ってはならない（MUST NOT）。

### BUILD-ARTIFACT-005

canonical buildはcanonical artifactだけを作成し、configured external publish targetを暗黙に更新してはならない（MUST NOT）。
`publish`はcanonical artifact setを入力とする別operationでなければならない（MUST）。

`build --publish`のような統合UXは将来候補であり、このrequirementはそのcommandの存在またはexit semanticsを確定しない。

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

`master-data/masterdata.toml`から`../unity/...`を指定する。absolute publish pathを許可するか、parent creation、symlink、path containmentを
どう扱うかは、PUBLISH-002のrelative baseとは別のOpen Questionとして残す。

### PUBLISH-003

publishは、Rustのvalidated/resolved modelから作成され、canonical artifact setとして検証済みのinputだけを使用しなければならない（MUST）。
raw YAML、raw DataDocument、source file order、既存の任意generated C#をpublishのauthorityとして使用してはならない（MUST NOT）。

publish-only operationのためにcanonical artifactがvalidなbuild resultであることを証明するmetadataが必要になる可能性はあるが、
そのmetadata、trust条件、semantic schema hashとの関係はこのspecificationでは定義しない。

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
実装strategyは後段で決めるが、実装が保証していない同時atomic commitをproduct contractとして説明してはならない（MUST NOT）。

### PUBLISH-009

`kind = "binary"` targetはconfigured explicit file 1個だけをpublisher-ownedとして扱わなければならない（MUST）。同じconfigured target fileに既存の
binaryがある場合、そのfileを新しいcanonical binaryでreplaceできる方向とする。binary publisherはparent directory、sibling file、隣接する`.meta`、または
同じdirectoryの他のentryへownershipを拡張してはならない（MUST NOT）。

binary publishはC# manifestをownership metadataとして使用してはならない（MUST NOT）。binary targetのparent creation、既存fileの具体的なreplace
mechanism、およびsymlink/path safetyは別途定義する。

### PUBLISH-010

publish targetのpathは、canonical artifactの生成元であるMasterData projectまたはGit repositoryの構造を推測して解決してはならない（MUST NOT）。
monorepoではproject rootから隣接repositoryへrelative pathを指定でき、separate repositoryでは適切なabsoluteまたはrelative filesystem pathを
使用できる方向とする。compilerはrepository topology自体をsemantic inputにしない。

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
canonical binaryの任意destinationとして再利用しない。target kindを増やす場合は、別途仕様化する。current parser/modelがこのshapeをまだ
受理しないことはimplementation gapであり、legacy configurationを別のcanonical contractとして扱う根拠にはならない。

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
canonical binaryをC# publish destinationへ混在させないため、B7のownership ambiguityはarchitecture上解消される。ただしcurrent implementationの
legacy configurationからの移行は未実施であり、implementation pendingとして扱う。

この整理は、外部publish destinationでのfilesystem-equivalent path、case equivalence、Unicode normalization、symlinkなどの問題を解決しない。
それらはB8としてpublish layerのpath safety implementation/fix scopeに残し、ASCII lowercaseなどの具体的な実装方式をこのspecificationで仕様化しない。

## 責務境界と非目標

- `masterdata-core`: project/config解決、typed YAML AST、Type System/Table resolution、Build Selection、record/constraint validation、canonical ordering、
  normalized semantic model。
- `masterdata-codegen-csharp`: resolved modelからのC# generation planとcanonical C# artifact materialization。raw YAMLからsemanticを推論しない。
- `masterdata-dotnet`: internal normalized protocol、.NET process invocation、schema-specific builder、MasterMemory/MessagePack compile、DatabaseBuilder、
  MemoryDatabase reload validation。
- `masterdata-app`: build/publish orchestration、staging、canonical artifact publication、将来のpublish target adapter。
- CLI/Tauri: application serviceを呼び出すadapter。YAML semantics、filesystem discovery、または.NET process invocationを複製しない。

このspecificationでは、Reference、named Build Profile adapter、GUI、Unity importer、semantic schema hash、builder cache/reuse、cache eviction、
released-schema binary compatibility、exact binary bytes identity、artifact signing/versioning、publish filesystem copy、CLI command追加、config parser変更を
実装または確定しない。

## Open Questions（未解決事項）

今回のHuman Approvalによって、project-local canonical output、canonical root ownership、C# manifest-based ownership、stale managed file retirement、
unmanaged preservation/collision、binary explicit-file ownership、`[[publish.targets]]` syntax、relative target pathのproject-root base、複数targetの方向、
monorepo/separate repositoryの想定、およびdirectory basenameとproject identityの分離はApproved contractとなった。以下だけを未解決として残す。

- publish-onlyが必要とするcanonical artifact metadata、build identity、trust判定、およびmetadataの保存場所。
- external publish pathでのfilesystem-equivalent path、case equivalence、Unicode normalization、symlink、path containmentの正確なpolicy（B8）。
- absolute publish pathを許可するか、外部targetのparent directory作成と既存destinationの詳細なI/O policy。
- 複数publish targetの一部成功時のretry、rollback、再開、およびoperation/exit semantics。複数destinationのcross-path atomicityは保証するか。
- `masterdata build --publish`を提供するか。提供する場合、canonical build成功とpublish失敗をどのようにCLI resultへ表すか。
- canonical artifactのmetadata/version、semantic schema hash、builder cache key、released-schema binary compatibilityをどの独立specificationで定義するか。
- Unity `.meta` lifecycleとpublish manifestの連携をMasterData publisherが持つか、Unity importerへ委譲するか。
- generated .NET projectのownership、cache eviction、Unity asset importがartifact publicationをどう観測するか。
- source discoveryでsymlinkをfollowまたはignoreするproduct-level policy。current traversal guardはcycle防止のためsymlink entryをfollowしない。

- canonical layoutを将来`output/binary/`や`output/metadata/`へ分割する必要があるかは、v1 layoutを承認する際に再確認する。

## Statusと実装方針

この文書の`Status: Approved`は、仕様変更0004および0005に記録されたHuman Approvalとcanonical applicationを反映する。implementation evidenceが揃う前に
`Implemented`へ変更しない。

このlifecycle taskはdocs-onlyであり、Rust、CLI、Tauri、config parser/model、publish filesystem implementation、current `build.output`、current
`build.binary_output`、およびcurrent production behaviorを変更しない。承認されたrequirementを対象にimplementation task、tests、fixturesを別途作成する。
legacy configurationの受理、alias、warning-only、automatic migrationは禁止され、legacy artifactのmove/delete/renameは行わない。
