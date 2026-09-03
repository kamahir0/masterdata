# Build pipeline仕様（Build pipeline）

Status: Draft

## この文書の位置付け

この文書は、MasterData projectのbuild artifactを作成する処理と、作成済みartifactを外部へ配置する処理の
境界を整理するDraftである。ここでのproposalは、`Status: Approved` または `Status: Implemented` のcanonical
specificationに代わるimplementation authorityではない。Approvedなdomain semanticsは、[Build Selection仕様](build-selection.md)、
[Table / Primary Key / Secondary Key仕様](table-and-keys.md)、各Type System仕様、[YAML subset仕様](yaml-subset.md)、および
[Project layout仕様](project-layout.md)がそれぞれ所有する。

今回のrefinementでは、project-localなcanonical build artifactsと、Unityなどの外部publish destinationsを別の層として
扱う方向を採用する。この文書は引き続きDraftであり、未確定のobservable behaviorはOpen Questionsに残す。今回の変更は
specification status、Approved specification、configuration parser、CLI、またはproduction implementationを変更しない。

## Human directionを受けたDraftの中心proposal

今回のrefinementでは、次の方向をこのDraftの前提として整理する。

- MasterData projectはproject directory内にcanonical build artifact領域を持つ。
- Unity projectやserver projectなど、project root外を含む外部配置はpublish targetとする。
- monorepoとseparate repositoryのどちらもdeployment topologyとして扱える。compiler semanticsはGit repository topologyに依存しない。
- 1つのcanonical artifactを0個以上のpublish targetへ配布できる。配布先ごとにbuildを繰り返さない。
- canonical artifact領域はMasterData toolが所有するbuild outputであり、user-owned treeへの直接出力とは区別する。
- project directoryのbasenameはconvention上kebab-caseを基本とするが、project identityは `project.id` であり、basenameから導出しない。

上記はこのDraftで採用するarchitecture directionであり、Human approvalなしに `Approved` へ昇格させない。

## 提案するpipeline

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

ここで `build` はcanonical artifactを作成するoperationであり、外部publishを暗黙に含めない方向をDraft proposalとする。
`publish` は検証済みcanonical artifact setを外部destinationへ配布する別operationである。

Build Selectionとselected datasetに対するconstraint validationの順序は、[Build Selection仕様](build-selection.md)の
`BUILD-SELECT-010`、`BUILD-SELECT-011`、および`BUILD-SELECT-017`に従う。pipelineはselection前のprofile-independent
validationと、selection後のdataset-level validationを混同してはならない。

Rust coreはproject/config解決、YAMLのtyped AST、semantic validation、Type System/Table resolution、BuildPlan、および
canonical artifact生成に必要なvalidated modelを担当する。`masterdata-codegen-csharp` はstructured C# loweringを担当し、
MasterMemory binary formatとSource Generatorのbehaviorは.NET dependencyに残す。`.NET` process invocationは
`masterdata-dotnet` に集約し、application serviceはcanonical stagingとpublish orchestrationを担当する。CLIとTauriはこの
shared workflowを呼び出し、domain semanticsまたは.NET invocationを複製しない。

generated C#のtype、property、constructor parameterのidentifier contractは[C#命名仕様](type-system/csharp-naming.md)が所有する。
build pipelineは、source nameのnormalization、transliteration、suffix付与、または自動repairによってこのcontractを置き換えてはならない。

## Canonical build artifacts

### project-localなartifact root

v1のcanonical artifact rootは、原則として次のproject-local pathとする。

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

このDraftでは、v1に必要なbinaryを `output/masterdata.bytes` に置く構成を第一候補とする。将来、binaryやmetadataを
subdirectoryへ分ける必要が生じた場合は、別の仕様refinementで扱う。今回のDraftはその将来layoutを先取りしない。

canonical rootには、generated C#、MasterMemory binary、およびそれらを識別するために必要なtool metadataだけを置く方向とする。
canonical rootはtool-ownedであり、通常のuser-owned fileとのcoexistenceを前提にしない。このownershipは、現在の
`build.output` 配下で行っているmanaged/unmanaged file coexistenceの一時的なmigration behaviorとは区別する。

