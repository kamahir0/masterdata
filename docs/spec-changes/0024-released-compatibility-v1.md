# 仕様変更: Released Compatibility v1

Status: Applied

## Affected Specifications

- [Compatibility仕様index](../specs/compatibility/README.md): released compatibility ownerを追加する。
- [Table identity](../specs/compatibility/table-identity.md): current project-local identityとreleased rename classificationを分離する。
- [Field identity history](../specs/compatibility/field-identity.md): retired Field IDを復活させず、current field evolution classificationへroutingする。
- [Enum identity](../specs/compatibility/enum-identity.md): external wire compatibilityとMasterdata generated API compatibilityを分離する。
- [Index identity](../specs/compatibility/index-identity.md): logical key shapeとbackend indexNoをreleased compatibilityで混同しない。
- [Table / Key](../specs/table-and-keys.md)、[Index / Reference](../specs/index-and-reference.md)、Type System owner specs: current-schema semanticsは変更せずcompatibility analyzerが参照する。
- [Build pipeline](../specs/build-pipeline.md): artifact-set receiptをreleased compatibility identityへ昇格させない。
- [Schema Migration](../specs/schema-migration.md)、[Type Migration](../specs/type-migration.md): migration successとreleased consumer compatibilityを分離したままimpact reportへ接続する。

## Source Evidence and Classification

- Human Decision: 2026-09-21 JST、Reference v1完了後の次priorityとしてreleased compatibility / schema evolution方向へ進む。
- Approved Constraint: current buildは `schema A -> generated C# A -> binary A` のcoherent artifact setを保証し、cross-schema-version C#/binary mixingを保証しない。
- Approved Constraint: artifact-set receiptはartifact integrity metadataであり、semantic schema hash、released compatibility identity、schema identityではない。
- Approved Constraint: MessagePack field `key`はserialization metadataでありlogical field identity / rename / migration identityではない。Secondary `indexNo`もbackend detailである。
- Approved Constraint: `table`はcurrent project-local logical identity、`csharpName`はpresentation concernである。global identity / released rename compatibilityは未定義。
- Approved Constraint: Schema / Type Migrationはcurrent source transformationのcorrectnessを保証するが、released external consumerのgenerated API compatibilityを保証しない。
- Existing Draft: Enum / Index compatibility documentsにはexternal contractとindex compatibilityのOpen Questionsが残る。

## Problem

現在の仕様はcurrent schemaを安全にresolve / migrate / buildするが、revision Aからrevision Bへ変更したときに、

- generated C# consumerがsource-compatibleか、
- canonical sourceのmigrationが必要か、
- artifact/binaryを跨いで互換と呼べるか、
- external save/network/databaseへ影響するか、

を一つのmeaningへ潰さず判定するreleased compatibility contractを持たない。

この区別なしに「compatible / breaking」を導入すると、例えばfield renameがcurrent source migrationとして安全でもgenerated C# property renameとしてbreaking、MessagePack key維持でもcross-schema binary互換を保証しない、という異なる観測を誤って一つのboolへまとめる危険がある。

## Common principles

どのOptionでも以下を維持する。

1. current-schema semantic ownerを互換性仕様が上書きしない。
2. path / filename / generated artifact / receiptをschema identityへ昇格させない。
3. MessagePack `key`をlogical Field IDとして扱わない。
4. Secondary `indexNo`をlogical identityとして扱わない。
5. Reference targetはexisting `table + ordered target fields` semanticsを使用し、generated C# helper名をrelationship identityへ使わない。
6. compatibility reportは根拠となるchangeとaxisをstructuredに示し、unknown changeをsilent compatibleにしない。
7. source mutation、Build、Publish、release/tag作成をcompatibility analysisへ暗黙結合しない。

## Option A — 推奨: Source snapshot comparison + multi-axis compatibility

explicitなbaseline project snapshotとcurrent project snapshotをcanonical parser / resolverで読み、semantic model同士を比較する。

baselineはcallerが明示的に提供したsnapshotであり、artifact receipt、last build、directory mtime、Git HEADを暗黙baselineにしない。Git refを利用するadapterを将来/同sliceで提供しても、core inputはmaterialized canonical source snapshotとする。

v1 reportは最低限、changeごとに次のaxisを分ける。

### Generated API compatibility

generated C# public surfaceのconsumerに対するimpact。

例:

- Table `csharpName`変更
- field/property rename
- field type / modifier変更
- generated constructor shape変更
- Enum member rename/remove
- Reference `csharpName`またはdefault effective helper name変更
- Secondary Key変更によるgenerated `FindBy...` API変更

を評価する。

### Masterdata source / migration impact

新schemaへ進むためcanonical YAMLにmigration/actionが必要か、existing approved migrationで表現可能か、unsupported/manualかを報告する。

これはMigration operationを自動実行する意味ではない。

### Artifact/binary axis

v1では「coherent rebuild required / cross-schema compatibility not guaranteed」を報告できるが、旧MasterMemory binaryと新generated C#の相互運用をcompatibleと判定するcontractは導入しない。

### External contract axis

save data / network / external DB等はMasterdata sourceだけから安全に判定できないため、v1では `not assessed` / external policy required とする。Enum numeric valueやMessagePack keyから外部互換を推測しない。

### Identity / baseline

- project identityはexisting `project.id`を使用する。
- `project.version`はreport metadataとして比較してもよいが、compatibility identityやsemantic-version enforcementにしない。
- new global Table ID / Field ID / Reference IDを要求しない。
- renamed entityを確実に対応付けられない場合はguessせずreview-required / unmatchedとして報告する。

このOptionは既存のYAML + Git workflowと整合し、compatibility analysisをcurrent semanticsの上に追加できる。

## Option B — Cross-schema MasterMemory binary compatibilityを含める

Option Aに加え、

- schema A generated code + binary B
- schema B generated code + binary A
- field key/type変更
- table/index shape変更
- MessagePack resolver/constructor behavior

などについてwire/runtime compatibility contractを定義する。

これにはMasterMemory / MessagePackのversioned behavior、generated code shape、binary layout、reader/writer matrixを正式なproduct contractとして扱う必要がある。

既存仕様はcoherent artifact setだけを保証しているため、これは新しい大きなcompatibility promiseである。

## Option C — Persistent release identity / manifest first

比較器より先に、

- release/schema manifest
- persistent schema revision identity
- stable Table / Field / Type / Reference IDs
- tombstone / rename lineage
- semver relation

等をcanonical sourceまたはtool metadataへ導入し、それをcompatibility analysisの基盤にする。

rename追跡精度は上げられるが、source formatとidentity modelへの影響が大きい。特にretired Field ID modelを別名で復活させない設計が必要。

## Representative changes to classify under Option A

Human decision後、少なくとも以下を分類表とfocused evidenceへ落とす。

- Table追加 / 削除 / `table`変更 / `csharpName`変更
- field追加 / rename / drop / type変更 / Nullable変更 / Array変更 / MessagePack key変更 / reorder
- Primary Key shape/order変更
- Secondary Key add/drop/reorder/nonUnique変更
- Reference add/drop/name変更/csharpName変更/source fields変更/target変更/nullability change
- Value Object underlying/conversion change
- Enum / Flags member add/rename/drop/numeric value change/underlying change
- Custom Type field add/rename/drop/type/modifier/key/reorder
- Build Profileやdata-only changeがschema compatibilityへ影響しないcase

## Refined Option A delta

### Canonical snapshot and comparison boundary

`CompatibilitySnapshot`はcallerが明示的にmaterializeした一つのProjectのsnapshotで、既存の
`project.id`、`project.name`、`project.version` metadataと、canonical YAMLをparseして得た
`ProjectDocuments`を含む。shared analyzerはsnapshotを受け取って比較し、filesystem discovery、
Git、artifact、receipt、Build、Publish、source mutationを行わない。

baseline/current双方のschema/type semantic structureをcanonical Type System、Table / Key、
Reference resolverで一度だけresolveする。schema/type構造を安全にresolveできないsnapshotは
compatibility resultではなくstructured input diagnosticでfail closedする。data recordはschema
identityとは別にcanonical typed source valueの集合として扱い、record order、mapping member order、
physical path、file split、YAML formatting/commentsを比較identityにしない。

`project.id`がbaseline/currentで一致しない場合はreportを生成せず、`E-COMPAT-PROJECT-MISMATCH`
によるinvalid comparison requestとする。`project.version`と`project.name`はreport metadataに
保持できるが、SemVer parsing、version bump、version値によるclassificationは行わない。

Tableは`table` value、Typeは既存Type declarationのlogical `name`でmatchする。同一Tableまたは
Custom Type内のfieldはcurrent field symbol (`name`)でmatchする。Referenceはdomain `name`で
matchし、`csharpName`はGenerated API presentationとして別に比較する。MessagePack `key`、
Secondary `indexNo`、path、filename、generated C# nameをlogical identityまたはrename lineageに
使用しない。確実な対応付けができないremove/addはrenameとして結合せず、unmatched remove/addと
してreportする。

