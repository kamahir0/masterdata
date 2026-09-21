# 仕様変更: Released Compatibility v1

Status: Draft

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

## Public surface

exact CLI command名、GUI placement、Git ref adapter、JSON report schemaはcompatibility semantics確定後にrefineする。shared analyzerを第二のCLI-specific domain実装へしない。

## Compatibility

本change自体はcurrent source schemaやgenerated artifact behaviorを変更せず、revision比較の新しいsemantic layerを追加する方向である。

Option Cを選ばない限り、新しいpersistent identity fieldをcanonical YAMLへ追加しない。

Option Bを選ばない限り、existing coherent artifact-set contractをcross-schema binary compatibility guaranteeへ拡張しない。

## Human decision required

1. Option A / B / CのどれをReleased Compatibility v1のboundaryとするか。

推奨はOption A。

Option Aを選ぶ場合、baselineはexplicit old/new canonical project snapshotとし、receiptや`project.version`をimplicit compatibility identityへしない方向まで合わせて承認することを推奨する。

## Review

Pending Human decision. current authorityから、identity非昇格、receipt非identity、coherent artifact set、migration/released compatibility分離は固定できる。一方、cross-schema binaryまでproduct guaranteeに含めるか、persistent release identityを先に導入するかはmaterial product forkでありHuman gate。

## Approval Record

Pending.