canonical C#とbinaryは1回のvalidated build resultに由来するcoherent artifact setでなければならない。build失敗時に最後の
coherent setを破壊してはならず、成功時にpartial setや前回buildのstale generated fileを残してはならない。これは
[YAMLをSource of TruthとするADR](../adr/0001-yaml-is-source-of-truth.md)および、schema revision・generated C#・binaryを
coherent artifact setとして扱う[MessagePack key ADR](../adr/0005-messagepack-key-as-serialization-only.md)のarchitecture directionに従う。

canonical outputのdefault locationがあるため、通常のcanonical `build` は任意の外部binary pathを必須入力にしない方向とする。
ただし現行implementationは `build.binary_output` を明示的に要求するため、この変更は将来のimplementation/migration taskで行い、
今回のrefinementだけでは挙動を変更しない。

### artifact directory設定のDraft proposal

canonical rootを上書き可能にする場合は、現行の `build.output` とは別に次のkeyを候補とする。

```toml
[build]
artifact_dir = ".masterdata/output"
cache = ".masterdata/cache"
```

`artifact_dir` はproject rootを基準とするrelative pathを第一候補とし、resolved pathがproject directory外へ出ないことを
想定する。absolute path、`..` によるescape、symlinkを含むrootの扱いなど、正確なpath validationはimplementation taskで
既存のpath policyと照合する。canonical artifactの内部layoutは `csharp/` と `masterdata.bytes` を固定する方向であり、
binary pathを別の任意pathから推論しない。

`cache` はcanonical outputとは別の領域である。`schema_source_content_hash` はsource bytesのhashであり、semantic schema hashまたは
builder cache keyではない。これをartifact identityや将来のbuilder reuse cache keyとして流用しない。

### project identityとdirectory naming

projectは引き続き `masterdata.toml` と `project.id` によって識別する。project directoryのbasenameは、initやrepository運用上の
presentation conventionとしてkebab-caseを基本にできるが、table/type/artifactのidentityを決めない。checkout directoryをrenameしても
`project.id` が変わらない限りproject identityは変わらない。

この整理は、filesystem locationにsemantic meaningを与えない[ADR 0004](../adr/0004-file-location-has-no-semantic-meaning.md)および
[Project layout仕様](project-layout.md)のcurrent identity ruleを変更しない。

## Publish targets

### 責務と構成

publish targetはcanonical artifactのdistribution destinationを表す。targetは0個以上定義でき、同じartifact kindに複数の
destinationを持てる。publishはraw YAML、raw DataDocument、既存のgenerated C#、またはsource file orderを入力にしてはならない。
canonical artifact setがRustのvalidated/resolved modelから作成され、検証済みであることを前提にする。

設定syntaxは次のtarget単位の形を第一候補とする。

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

`kind = "csharp"` はcanonical C# artifact setをdirectory destinationへ配布し、`kind = "binary"` はcanonical binaryを
explicit file destinationへ配布する方向とする。relative pathとabsolute pathの両方をmonorepo/separate repositoryのdeployment topology
で利用できるようにする候補である。relative pathのbase、parent creation、targetの重複、symlink、path containment、および既存destinationの
overwrite policyはOpen Questionとして残す。

1つのcanonical C# setからUnity Client A/Bへ、1つのbinaryからUnityとdedicated serverへpublishできる。destinationごとにschemaを
再parseしたり、builderを再実行したりしてはならない。compilerはGit repositoryの境界、remote、branch、またはcheckout basenameを
検査してdeployment semanticsを決めない。

### publish destinationのownership

publish destinationはuser-owned treeである可能性があるため、canonical rootとは別のownership policyが必要である。次の候補を
比較対象として記録する。

| artifact | 候補 | 保護される境界 |
| --- | --- | --- |
| C# | dedicated destination directoryをtool-ownedと明示する | directory内のgenerated setを一括管理し、未知のuser fileを黙って削除しない境界を別途定義する |
| C# | generated-file manifestでtool-owned fileだけを管理する | unmanaged fileを保持し、manifestにないfileを上書き・削除しない |
| binary | explicit file target | 指定された1つのfileだけをpublisher-ownedとして扱う |

どの候補をpublish v1のcanonical ownership policyとするかは、外部destinationにuser-owned contentが存在するか、stale fileをどう
retireするか、manifestをどこで管理するかに依存する。新しいpublish ownershipをこのDraftで暗黙に確定せず、Open Questionsに残す。
いずれの候補でも、user-owned fileをsilent delete、silent overwrite、またはgenerated nameの自動repairで置き換えてはならない。

