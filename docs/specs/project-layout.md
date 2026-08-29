# Projectの構成と探索（Project layout and discovery）

Status: Implemented

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
output = ".masterdata/generated"
cache = ".masterdata/cache"
```

`project.id`、`project.name`、`project.version`、および少なくとも1つのsource rootが必要である。
以下の詳細なconfigurationとpath ruleによって、これらのruleは独立してtraceできる。
path APIに関するimplementation noteはnon-normativeであり、callerは特定のshell separatorを
前提にせずplatformのpath valueを使用するべきである。

### PROJECT-CONFIG-001

`project.id`、`project.name`、`project.version` は、それぞれwhitespace以外のvalueを少なくとも
1文字含まなければならない（MUST）。

### PROJECT-CONFIG-002

`sources.roots` は少なくとも1つのsource rootを含まなければならない（MUST）。

### PROJECT-CONFIG-003

設定されたsource rootは空文字列であってはならない（MUST NOT）。`build.output` と `build.cache`
は空であってはならず（MUST）、任意の `build.binary_output` が存在する場合も空であっては
ならない（MUST）。

### PROJECT-PATH-001

relativeなsource pathとbuild pathはproject rootを基準にresolveしなければならない（MUST）。
absolute pathはabsoluteのまま扱う。

Open Questions: configがnamed source group、ignore pattern、明示的なUnity project linkを将来
サポートするか、および設定されたsource rootがsymlinkをfollowするか。current implementationは
cycle-safetyのinternal guardとしてsymlink entryをfollowしない。これはproduct-levelのpermission
または禁止ではない。

## 受け入れmatrix

このmatrixは上記requirementに対するnon-normativeなimplementation evidenceである。test numberが
requirement definitionに見えないよう、canonical ruleの隣に置く。`implement-spec` は同じ
observable behaviorを、このmatrixによって確認する。

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
| PROJECT-CONFIG-003 | 設定されたsource pathとbuild pathが空でない。 | `masterdata-core::ProjectConfig::validate` | 空でないpathを受け入れる。 | 空のsource、output、cache、または任意のbinary pathはstructured config diagnosticを返す。 | `project_config_003_rejects_empty_source_or_build_paths` | Temporary project |
| PROJECT-PATH-001 | relative pathはproject rootを基準にresolveし、absolute pathはabsoluteのまま扱う。 | `masterdata-core::Project::info` | project infoがresolve済みsource/build pathを公開する。 | relative pathがprocess working directoryを基準にresolveされない。 | `project_path_001_resolves_relative_paths_against_project_root` | Temporary project |

すべてのrowは、path separatorがplatformによって異なっても有効でなければならない。
symlink traversal safetyはsource-discovery documentationに Open Question として記録された
internal implementation guardであり、追加のproject identity ruleではない。
