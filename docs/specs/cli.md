# CLI surface仕様

Status: Approved

この文書は、MasterDataのpublic CLI terminology、canonical command surface、および
複数のsemantic operationをCLIからcompositionする規則を定義する。StatusはApprovedであり、
CLI surfaceのcurrent canonical authorityである。既存のApproved specificationが所有する
build、artifact receipt、publish、host capabilityの意味を再定義せず、それらを参照してCLI
surfaceへ写像することだけを所有する。

適用した仕様変更は
[0011-cli-surface-and-schema-migration](../spec-changes/0011-cli-surface-and-schema-migration.md)
である。

## 用語

### Operation

Operationは、applicationまたはdomainが提供するfrontend非依存のsemantic operationで
ある。CLI syntax、Tauri command、RPC method、Web UI eventはOperationそのものではない。
Operationのsemantic ownerは、buildについては
[build pipeline仕様](build-pipeline.md)、migrationについては
[Schema Migration v1仕様](schema-migration.md)など、個別のcanonical specificationで
管理する。

### CLI Command

CLI Commandは、Operationをpublic CLI surfaceとして呼び出す名前とargument surfaceで
ある。CLI Commandは、対応するOperationのdomain logicを複製または再定義しない。

### Capability

Capabilityは、runtime hostが特定のOperationを実行できる能力である。CLI Commandと
Capabilityは同義ではない。例えば、Connected Webの`build`利用可否はplatform名では
なくNative Hostのadvertised/granted capabilityで決まり、詳細は
[runtime hosts仕様](runtime-hosts.md)が所有する。

## Normative Requirements

### CLI-001

CLIの仕様は、Operation、CLI Command、Capabilityを別conceptとして扱わなければならない
（MUST）。CLI Commandは対応するOperationのentrypointであり、domain semanticの第二の
実装またはhost capabilityの別名になってはならない（MUST NOT）。

### CLI-002

canonical public CLI command nameは、次の7つである。

| CLI command | 主なOperation | semantic owner |
| --- | --- | --- |
| `init` | 新規projectの初期化 | project layout仕様 |
| `doctor` | project/environment診断 | application/diagnostic contract（詳細は別途） |
| `validate` | canonical sourceの検証 | build selection、schema、table/type仕様 |
| `generate` | validation後のC# generationまで | 本CLI仕様（materializationはOpen Question） |
| `build` | coherent canonical artifact setの生成 | build pipeline仕様 |
| `publish` | 既存artifact setのexternal配布 | build pipeline仕様 |
| `migrate` | schema-aware deterministic transformation | Schema Migration v1仕様 |

この表のcommand nameはcanonical surfaceである。ただし、実装済みであることを意味せず、
deprecation policy、argument grammar、output schemaをこの仕様で確定しない。`generate`
の出力先と`migrate`のargument grammarはOpen Questionとして扱う。

### CLI-003

`masterdata validate`は、canonical YAML sourceについて必要なproject/source loading、
parse、validation、semantic resolution、および既存のselection contractに従う検証を
行い、artifact生成を行わずに停止しなければならない（MUST）。既存artifactや生成済み
C#をvalidationのauthorityにしてはならず（MUST NOT）、validationを理由にcanonical
artifact set、external publish target、publish manifestを変更してはならない（MUST NOT）。

このrequirementは既存のschema、type、table、Build Selection、project仕様をCLI側へ
複製するものではない。

### CLI-004

`masterdata generate`は、概念上、次の順序で実行するOperationを公開する。

```text
resolve project
→ load canonical YAML source
→ parse and validate
→ semantic resolution / build selection
→ C# generation
→ stop
```

`generate`は既存のgenerated C#をauthorityとして使用してはならず（MUST NOT）、validation
をskipしてはならず（MUST NOT）、raw YAMLを.NETへ渡したり、MasterMemory binaryを生成
したり、publish targetを変更したりしてはならない（MUST NOT）。

`generate`は現在のcanonical artifact setを部分的に置換または破壊してはならない
（MUST NOT）。C#生成物をstdoutへ出すか、explicit output directoryへ出すか、tool-owned
non-canonical areaへ出すかなど、最終的なmaterialization先はこの仕様では決めない。
この未決定事項は「Open Questions」のOQ-Aで管理する。

### CLI-005

`masterdata build`は、[build pipeline仕様](build-pipeline.md)が定義するfull buildを
呼び出し、canonical C#、canonical binary、その他Approvedなartifact-set metadataを
coherent setとして生成するOperationを公開する。buildのartifact、receipt、failure、
staging、およびcanonical root publicationの意味をCLI仕様が再定義してはならない
（MUST NOT）。