### artifact buildとpublishの成功状態

canonical `build` の成功と、全publish targetへの配布成功は別のoperation resultとして扱う方向とする。

```text
canonical build success
  -> canonical artifacts are available
  -> publish may be attempted independently

publish target A success + target B failure
  -> canonical build remains successful
  -> publish operation reports distribution failure
```

publish target間のpartial successをrollbackするか、再実行時にどのtargetを対象とするか、canonical artifactのversion/identityを
どのように確認するかは未確定である。filesystem transactionとして複数の外部destinationを同時atomic commitできるとは主張しない。

publish-only operationを許可する場合、既存canonical artifactを「最後のvalid build result」と信頼するためのtool metadataまたは
build identityが必要になる可能性がある。metadataのexact shape、trust条件、semantic schema hashとの関係は今回定義しない。
publishがraw YAMLや任意の既存generated C#をauthorityとしてbypassすることは禁止する。

## Operation semanticsのDraft proposal

通常operationは次の責務を持つ方向とする。

| operation | 処理 | canonical artifact | publish target |
| --- | --- | --- | --- |
| `validate` | YAML parse、profile-independent validation、selection、およびsemantic validation | 書き込まない | 実行しない |
| `schema` | validation後にC# generation planを作成し、canonical C# artifactを作成する | C#のみ | 実行しない |
| `build` | validation、C# generation、MasterMemory binary build/reload validationを行う | C#とbinary | 実行しない |
| `publish` | validなcanonical artifact setをconfigured targetsへ配布する | 変更しない | 実行する |

`masterdata build --publish` のような統合UXは将来候補であり、このDraftではcommandの存在、失敗時のexit semantics、targetごとの
retry/rollbackを確定しない。operationを統合する場合も、publishがpipeline途中のvalidationを飛ばすbypassになってはならない。

### dry-run

`build --dry-run` はvalidationとplan作成までにとどまり、canonical output、publish target、binary、generated C#のfinal pathを変更しない。
canonical outputが標準化されても、dry-runがfilesystem上のartifactを作成したように見せない方向を維持する。

## 現行設定からのmigration

現行のconfiguration ownerは `Status: Implemented` の[Project layout仕様](project-layout.md)であり、現在のparser/modelをこの
refinementで削除しない。次のmigration mappingをDraft proposalとして記録する。

| 現行設定 | target modelでの扱い | 備考 |
| --- | --- | --- |
| `build.output` | `build.artifact_dir` に置き換える候補 | 現行はgenerated C# directoryとして機能している。外部C#配置はpublish targetへ分離する |
| `build.binary_output` | canonical root内の固定binary pathに置き換える候補 | 外部binary配置はbinary publish targetへ分離する |
| `build.cache` | `.masterdata/cache` などのcache設定として維持 | canonical outputと混在させず、source-content hashをcache identityへ昇格させない |

既存設定を直ちに削除、拒否、または自動変換するimplementationは今回行わない。legacy configを受理する期間、warning/error、
既存artifactからcanonical layoutへ移行する方法、および既存のmanaged/unmanaged generated outputとの関係は、implementation taskの
前にmigration decisionとして確定する必要がある。

新modelでは、canonical rootをproject-localなtool-owned directoryとして扱うため、旧モデルの

```text
Generated/
├─ Item.g.cs
└─ artifacts/masterdata.bytes
```

のようなgenerated C# directory配下へのnested binaryはcanonical layoutに含めない。binaryを `csharp/` の外側に置くことで、
binaryのownershipをgenerated-file ownershipと同時に推論する必要がなくなる。これはB7のownership ambiguityを解消する方向であるが、
現行設定・既存artifactを自動移行する実装を意味しない。

## Artifact coherenceとsafetyの境界

canonical buildは、validated/resolved modelからcompleteなC# setとbinaryを同一staging workspaceで準備し、必要なcompile/reload validationが
成功した後にcanonical rootへpublishする。失敗時に既存のcoherent canonical setを残すためのstaging、set publication、atomic binary
replacement、rollbackの具体的なimplementationはapplication layerが所有する。

このDraftはfilesystem crashやpower-loss時にC#とbinaryを同時atomic commitできることを保証しない。外部publish targetについても、
canonical artifact作成とcross-path distributionを同一filesystem transactionとして扱わない。実装が保証していないcross-artifact
transactionalityをproduct contractとして説明してはならない。

