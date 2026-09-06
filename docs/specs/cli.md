# CLI surface仕様

Status: Approved

この文書は、MasterDataのpublic CLI terminology、canonical command surface、および
複数のsemantic operationをCLIからcompositionする規則を定義する。StatusはApprovedであり、
CLI surfaceのcurrent canonical authorityである。既存のApproved specificationが所有する
build、artifact receipt、publish、host capabilityの意味を再定義せず、それらを参照してCLI
surfaceへ写像することだけを所有する。

適用した仕様変更は、
[0011-cli-surface-and-schema-migration](../spec-changes/0011-cli-surface-and-schema-migration.md)および
[0012-project-conventions-and-generate-removal](../spec-changes/0012-project-conventions-and-generate-removal.md)
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

canonical public CLI command nameは、次の6つである。

| CLI command | 主なOperation | semantic owner |
| --- | --- | --- |
| `init` | 新規projectの初期化 | project layout仕様 |
| `doctor` | project/environment診断 | application/diagnostic contract（詳細は別途） |
| `validate` | canonical sourceの検証 | build selection、schema、table/type仕様 |
| `build [--publish]` | coherent canonical artifact setの生成、およびApproved compositionによるpublish | build pipeline仕様および本CLI仕様 |
| `publish` | 既存artifact setのexternal配布 | build pipeline仕様 |
| `migrate` | schema-aware deterministic transformation | Schema Migration v1仕様 |

この表のcommand nameはcanonical surfaceである。ただし、実装済みであることを意味せず、
deprecation policy、argument grammar、output schemaをこの仕様で確定しない。
`build --publish`は`CLI-007`のcompositionであり、`publish`および`migrate`のsemantic ownerを
変更しない。

### CLI-003

`masterdata validate`は、canonical YAML sourceについて必要なproject/source loading、
parse、validation、semantic resolution、および既存のselection contractに従う検証を
行い、artifact生成を行わずに停止しなければならない（MUST）。既存artifactや生成済み
C#をvalidationのauthorityにしてはならず（MUST NOT）、validationを理由にcanonical
artifact set、external publish target、publish manifestを変更してはならない（MUST NOT）。

このrequirementは既存のschema、type、table、Build Selection、project仕様をCLI側へ
複製するものではない。

### CLI-004 — Historical

このRequirement IDは、仕様変更0011で定義されたpublic `masterdata generate` contractの
traceabilityを保持するためのhistorical tombstoneである。仕様変更0012の適用後、`CLI-004`
はcurrent canonical CLI behaviorを定義せず、別semanticへreassignまたは再利用してはならない
（MUST NOT）。0011でreviewされた、source-derived validation後にC# generationで停止する
Operationの履歴は参照可能な状態に留めるが、現行public command surfaceには含めない。

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

`CLI-011`は、source-derived staged operationである`validate`、`build`に
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

現在のCLI実装が提供するcommandは、`init`、`doctor`、`project-info`、`validate`、`build`、`publish`
である。したがって、canonical surfaceとの差分は次のとおりである。

- `project-info`は現行実装に存在するが、今回のtarget canonical public command setには
  含めない。
- このdocs-only canonicalizationでは`project-info`を削除、rename、別namespaceへ移動しない。
- `publish`は既存のreceipt-valid artifact setを`NativeApplicationService::publish`へ直接委譲するadapterとして実装済みである。
- `migrate`、`build --publish`は未実装であり、Implementation Gapとして扱う。
- `generate`はcurrent canonical targetではないため、Implementation Gapとして扱わない。
- `project-info`の将来のdiagnostics/info系surfaceは、この仕様では代替案を確定しない。

この差分はcanonical 6 command全体を実装済みと示すものではない。migrateと`build --publish`の
CLI wiringは引き続きImplementation Gapである。

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
- Generated C# Preview / explicit C# ExportのUX、destination、filename、将来CLI surface
- `build --publish`の詳細なconsole/result serialization

## Acceptance matrix（implementation / future evidence）

この文書はApprovedであり、以下のmatrixは実装済みevidenceとfuture planned evidenceを区別する。
未実施のtestをpass済みとは扱わない。

| Requirement | Planned evidence | Status |
| --- | --- | --- |
| CLI-001, CLI-010 | CLI/Tauri/Connected Webが同じOperation ownerを使い、CLI direct pathにRPCがないことを確認するarchitecture/integration evidence | pending implementation |
| CLI-002 | canonical command surfaceと現行実装gapのCLI acceptance test | pending implementation |
| CLI-003 | `validate`がartifact、publish target、manifestを変更しないtest | pending implementation |
| CLI-004 | historical traceabilityを保持し、current command surfaceまたは別semanticへ再利用しないことのdocumentation review | historical tombstone |
| CLI-005 | buildがcoherent canonical artifact setを生成し、単体ではexternal targetを変更しないtest | pending implementation |
| CLI-006 | `publish_uses_receipt_valid_artifacts_without_loading_current_yaml`; `publish_missing_receipt_fails_without_target_mutation`; `publish_preflight_failure_mutates_no_targets`; `publish_zero_targets_is_successful_noop`; `publish_report_preserves_per_target_status` | implemented |
| CLI-007 | build失敗時のpublish未開始、publish失敗時のbuild保持、partial failure集約のtest | pending implementation |
| CLI-008 | `publish_does_not_accept_unapproved_short_publish_flag` | implemented |
| CLI-009 | migrateがSchema Migration engineへ委譲され、CLIがsemantic logicを複製しないtest | pending implementation |
| CLI-011 | source-derived validate/buildが前段をskipせず、publishがsource validationを要求しないことのCLI pipeline test | pending implementation |

## Open Questions

### OQ-A: Generated C# preview / explicit export

将来のGenerated C# Previewまたはexplicit C# Exportを提供するか、そのdestination、filename、
cleanup、将来のCLI surfaceは未決定である。`masterdata export`を今回追加せず、full buildの
canonical artifact setをC#だけ部分更新する方式も採用しない。`.masterdata/generated/`を
recommended layoutとして導入しない。

### OQ-B: CLI result contract

`build --publish`におけるbuild successとpublish failure/partial failureの区別を、どの
structured result、console形式、exit codeで表現するかは未決定である。CLI-007のsemantic
result境界だけをこの仕様で定義し、global output/exit contractは別途決める。

### OQ-C: Public argument surface

`migrate`のargument grammar、Build Profile選択syntax、global machine-readable output、
versioning/deprecation policyは未決定である。

## Non-goals

この仕様は、CLI parser、Tauri command、Web UI、Native Host、migration engine、YAML
rewrite、receipt runtime、external publisher、`project-info` removal、Generated C# Preview / explicit
Export UXを実装または確定
しない。