### Finite axis classifications

各`CompatibilityChange`は次の型付きaxis resultを持つ。classificationを一つのoverall boolへ
縮退させない。summaryはchangesから再計算され、個別changeのsubject、baseline/current locator、
reason、Requirement IDを失ってはならない。

`GeneratedApiImpact`は`unchanged`、`additive`、`breaking`、`review_required`のいずれかである。
`SourceMigrationImpact`は`not_required`、`supported_operation`、
`destructive_authorization_required`、`manual_action_required`、`review_required`のいずれかである。
`ArtifactBinaryImpact`は`unchanged`、`rebuild_required`、
`cross_schema_interoperability_not_guaranteed`のいずれかである。
`ExternalContractImpact`は`not_assessed`または`external_policy_required`のいずれかである。
未分類、未知のchange、または不確実な対応付けを`unchanged` / `additive`へ黙って分類してはならない。

`supported_operation`はmigration operationの存在可能性を示すだけで、operationのPlan / Applyを
実行または成功保証するものではない。既存migrationのprecondition、Reference dependency、
destructive authorizationがある場合は対応する分類へ上げる。`review_required`はsemantic modelから
single-axis outcomeを安全に決定できない場合に使用し、理由を必ず返す。

`rebuild_required`はcurrent coherent artifact setの再Buildが必要なことだけを示す。schemaを跨いだ
MasterMemory binary / generated C# interoperabilityはv1で保証せず、schema shapeやserialized
representationに関わるchangeでは`cross_schema_interoperability_not_guaranteed`を併記する。
External axisはcanonical sourceだけではsave/network/database契約を検証できないため、sourceから
compatible/breakingを推測せず、必要に応じて`external_policy_required`を返す。

### Classification rules

- Table/type addはGenerated API `additive`、removeは`breaking`とする。
- Table `csharpName`、type shape、field type/modifier、Table field drop、Custom Type field change、
  Enum/Flags member removeまたはnumeric value change、Primary Key change、unique/non-uniqueを含む
  Secondary Key changeはGenerated API `breaking`とする。
- Table field add、Enum/Flags member add、Secondary Key addはactual generated surfaceへの追加として
  `additive`とする。Custom Type field addはpublic constructor shapeも変えるため`breaking`とする。
- field nameが不一致の場合はfield renameを推測せず、old field remove + new field addとする。
  Table/Type nameについても同じくremove + addである。
- MessagePack key change、field declaration reorder、Secondary declaration reorderはlogical identityを
  変更しない。actual generated public callable surfaceが変わらない場合Generated APIは`unchanged`、
  Artifactは`rebuild_required`とする。Custom Type constructor orderを変えるfield reorderは`breaking`とする。
- Primary / Secondary generated query name・signature、Reference helperのeffective C# identifier、
  return shapeが変わる場合はactual C# surfaceに基づき`breaking`とする。Reference target/source shapeの
  変更でcallable signatureだけから安全に決められない場合は`review_required`とする。
- Reference domain nameの不一致はremove/addであり、Reference `csharpName`だけの変更はrelationship
  identityを変えずGenerated API presentation changeとして個別reportする。Reference target、source
  fields、Required/Nullable、single/multiの変更はdomain/API behavior changeとして個別reportする。
- record valueの変更だけは`data_changed`として、Generated API `unchanged`、Source/Migration
  `not_required`、Artifact `rebuild_required`とする。schema/API breakingへ昇格しない。
- path/file split、comments、YAML formatting、record mapping member order、Build Profileだけの差は
  compatibility changeを生成しない。source contentのfilesystem identityをsemantic changeにしない。
- source/migration axisはexisting Schema Migration / Type Migration capabilityへ接続するが、
  compatibility operationはread-onlyである。Build、Publish、Git、version、receipt、source rewriteを
  implicitに開始してはならない。

### Structured report

`CompatibilityReport`はbaseline/current project metadata、ordered `CompatibilityChange` sequence、
axis別derived summaryを含む。各changeは少なくともsubject kind/owner/member、change kind、baseline
locator、current locator、4つのaxis result、reason、related Requirement IDをrecoverできる形でなければ
ならない。report orderingはlogical subject kind、owner/member、change kindのstable orderとし、
filesystem traversal / HashMap orderへ依存してはならない。