`build`単体はexternal publish targetを更新してはならない（MUST NOT）。これは
`BUILD-ARTIFACT-005`およびbuild pipeline仕様の責務分離に従う。

### CLI-006

`masterdata publish`はimplicit buildではなく、既存のpublish-eligible canonical artifact
setをexternal targetsへ配布するOperationを公開する。standalone publishは、少なくとも
次の既存Approved pipelineをそのまま使用しなければならない（MUST）。

```text
existing canonical artifact set
→ artifact-set receipt validation
→ artifact integrity validation
→ all-target path preflight
→ target-local publish execution
→ aggregate result
```

`publish`はcurrent YAMLを再parse、再validate、freshness比較、implicit buildしてはならない
（MUST NOT）。current YAMLがlast successful artifact set以後に変更または未完成であっても、
receipt-validなlast successful artifact setはpublish eligibilityを失わない。receipt、
PUBLISH、PUBLISH-PATH、PUBLISH-EXECの各semanticは、それぞれのowner specificationを
参照し、CLI仕様で複製しない。

### CLI-007

`masterdata build --publish`は、次のcompositionを持つ正式なconvenience UXとして定義する。

```text
build
↓ build success only
publish
```

このcompositionは、次を満たさなければならない（MUST）。

1. buildが失敗した場合、publishを開始してはならない。
2. build成功時点で確定したcanonical artifact setは、後続publishの失敗だけを理由に
   rollbackしてはならない。
3. publishはstandalone `publish`と同じreceipt validation、all-target preflight、
   target-local failure、continue-after-failure、およびaggregate result semanticsを
   使用する。
4. publish aggregate resultがfailureなら、`build --publish`全体をsuccessとして報告して
   はならない（MUST NOT）。ただし結果表現は、buildが成功しcanonical artifact setが
   確定したことと、publishが失敗またはpartial failureだったことを区別可能にしなければ
   ならない（MUST）。
5. `build`に`--publish`がない場合、external publish targetを更新してはならない。

このrequirementはbuildまたはpublishのdomain semanticを複製せず、CLI compositionの
順序と結果境界だけを定義する。

### CLI-008

短縮option `-p`は今回予約または仕様化しない（MUST NOT）。convenience compositionの
canonical surfaceで指定する正式形は`--publish`だけである。その他のshort option、global
`--json`、stdout/stderr、exit code taxonomyはこの仕様では固定しない。

### CLI-009

`masterdata migrate`は、Schema Migration v1仕様が定義するschema-aware deterministic
transformationを公開するtop-level CLI commandとして分類する。`migrate`のsubcommand、
argument grammar、SQL-like syntax、JSON plan schemaはこのCLI仕様で固定してはならない
（MUST NOT）。Migrationのsemantic AST、対象解決、resolution closure、operation-specific
postcondition、destructive authorization、source commitは[Schema Migration v1仕様](schema-migration.md)が所有する。
Migration executionのsuccessはProject全体がerror-freeであることを意味せず、Migration
Resolvableとoperation-specific postconditionを満たしたことを意味する。project-wide
diagnosticsとCLI resultの分離方法はSchema Migration仕様の未決定output contractへ委譲する。

### CLI-010

CLI commandの実行可否を、単なるplatform名と同義に扱ってはならない（MUST NOT）。Native
CLIは`NativeApplicationService`をdirect/in-processで使用し、Web対応のためにlocalhost
RPC、daemon、network serialization、async runtimeを必須化してはならない。Connected Web
やTauriは同じNative application semanticsをhost adapterから利用できるが、command
surfaceとruntime capabilityは別に判定する。これは`RUNTIME-HOST-002`、
`RUNTIME-HOST-005`、`RUNTIME-HOST-006`と整合する。

### CLI-011

`CLI-011`は、source-derived staged operationである`validate`、`generate`、`build`に
適用する。これらのOperationは、source resolve、parse、semantic validation / resolution
という前段を飛ばしてはならない（MUST NOT）。後段のartifact生成を行わず前段の境界で
停止することは許可されるが、後段のsource-derived Operationがvalidation bypassを行って
はならない。

`publish`はsource-derived staged pipelineの後段stageではない。`publish`のpreconditionは
publish-eligible canonical artifact setであり、current YAMLのparse、validation、freshness
comparison、implicit buildをCLI specificationが要求してはならない（MUST NOT）。receipt
validation、artifact integrity validation、target preflight、target executionは
[build pipeline仕様](build-pipeline.md)が所有する。この区別により、「途中までで止める
ことはできるが、source-derived前段を飛ばしてはならない」というruleと、Approved publish
semanticsを両立させる。

