# Build Selection仕様（Record Tags / Build Profiles）

Status: Proposed

Domain: Build

## 概要

本proposalは、source recordに付与するRecord Tag、project configurationに定義するBuild Profile、およびprofileから
解決されるBuild Selectionを定義する。これらはbuild-timeのselection metadataであり、domain fieldやMasterMemory binaryの
dataではない。

このdocumentはtag、profile、selection formula、selection後のdataset validation順序、およびselectionがbinary semanticsへ
与える影響のcanonical ownerである。全体のpipelineは[Build pipeline仕様](build-pipeline.md)が所有し、Primary Key、Unique、
Referenceのsyntaxと個別のconstraintは、それぞれのowner specificationが所有する。

## 用語

`Record Tag` は、source recordに付与されるproject-localなbuild-time labelである。record metadata keyは `$tags` とする。
`Build Profile` は、`masterdata.toml` に保存される名前付きのinclude/exclude selectionである。`Build Selection` は、profile
またはunfiltered buildから解決されたselectorをrecordへ適用する処理である。

`selected logical dataset` は、Build Selection後にtableごとに構成され、dataset-level constraintとbinary buildの入力になる
record集合を指す。`unfiltered build` は、include setとexclude setがともに空の名前なしselectionである。

## 規範要件

### BUILD-SELECT-001

source recordは、0個以上のRecord Tagを持ってもよい（MAY）。Record Tagはrecord metadataとして `$tags` に保持しなければ
ならず（MUST）、domain fieldとして解釈してはならない（MUST NOT）。`$tags` をdomain fieldまたはrecordの通常のdata member
として宣言または使用する入力はrejectしなければならない（MUST）。Record Tagおよびprofile metadataはgenerated C# row/domain
fieldまたはMasterMemory binary dataへ含めてはならない（MUST NOT）。

### BUILD-SELECT-002

Record Tagはcase-sensitiveなASCII lowercase kebab-caseでなければならない（MUST）。tagのlexical grammarは次である。

```text
[a-z][a-z0-9]*(?:-[a-z0-9]+)*
```

`common`、`debug`、`development`、`event-summer`、`dlc-1`、`region-jp`はvalidなtagであり、`Debug`、`event_summer`、
`-event`、`event-`、日本語を含むtagはinvalidでなければならない（MUST）。

### BUILD-SELECT-003

`$tags` が存在する場合、そのvalueはYAML sequenceでなければならない（MUST）。block sequenceとflow sequenceの両方を
使用してもよい（MAY）が、scalar shorthandまたはnullを使用してはならない（MUST NOT）。`$tags` の省略は空のtag setと同じ
意味でなければならない（MUST）。

### BUILD-SELECT-004

1つのrecordにおけるtag membershipはsetでなければならず（MUST）、tagの順序はsemantic meaningを持ってはならない
（MUST NOT）。同じrecord内のduplicate tagはinvalidでなければならない（MUST）。tagは事前のregistryへの登録を必要とせず、
recordまたはBuild Profileで初めて現れてもよい（MAY）。tagはproject-local labelであり、file、directory、tableから導出しては
ならず（MUST NOT）、file-level、directory/path-derived、table-level tagまたはtag inheritanceを定義してはならない
（MUST NOT）。

### BUILD-SELECT-005

projectは、0個以上の名前付きBuild Profileを`masterdata.toml` の `[build.profiles.<name>]` 配下に定義してもよい
（MAY）。Profile nameはproject内でuniqueかつcase-sensitiveでなければならず（MUST）、Record Tagと同じASCII lowercase
kebab-case grammarに従わなければならない（MUST）。

### BUILD-SELECT-006

v1のBuild Profileが持つselection semanticsは `include_tags` と `exclude_tags` に限らなければならない（MUST）。profileは
output、compression、platform、per-table selector、inheritance、`extends`、またはprofile compositionの意味を定義してはならない
（MUST NOT）。v1で定義されていないprofile keyを受理、無視、またはerrorとする具体的な扱いは、このrequirementでは決定しない。

`include_tags` と `exclude_tags` はそれぞれsemantic setであり、entryの順序は意味を持ってはならない（MUST NOT）。同じ
collection内のduplicate entry、および同じprofile内でincludeとexcludeの両方に現れるtagはconfiguration errorでなければ
ならない（MUST）。各entryは`BUILD-SELECT-002`のRecord Tag grammarに従わなければならない（MUST）。現在どのsource recordにも
現れないtagをprofileが参照してもvalidでなければならない（MUST）。実装はその
状態をwarningとして通知してもよい（MAY）が、unusedであることだけを理由にfailしてはならない（MUST NOT）。