invalid input diagnosticとvalid snapshot間のbreaking change reportは別conceptとする。invalid inputで
empty compatible reportを返してはならない。

### Shared operation surface

shared `masterdata-app`へ、明示的なbaseline project pathとcurrent project pathを受け取るread-only
compatibility operationを追加する。CLIは既存noun command conventionに従う`masterdata compatibility`
をJSON/console adapterとして公開し、baseline/currentを必須argumentとする。Tauri Desktopは同じ
application operationへ両方のpathを渡す薄い`compatibility_report` commandを公開する。
frontendはentity matching、classification、summary計算を実装せず、shared reportを表示する。

これはOption Aのexplicit input boundaryに必要なadditive surfaceであり、既存CLI commandのmeaning、
source format、artifact receipt、publish、Git integrationを変更しない。

### Acceptance evidence

- unchanged、format/path-only、file split、data-only snapshotがschema/API changeを生成しない。
- same project.idのTable/field/type/key/Reference/Enum/Flags evolutionがaxis別にdeterministicに分類され、
  MessagePack key、indexNo、csharpName、pathをidentityへ昇格しない。
- field/table/typeのunmatched remove/add、project.id mismatch、invalid baseline/currentがそれぞれ
  rename推測またはcompatibleへの潰し込みなしでstructured outcomeになる。
- migration classificationがread-onlyで、YAML bytes、config、artifact、receipt、publish target、Git
  state、version metadataを変更しない。
- CLIとTauri commandがshared analyzer/application reportを利用し、frontend-only diffを持たない。

## Public surface

shared application operationへ明示的なbaseline/current project pathを渡す。CLIは
`masterdata compatibility --baseline PATH --current PATH [--json]`を公開し、Tauri Desktopは
`compatibility_report` commandを公開する。Git ref adapter、GUI placement、JSON reportの詳細は
implementation detailとし、shared analyzerを第二のCLI-specific domain実装へしない。

## Compatibility

本change自体はcurrent source schemaやgenerated artifact behaviorを変更せず、revision比較の新しいsemantic layerを追加する方向である。

Option Cを選ばない限り、新しいpersistent identity fieldをcanonical YAMLへ追加しない。

Option Bを選ばない限り、existing coherent artifact-set contractをcross-schema binary compatibility guaranteeへ拡張しない。

## Human decision

2026-09-21 JST、Human maintainerはOption Aを選択した。

Released Compatibility v1はexplicitなbaseline/current canonical project snapshotを比較するmulti-axis analyzerとする。

- generated C# API compatibilityとMasterdata source / migration impactを別axisで扱う。
- artifact/binary axisはcoherent rebuild requirementと「cross-schema compatibility not guaranteed」を表現するに留める。
- save data / network / external database等のexternal long-lived contractはv1で判定しない。
- artifact-set receipt、`project.version`、MessagePack `key`、Secondary `indexNo`をreleased compatibility identityへ昇格させない。
- new global Table / Field / Type / Enum / Reference stable ID、release manifest、rename lineageをv1の前提として導入しない。
- baseline/currentはcallerが明示的に与えるcanonical project snapshotとし、last build、receipt、mtime、Git HEADをimplicit baselineにしない。

Option B / CはRejected alternativeとする。

## Review

2026-09-21 JSTに`review-spec`相当のfresh reviewを完了した。Option Aのexplicit snapshot boundary、
4-axis finite vocabulary、same-project matching、no-rename inference、read-only operation、shared
Rust/application ownership、CLI/Tauri surfaceをcanonical ownerへ反映した。

- Blocking: None
- Human gate: None。Option AはHuman決定済みで、残るCLI/Tauri surfaceはCurrent Objective内のadditive adapterである。
- Open Questions: None
- Approval eligibility: satisfied。classificationはmachine-actionableで、unknown inputはstructured
  diagnosticへfail closedし、cross-schema binary / external contract / stable IDはscope外に留めた。

classification detailsに複数のmaterially different product choiceが残る場合だけ新しいHuman gateへ戻す。

## Approval Record

Option A Human decision: 2026-09-21 JST。
Option B / C: Rejected alternative。
Canonical application: [Released Compatibility v1仕様](../specs/compatibility/released-compatibility.md)へApplied。
Implementation: shared Rust core/application、CLI、Tauri command、focused testsで実施済み。Candidate作成後にremote CI reconciliationを行う。