## 現行実装との差分

現在のCLI実装が提供するcommandは、`init`、`doctor`、`project-info`、`validate`、`build`
である。したがって、canonical surfaceとの差分は次のとおりである。

- `project-info`は現行実装に存在するが、今回のtarget canonical public command setには
  含めない。
- このdocs-only canonicalizationでは`project-info`を削除、rename、別namespaceへ移動しない。
- `generate`、`publish`、`migrate`、`build --publish`は未実装であり、Implementation Gap
  として扱う。
- `project-info`の将来のdiagnostics/info系surfaceは、この仕様では代替案を確定しない。

この差分は仕様を実装済みと示すものではない。`artifact-set receipt runtime`、external
publish runtime、Build Profile CLI wiringも、command surfaceとは別の既存Implementation
Gapである。

## Capabilityとの関係

CLI commandは、runtime host capabilityの有無を自動的に意味しない。特に、Standalone Web
はauthoring/validationを提供できてもNative build/publish capabilityを持たず、Connected
Webはauthorized Native Hostがadvertiseしたcapabilityに応じて同じNative Operationを
利用する。CLIはNative application serviceをdirectに呼び出すため、CLI利用にNative Host
process、pairing、Web handshakeを要求しない。

## 固定しないCLI事項

次の事項はこのApproved surfaceに関連する未決定事項として固定しない。

- global `--json`、machine-readable output schema、stdout/stderr contract
- exit code taxonomy全体
- Build ProfileのCLI syntax
- `migrate`のargument grammar、SQL-like grammar、short options
- CLI deprecation/versioning policy
- `generate`のmaterialization destination
- `build --publish`の詳細なconsole/result serialization

## Acceptance matrix（future evidence）

この文書はApprovedだが、以下のevidenceはすべて実装前のplanned evidenceである。未実施のtestを
pass済みとは扱わない。

| Requirement | Planned evidence | Status |
| --- | --- | --- |
| CLI-001, CLI-010 | CLI/Tauri/Connected Webが同じOperation ownerを使い、CLI direct pathにRPCがないことを確認するarchitecture/integration evidence | pending implementation |
| CLI-002 | canonical command surfaceと現行実装gapのCLI acceptance test | pending implementation |
| CLI-003 | `validate`がartifact、publish target、manifestを変更しないtest | pending implementation |
| CLI-004 | generateのvalidation順序とcanonical artifact set非破壊境界のtest（materialization決定後） | pending implementation |
| CLI-005 | buildがcoherent canonical artifact setを生成し、単体ではexternal targetを変更しないtest | pending implementation |
| CLI-006 | source変更後もreceipt-valid artifact setをpublishし、implicit build/revalidateしないtest | pending implementation |
| CLI-007 | build失敗時のpublish未開始、publish失敗時のbuild保持、partial failure集約のtest | pending implementation |
| CLI-008 | `-p`を受理する未承認compatibilityが存在しないことのCLI test | pending implementation |
| CLI-009 | migrateがSchema Migration engineへ委譲され、CLIがsemantic logicを複製しないtest | pending implementation |
| CLI-011 | source-derived validate/generate/buildが前段をskipせず、publishがsource validationを要求しないことのCLI pipeline test | pending implementation |

## Open Questions

### OQ-A: Generate materialization

`generate`が生成したC#をstdout、explicit output directory、dedicated non-canonical
tool-owned areaのどこへmaterializeするかは未決定である。canonical full build artifact
setのC#だけを部分更新する方式は採用できない。destination、cleanup、
existing generated filesとの関係を発明してはならない。ユーザー向けproject directory
layoutや`.masterdata/generated/csharp`等のrecommended layoutも、この仕様では決めない。

### OQ-B: CLI result contract

`build --publish`におけるbuild successとpublish failure/partial failureの区別を、どの
structured result、console形式、exit codeで表現するかは未決定である。CLI-007のsemantic
result境界だけをこの仕様で定義し、global output/exit contractは別途決める。

### OQ-C: Public argument surface

`migrate`のargument grammar、Build Profile選択syntax、global machine-readable output、
versioning/deprecation policyは未決定である。

## Non-goals

この仕様は、CLI parser、Tauri command、Web UI、Native Host、migration engine、YAML
rewrite、receipt runtime、external publisher、`project-info` removalを実装または確定
しない。