### BUILD-SELECT-007

Build ProfileのselectionはGUIだけのstateではなく、projectとshared application/core workflowのconceptでなければならない
（MUST）。profileの解決はshared application orchestrationが行い、CLIまたはGUIから同じprofileを選択したBuild Requestは、同じ
project profileから同じinclude/exclude selectionを解決してcoreへ渡さなければならない（MUST）。この責務分担は[CLIとGUIで共有する
application orchestrationのADR](../adr/0002-rust-core-shared-by-cli-and-gui.md)に従う。GUIは独自のselection semanticsを実装しては
ならず（MUST NOT）、GUIからCLI subprocessを呼び出してprofile selectionを実行してはならない（MUST NOT）。

### BUILD-SELECT-008

解決されたinclude set、exclude set、およびrecord.tagsに対して、recordは次の条件を満たす場合にだけselectedでなければ
ならない（MUST）。

```text
(include is empty OR record.tags intersects include)
AND
NOT(record.tags intersects exclude)
```

include matchingはOR、exclude matchingもORでなければならず（MUST）、exclude matchingはinclude matchingに優先しなければ
ならない（MUST）。

### BUILD-SELECT-009

named Build Profileの指定はoptionalでなければならない（MUST）。名前なしの `masterdata build` は、保存されたprofileを暗黙に
選ぶのではなく、includeとexcludeがともに空のunfiltered buildとして扱わなければならない（MUST）。`--profile production` の
ようなnamed profile指定は、指定されたproject profileを使わなければならない（MUST）。includeとexcludeがともに空のnamed
profileはvalidであり、unfiltered buildとselection-equivalentでなければならない（MUST）。`--include-tag` や
`--exclude-tag` のようなprofile外のad-hoc selectorをcanonical build behaviorとして追加してはならない（MUST NOT）。

### BUILD-SELECT-010

Build Selectionはdataset-level uniquenessおよびreference validationより前に実行しなければならない（MUST）。conceptualな処理
順序は次のとおりである。

```text
source parse
 -> profile-independent validation
 -> Build Selection
 -> selected logical Table construction
 -> Primary Key / Unique constraints
 -> Reference integrity
 -> canonical ordering
 -> binary build
```

ここでprofile-independent validationは、selectionに依存しないsource/profileの構造と入力の検証を指す。Primary Key、Unique、
Referenceの具体的なschema syntaxは、このrequirementでは定義しない。

### BUILD-SELECT-011

将来のPrimary KeyまたはUnique constraintで同じkeyになるsource recordは、異なるselectionによって分離される場合、source内で
共存してもよい（MAY）。Primary KeyおよびUnique constraintは、source全体ではなくselected logical datasetに対して評価しなければ
ならない（MUST）。したがって、production selectionとdebug selectionがそれぞれ高々1件の同一keyを含むことはvalidだが、両方を
選ぶselectionで同じkeyが複数件selectedになる場合はconstraint errorでなければならない（MUST）。

このrequirementは、[Index / reference仕様](index-and-reference.md)のPrimary Key、Unique、secondary indexのsyntaxを定義せず、
それらのdataset評価対象がselection後であることだけを定める。

### BUILD-SELECT-012

Reference integrityはselected logical datasetだけに対して検証しなければならない（MUST）。selected source rowからnon-selectedまたは
masked target rowへのReferenceはmissing-reference errorでなければならない（MUST）。non-selected source rowは、そのselectionの
Reference validationへ参加してはならない（MUST NOT）。successfully produced binaryは、selected datasetについてreferentially
integralでなければならない（MUST）。Referenceの宣言syntaxは[Index / reference仕様](index-and-reference.md)が所有する。

### BUILD-SELECT-013

Build Selectionの結果、logical tableが0件になってもvalidでなければならない（MUST）。その場合でもtableとschemaは存在し続け、
empty selected datasetとして扱わなければならない（MUST）。

### BUILD-SELECT-014

Build ProfileとBuild Selectionはrecord selectionだけに影響しなければならない（MUST）。profile selectionによってtable existence、
schema existence、generated C# API shape、またはtype declarationを変更してはならない（MUST NOT）。

