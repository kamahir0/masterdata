# Projectの構成と探索（Project layout and discovery）

Status: Approved

## 規範ルール

### PROJECT-001

Projectは `masterdata.toml` という名前のfileによって識別されなければならない（MUST）。

### PROJECT-002

明示的なproject pathは暗黙のdiscoveryより優先されなければならない（MUST）。pathには
project directoryまたはconfig file自体を指定してもよい（MAY）。

### PROJECT-003

明示的なpathがない場合、discoveryはcurrent directoryと各parent directoryをfilesystem rootまで
検査しなければならない（MUST）。

### PROJECT-004

markerが見つからない場合、operationはstructured project-not-found diagnosticを返さなければ
ならない（MUST）。

### PROJECT-005

Unityの `Assets/` と `ProjectSettings/` は `init` のhintとして使用してもよい（MAY）が、project
identityを決めるmechanismにしてはならない（MUST NOT）。

### PROJECT-006

Source rootはscan boundaryに過ぎない。source fileのdirectoryが、そのfileのtable、type、または
index semanticsを決めてはならない（MUST NOT）。

## 設定の形

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

`project.id`、`project.name`、`project.version`、および少なくとも1つのsource rootが必要である。canonical artifactとexternal publish targetの
normative semanticsは[Build pipeline仕様](build-pipeline.md)の`BUILD-ARTIFACT-*`および`PUBLISH-*`が所有する。このdocumentはproject marker、
project metadata、およびsource rootのconfiguration boundaryを所有する。

current implementationはcanonicalな`build.artifact_dir`、`build.cache`、およびoptionalな`publish.targets`を読み込み、legacyの
`build.output`と`build.binary_output`を[仕様変更0005](../spec-changes/0005-legacy-build-path-hard-cut.md)のhard cutに従ってstructured migration
diagnosticで拒否する。legacy configuration rejectionはbuild開始前に行われ、既存artifactを変更しない。external publish operation自体は未実装である。

`init`が生成するminimum configurationにはpublish targetを含めなくてもよい。例えば次のconfigurationだけでcanonical buildを開始できる。

```toml
[project]
id = "my-game-master-data"
name = "My Game Master Data"
version = "0.1.0"

[sources]
roots = ["sources"]

[build]
artifact_dir = ".masterdata/output"
cache = ".masterdata/cache"
```

以下の詳細なconfigurationとpath ruleによって、これらのruleは独立してtraceできる。
path APIに関するimplementation noteはnon-normativeであり、callerは特定のshell separatorを
前提にせずplatformのpath valueを使用するべきである。

### PROJECT-CONFIG-001

`project.id`、`project.name`、`project.version` は、それぞれwhitespace以外のvalueを少なくとも
1文字含まなければならない（MUST）。

### PROJECT-CONFIG-002

`sources.roots` は少なくとも1つのsource rootを含まなければならない（MUST）。

### PROJECT-CONFIG-003

設定されたsource rootは空文字列であってはならない（MUST NOT）。`build.artifact_dir`と`build.cache`
は空であってはならず（MUST）。`publish.targets`の存在、kind、path、およびtarget ownershipのshapeは、
[Build pipeline仕様](build-pipeline.md)の`PUBLISH-001`、`PUBLISH-002`、および`PUBLISH-009`が所有する。

### PROJECT-CONFIG-004

configurationにlegacy `build.output`が存在する場合、canonical configuration implementationはそのfieldを受理してはならず（MUST NOT）、
`E-CONFIG-LEGACY-BUILD-OUTPUT`を含むstructured migration diagnosticを返さなければならない（MUST）。configurationにlegacy
`build.binary_output`が存在する場合も同様に受理してはならず、`E-CONFIG-LEGACY-BINARY-OUTPUT`を含むstructured migration diagnosticを
返さなければならない（MUST）。diagnosticはlegacy field名と、`build.output`については`build.artifact_dir`または`kind = "csharp"` publish targetを、
`build.binary_output`についてはcanonical `.masterdata/output/masterdata.bytes`または`kind = "binary"` publish targetを、ユーザーが明示的に選ぶ
migration guidanceとして示さなければならない（MUST）。これらはgeneric unknown-key errorへ置き換えてはならない。両方が存在する場合は、両方をstable orderでcollectしてもよい（MAY）。
first-error modelでは`build.output`、`build.binary_output`の順に診断しなければならない（MUST）。