canonical rootのtool-owned boundaryはB3/B4/B5のようなuser fileとgenerated/binary pathの衝突を限定する方向だが、publish destination
には同じpath safetyが必要である。case sensitivity、Unicode normalization、symlink、directory replacement、manifest ownershipの
詳細は実装時に別途確認し、今回のDraftで新しいfilesystem policyを発明しない。
canonical rootをproject-localなtool-owned領域へ整理することは、publish destinationに残るfilesystem path equivalenceやUnicode
normalizationの問題（B8）を解消するものではない。B8はpublish layerのimplementation/fix scopeとして別途扱う。

## Build pipelineの責務境界

- `masterdata-core`: project/config解決、typed YAML AST、Type System/Table resolution、Build Selection、record/constraint validation、
  canonical ordering、normalized semantic model。
- `masterdata-codegen-csharp`: resolved modelからのC# generation planとcanonical C# artifact materialization。raw YAMLからsemanticを推論しない。
- `masterdata-dotnet`: internal normalized protocol、.NET process invocation、schema-specific builder、MasterMemory/MessagePackのcompile、
  DatabaseBuilder、MemoryDatabase reload validation。
- `masterdata-app`: build/publish operationのorchestration、staging、canonical artifact publication、将来のpublish target adapter。
- CLI/Tauri: application serviceを呼び出すadapter。YAML semantics、filesystem discovery、または.NET process invocationを複製しない。

`masterdata-core` が公開する `schema_source_content_hash` はsource bytesのdiagnostic/change-detection inputであり、semantic artifact
identity、released binary compatibility、またはbuilder cache reuseを意味しない。Reference、semantic schema hash、builder cache、released
binary compatibility、production artifact versioningはこのDraftの実装対象ではない。

## Open Questions（未解決事項）

今回のHuman directionによって、canonical outputをproject-local build layerとし、外部配置をpublish layerとして分離する方向、
monorepo/separate repositoryの両方を想定すること、複数publish targetを持つ方向、およびdirectory basenameをproject identityにしない
ことは、このDraftの前提として整理した。以下は依然として未解決である。

- `[[publish.targets]]` と `[[publish.csharp]]` / `[[publish.binary]]` のどちらをcanonical configuration syntaxとするか。
- `build.artifact_dir` のproject-local path validation、absolute path、`..`、symlink、親directory作成、およびcanonical root内のmetadata layout。
- canonical rootのmigration時に、現行 `build.output` / `build.binary_output` と既存artifactをどの順序・diagnostic・compatibility policyで扱うか。
- publish C# targetをdedicated tool-owned directoryとするか、manifest-based ownershipとするか。user-owned entry、stale file、rename、overwriteをどう扱うか。
- publish binary targetのexplicit file ownership、既存file replacement、parent creation、case/Unicode/symlink path safety。
- publish-onlyが必要とするcanonical artifact metadata、build identity、trust判定、およびmetadataの保存場所。
- 複数publish targetの一部成功時のretry、rollback、再開、およびoperation/exit semantics。複数destinationのcross-path atomicityは保証するか。
- `masterdata build --publish` を提供するか。提供する場合、canonical build成功とpublish失敗をどのようにCLI resultへ表すか。
- `schema` operationでcanonical C#だけを作成するか、metadataを含むartifact setとして扱うか。
- canonical layoutを将来 `output/binary/` や `output/metadata/` へ分割する必要があるか。
- generated .NET projectのownership、cache eviction、Unity asset importがartifact publicationをどう観測するか。
- source discoveryでsymlinkをfollowまたはignoreするproduct-level policy。current traversal guardはcycle防止のためsymlink entryをfollowしない。
- semantic schema hash、builder cache key、artifact version、released-schema binary compatibilityをどの独立specificationで定義するか。

## 非目標

このDraft refinementは次を実装または確定しない。

- Reference semantics、named Build Profile adapter、GUI、Unity importer。
- semantic schema hash、builder cache/reuse、cache eviction。
- released-schema binary compatibility、exact binary bytes identity、artifact signing/versioning。
- canonical output migration、publish filesystem copy、atomic directory replacement、CLI command追加、config parser変更。

これらは、Human review後に必要なApproved/Implemented specificationとimplementation taskへ分離する。