### BUILD-SELECT-015

Record Tag、profile name、include/exclude metadataはMasterMemory binaryへserializeしてはならない（MUST NOT）。tagはselected logical
datasetを決める入力としてのみbinary buildへ影響してよい（MAY）。

### BUILD-SELECT-016

2つのbuildが同じfinal selected logical datasetを生成する場合、そのbinary semanticsは同一でなければならない（MUST）。次の差異は、
selected domain datasetが変わらない限り、binary semanticsを単独で変更してはならない（MUST NOT）。

- profile name
- YAML source file path
- 1つのtableのrecordを複数のYAML fileへ分割する方法
- source YAML record order
- `$tags` のentry order
- selected domain datasetを変えないirrelevantなtag変更

このfileのpathやdirectoryはtable identityを決めず、既存の[Table identity仕様](compatibility/table-identity.md)に従う。

### BUILD-SELECT-017

canonical record orderingはBuild Selectionの後に行わなければならない（MUST）。orderingの正確なmechanismは、将来のTable、
Primary Key、または関連するordering specificationが所有し、このdocumentは決定しない。cache reuseはimplementation detailであり、
安全のためにprofileまたはselectionごとにconservative rebuildを行ってもよい（MAY）。cache identityの最終形は
[Build pipeline仕様](build-pipeline.md)のOpen Questionに委譲する。

## 検証ルール

各source recordについて、`$tags` のreserved metadata、tag grammar、sequence shape、duplicate tagを検証する。各project profileに
ついて、name grammar、profile内のduplicate、include/exclude overlap、profile shapeを検証する。selection後にselected logical datasetを
構成し、そのdatasetへPrimary Key、Unique、Reference、canonical orderingの順で適用する。

Tag、profile、selectionは、同じrecord/table identityを異なるYAML fileに分けてもfile pathから意味を導出してはならない。
このvalidation ruleは、将来のPrimary Key、Index、Referenceの具体的なsyntaxまたはdiagnostic codeを定義しない。

## 互換性

Record TagとBuild Profileはbuild-time metadataであり、generated C# domain field、schema/type declaration、MasterMemory binary wire
identityに追加されない。profileの追加・変更は、そのprofileで選択されるdatasetとbuild resultに影響し得るが、profile name自体はbinary
identityではない。同じselected logical datasetを生成するprofileはbinary semantics上同一でなければならない。

同じ将来Primary Keyを持つrecordのsource共存を許可するため、source全体に対する従来型のuniqueness validationとは互換でない可能性が
ある。このdocumentはPrimary KeyやReferenceのsyntaxを変更せず、selected datasetへの適用順序だけを定義する。既存のApprovedな
Table identity、Field identity、Type System contractは変更しない。released schema migration、cache key、binary versioningはこの
proposalのscope外である。

## 受け入れ証拠