### PROJECT-CONFIG-005

legacy configurationのrejectionはbuild/publish operationを開始してはならず（MUST）、旧pathを`build.artifact_dir`または`publish.targets`へ自動変換してはならない
（MUST NOT）。rejection時にcanonical output、legacy output、binary、external publish destination、またはそれらのparentをmove、delete、rename、writeしてはならない
（MUST NOT）。

### PROJECT-CONFIG-006

`init`はcanonical configurationとして`build.artifact_dir = ".masterdata/output"`と`build.cache = ".masterdata/cache"`を生成しなければならず（MUST）、
legacy `build.output`または`build.binary_output`を生成してはならない（MUST NOT）。`publish.targets`は0..Nであるため、初期configurationで生成しなくてもよい（MAY）。
publish targetがない初期projectでも、canonical buildがproject-local artifactを作成できるconfigurationを生成しなければならない（MUST）。

### PROJECT-PATH-001

relativeなsource pathとcanonical build artifact pathはproject rootを基準にresolveしなければならない（MUST）。canonical artifact pathは
project directory外へescapeしてはならず（MUST NOT）、absolute pathをcanonical artifact rootとして扱ってはならない（MUST NOT）。relativeな
publish target pathのbaseは、[Build pipeline仕様](build-pipeline.md)の`PUBLISH-002`に従いproject rootとする。absolute publish target pathを
許可するかは同仕様のOpen Questionであり、このrequirementでは確定しない。

Open Questions: configがnamed source group、ignore pattern、明示的なUnity project linkを将来
サポートするか、および設定されたsource rootがsymlinkをfollowするか。current implementationは
cycle-safetyのinternal guardとしてsymlink entryをfollowしない。これはproduct-levelのpermission
または禁止ではない。

## 受け入れmatrix

このmatrixは上記requirementに対するnon-normativeなimplementation evidenceまたは将来のacceptance planである。test numberが
requirement definitionに見えないよう、canonical ruleの隣に置く。`implement-spec` は同じ
observable behaviorを、このmatrixによって確認する。Approved configuration contractへ変更された
`PROJECT-CONFIG-003`から`PROJECT-CONFIG-006`および`PROJECT-PATH-001`は、canonical configuration implementationとそのtest evidenceで確認する。