| Requirement | Success observation | Failure observation |
| --- | --- | --- |
| `BUILD-SELECT-001` | `$tags` がmetadataとして保持され、生成row/binaryに現れず、domain member `$tags` はrejectされる。 | `$tags` が通常fieldまたはbinary dataとして扱われる。 |
| `BUILD-SELECT-002` | `common`、`event-summer`、`region-jp` が受理される。 | `Debug`、`event_summer`、`event-`、非ASCII tagがrejectされる。 |
| `BUILD-SELECT-003` | block/flow sequenceが受理され、省略がempty setになる。 | scalar、null、またはsequence以外のshorthandが受理される。 |
| `BUILD-SELECT-004` | tag orderを変更してもselectionが変わらず、registryなしの新tagが受理される。 | duplicate tag、path/table由来tag、inheritanceが有効になる。 |
| `BUILD-SELECT-005` | `production` と `debug` を同一project内で定義でき、profile nameのcaseとgrammarが検証される。 | 同名profile、invalid name、case-insensitiveな同一視が受理される。 |
| `BUILD-SELECT-006` | include/excludeのset semantics、overlap error、unused tagのvalidityが観測できる。 | duplicate、overlap、未使用tagだけを理由とするfailure、またはv1外のprofile propertyへselection semanticsが付与される。 |
| `BUILD-SELECT-007` | CLIとGUIの同じprofile選択が同じresolved selectionになる。 | GUIが独自selectionを持つ、またはCLI subprocessへdomain selectionを委譲する。 |
| `BUILD-SELECT-008` | includeのOR、excludeのOR、exclude優先、untagged recordの条件がformulaどおりになる。 | formulaと異なるrecordがselectedになる。 |
| `BUILD-SELECT-009` | unfiltered build、empty named profile、named profileがそれぞれ定義どおりに解決される。 | unnamed buildが保存profileを暗黙に選ぶ、またはad-hoc tag selectorがcanonicalになる。 |
| `BUILD-SELECT-010` | source/profile validation、selection、selected table、PK/Unique、Reference、ordering、binaryの順序が確認できる。 | selection前にdataset uniquenessまたはReference validationが実行される。 |
| `BUILD-SELECT-011` | selectionで分離された同一future keyのrecordが各profileで個別にvalidになり、同時selected時だけduplicateになる。 | source全体だけを対象にduplicateをrejectする、またはselected duplicateを許可する。 |
| `BUILD-SELECT-012` | selected sourceからmasked targetへのReferenceがmissing errorになり、masked sourceは検証対象外になる。 | non-selected rowがReference validationへ混入する。 |
| `BUILD-SELECT-013` | zero-row selected tableがtable/schemaを保持したままvalidになる。 | zero-row selectionがtable/schemaを消す、または必ずfailureになる。 |
| `BUILD-SELECT-014` | profile変更でrecord selectionだけが変わり、table/schema/API/type declarationは変わらない。 | profileがschema/API/table existenceを変更する。 |
| `BUILD-SELECT-015` | 同じselected datasetのbinaryにtag/profile metadataが含まれない。 | profile nameまたはtagがbinary dataへserializeされる。 |
| `BUILD-SELECT-016` | path、file split、record order、tag order、profile nameを変えても同じdatasetのbinary semanticsが一致する。 | irrelevant metadataだけでbinary semanticsが変わる。 |
| `BUILD-SELECT-017` | selection後にcanonical orderingが行われ、cache implementationがselection順序を壊さない。 | source orderをそのままbinary semanticsとして扱う、またはselection前にorderingする。 |

## 例

次はnon-normativeな例である。

```yaml
kind: data
table: item
records:
  - itemId: 1001
    name: DebugSword
    $tags: [development, debug]
```

```toml
[build.profiles.production]
include_tags = []
exclude_tags = ["debug", "development"]
```

同じfuture Primary Keyを持つrecordをselectionで分離する例:

```yaml
kind: data
table: item
records:
  - itemId: 1001
    name: Sword
    $tags: [production]
  - itemId: 1001
    name: Debug Sword
    $tags: [debug]
```

`$tags: debug` のようなscalar shorthand、`$tags: null`、または同一recordの`[debug, debug]`はinvalidである。

GUIが提案候補として利用できる概念上のKnownTagsは次のunionである。これはregistryではなく、discovery/autocompleteのための
non-normativeなviewである。

```text
KnownTags =
  tags appearing on records
  ∪ profile include tags
  ∪ profile exclude tags
```

## 未解決事項（Open Questions）

- 指定されたprofile nameが存在しない場合のfailure、fallback、diagnostic codeとseverityは何か。
- v1で定義されていないprofile keyをunknown configurationとしてreject、ignore、または別の扱いにするか。
- named profileで `include_tags` または `exclude_tags` の一方を省略した場合に、欠けたcollectionをempty setとして解決するか。
- unused tagへのwarningを表示する場合、そのchannel、severity、source locationをどうするか。
- CLIの `--profile` とGUIのprofile selectionをBuildRequestへ表現する正確なAPI/DTOは何か。
- tag/profile validation failureおよびselection後のconstraint failureへ、どのDiagnostic Codeとlocationを割り当てるか。
- Primary Key、Unique、Index、Referenceの具体的syntaxと、selected logical datasetに対する詳細なconstraint classificationは何か。
- canonical record orderingの正確なmechanismと、同じselected datasetの同一性をcache keyへどう表現するか。
- GUIでprofileやtagを編集・保存する際に、comment、formatting、quoteをどこまで保持するか。
- profileまたはtag変更をreleased schema compatibilityでどのように分類するか。

## 非目標

このproposalは、tag registry、file/directory/table tag、tag inheritance、profile inheritance、`extends`、profile composition、
per-table selector、output/compression/platform configuration、Primary Key/Index/Referenceの宣言syntax、binaryへのmetadata
serialization、cache keyの最終形式、またはGUI editorを実装・確定しない。