| Requirement（要件ID） | Observable behavior（観測可能な挙動） | Implementation owner（実装owner） | Success case（成功例） | Failure case（失敗例） | Test（テスト） | Fixture |
| --- | --- | --- | --- | --- | --- | --- |
| PROJECT-001 | `masterdata.toml` という名前のmarkerがprojectを識別する。 | `masterdata-core::Project` | 明示的なdirectoryがそのmarkerを解決する。 | markerのないdirectoryはprojectとして受け入れられない。 | `project_001_explicit_directory_uses_masterdata_marker` | `fixtures/minimal` |
| PROJECT-002 | 明示的なdirectory/file pathがparent discoveryより優先される。 | `masterdata-core::Project::discover` | 内側の明示的なprojectが選択される。 | explicit pathがvalidな場合、parent projectは選択されない。 | `project_002_explicit_path_has_priority_over_parent_search` | Temporary project |
| PROJECT-003 | searchはcurrent directoryから始まり、parentをたどる。 | `masterdata-core::find_config_upwards` | nested directoryから最も近いancestor markerが見つかる。 | searchはfilesystem rootで停止する。 | `project_003_discovers_config_from_parent_directory` | Temporary project |
| PROJECT-004 | markerがないことがstructured diagnostic dataとして返る。 | `masterdata-core::Project::discover` | errorがdiagnostic code/kindと、任意のstructured contextを持つ。 | conditionを特定するためにstring-only errorへ依存する必要がない。 | `project_004_returns_structured_not_found_diagnostic` | Temporary directory |
| PROJECT-005 | Unity folderはidentityを確立しない。 | `masterdata-core::Project::discover` | `Assets/` と `ProjectSettings/` だけではprojectをresolveしない。 | `masterdata.toml` がない場合はnot foundのままである。 | `project_005_unity_folders_do_not_define_identity` | Temporary directory |
| PROJECT-006 | 宣言されたYAML `kind` と `table` がdocument semanticsを決める。 | `masterdata-core::Project::load_documents` | 1つのroot内にある複数fileが、宣言したtableを保持する。 | file pathまたはdirectory nameでdocumentを別の意味に変更できない。 | `project_006_source_directory_does_not_define_table_identity` | `fixtures/minimal` |
| PROJECT-CONFIG-001 | project metadata fieldがwhitespace以外のvalueを含む。 | `masterdata-core::ProjectConfig::validate` | metadataが揃ったblockを受け入れる。 | 空の `id`、`name`、`version` はstructured config diagnosticを返す。 | `project_config_001_requires_non_empty_metadata` | Temporary project |
| PROJECT-CONFIG-002 | 少なくとも1つのsource rootが設定されている。 | `masterdata-core::ProjectConfig::validate` | source rootのあるprojectを受け入れる。 | 空の `sources.roots` listはstructured config diagnosticを返す。 | `project_config_002_requires_a_source_root` | Temporary project |
| PROJECT-CONFIG-003 | 設定されたsource root、canonical artifact path、cache pathが空でない。 | `masterdata-core::ProjectConfig::validate` | 空でないpathを受け入れる。 | 空のsource、`artifact_dir`、またはcache pathはstructured config diagnosticを返す。 | `project_config_003_rejects_empty_source_or_build_paths` | Temporary project |
| PROJECT-CONFIG-004 | legacy `build.output` / `build.binary_output`はmigration-aware structured diagnosticで拒否され、generic unknown-key errorへ潰れない。 | `masterdata-core::Project::from_config_path` | 新configurationを受け入れる。 | 各legacy fieldを対応するdiagnostic codeで拒否する。 | `project_config_004_rejects_legacy_build_paths_with_migration_diagnostics`; `project_config_004_reports_legacy_output_before_binary_output` | Temporary project |
| PROJECT-CONFIG-005 | legacy rejectionはbuild/publishとfilesystem mutationを開始せず、自動変換もしない。 | `masterdata-core::Project::from_config_path` | 旧artifactを残したままoperation開始を拒否する。 | rejection後にcanonical/legacy/external artifactが変更されない。 | `project_config_004_rejects_legacy_build_paths_with_migration_diagnostics`（config boundary） | Temporary project |
| PROJECT-CONFIG-006 | `init`はcanonical build configだけを生成し、publish targetを任意で省略できる。 | `masterdata-core::initialize_project` | `artifact_dir`と`cache`を持つconfigが生成される。 | legacy keysが生成される、またはpublish targetが必須になる。 | `init_creates_a_project_marker_and_source_root` | Temporary project |
| PROJECT-PATH-001 | relative source/canonical artifact pathはproject rootを基準にresolveされ、canonical artifactはproject-localに留まる。relative publish targetのbaseもproject rootである。 | `masterdata-core::Project::info` / `masterdata-core::Project::from_config_path` | project rootからcanonical/publish pathを解決する。 | canonical artifactがproject外へescapeする、またはprocess working directoryを基準に解決される。 | `project_path_001_resolves_relative_paths_against_project_root`; `project_path_001_rejects_unsafe_canonical_artifact_paths`; `project_path_001_rejects_artifact_source_and_cache_overlap` | Temporary project |

すべてのrowは、path separatorがplatformによって異なっても有効でなければならない。
symlink traversal safetyはsource-discovery documentationに Open Question として記録された
internal implementation guardであり、追加のproject identity ruleではない。
