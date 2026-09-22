# Released Compatibility v1仕様

Status: Implemented

Domain: Released Compatibility

## 位置付け

この仕様は、同一Projectの2つの明示的なcanonical source snapshotを比較し、schema evolutionの
impactをGenerated API、Source / Migration、Artifact / Binary、External Contractの4 axisへ分離して
reportするcontractを所有する。current schemaの意味は既存のTable / Key、Type System、Reference、
Build、Migration仕様が所有し、この仕様はそれらをreleased comparisonへlowerする。
authoring-onlyの[Computed View](../computed-view.md)はruntime Table/API/binaryへlowerされないため、View定義の
追加・編集・削除をGenerated APIまたはArtifact / Binaryのbreaking changeとして推測しない。Viewのexpression、
Overview、source-preserving migration semanticsはComputed View仕様が所有する。

仕様変更0024でHuman maintainerが2026-09-21 JSTにOption Aを採用した。cross-schema MasterMemory
binary guarantee、external wire compatibility engine、persistent stable member identityはこのv1の
scope外である。

## 用語

- **Compatibility snapshot**: callerが明示的にmaterializeしたProject metadataとcanonical YAML
  source documentsの組。artifact、receipt、Git ref、mtime、branch、version値から暗黙に選択しない。
- **Baseline**: 比較の左辺としてcallerが明示したsnapshot。
- **Current**: 比較の右辺としてcallerが明示したsnapshot。
- **Semantic comparison**: canonical parser / Type System / Table / Key / Reference resolverから構成
  したmodel同士の比較。raw YAML textやphysical pathのdiffではない。

## 規範要件

### COMPAT-RELEASED-001 — Explicit snapshot input

Compatibility analyzerは、baselineとcurrentの両方をcallerが明示的に提供したmaterialized canonical
snapshotとして受け取らなければならない（MUST）。last successful Build、artifact-set receipt、
artifact timestamp、directory mtime、Git HEAD、branch、`project.version`をimplicit baseline selector
またはschema identityとして使用してはならない（MUST NOT）。Analyzer本体はfilesystem discovery、
Git、Build、Publish、source mutation、version bump、receipt generationを行ってはならない（MUST NOT）。

### COMPAT-RELEASED-002 — Project matching and input failure

baseline/current `project.id`が完全一致しない場合、operationはcompatibility change reportを返さず、
structured `E-COMPAT-PROJECT-MISMATCH` input diagnosticで拒否しなければならない（MUST）。`project.name`
と`project.version`はreport metadataへ保持してもよい（MAY）が、SemVer parser、version bump、version
値によるclassification、same-version compatibleまたはmajor-version許可の推測へ使用してはならない
（MUST NOT）。baseline/currentのparseまたはcomparisonに必要なsemantic structureを安全にresolve
できない場合も、breaking changeではなくstructured input diagnosticでfail closedしなければならない
（MUST）。

### COMPAT-RELEASED-003 — Semantic closure

両snapshotはcanonical Type System、Table / Primary / Secondary Key、およびReference resolverでresolve
しなければならない（MUST）。schema/type structureのcomparisonにBuild Selection後のrecord constraint、
canonical binary、generated source textをauthorityとして使用してはならない（MUST NOT）。data recordの
差はschema/API changeと分離した`data_changed`として必要な場合だけreportし、record mapping member
order、record order、source file split、comments、YAML formatting、physical pathをsemantic identityに
してはならない（MUST NOT）。

### COMPAT-RELEASED-004 — Logical matching and no rename inference

同一project.id内のTableは`table` value、Typeはexisting Type declaration `name`でmatchしなければならない
（MUST）。matched TableまたはCustom Type内のfieldはcurrent field symbol `name`でmatchしなければならない
（MUST）。Referenceはdomain `name`でmatchし、Reference `csharpName`はGenerated API presentationとして
別に比較しなければならない（MUST）。MessagePack field `key`、Secondary backend `indexNo`、file path、
filename、generated C# name、project directory basenameをlogical identityまたはrename lineageへ使用しては
ならない（MUST NOT）。確実な対応付けがないremove/addをrenameとして結合または推測してはならず（MUST
NOT）、unmatched remove/addとして個別にreportしなければならない（MUST）。

### COMPAT-RELEASED-005 — Ordered key and Reference semantics

Primary KeyおよびSecondary Keyのcomparison identityは既存のordered field-symbol sequenceを使用しなければ
ならない（MUST）。Secondary declaration reorderによる`indexNo`変更だけでlogical key remove/addとしては
ならない（MUST NOT）。Reference relationship identityは既存の`target.table + ordered target.fields`と
source field sequence、domain name semanticsを使用し、target indexNo、MessagePack key、generated helper
nameを使用してはならない（MUST NOT）。Reference `csharpName`変更はrelationship target changeではなく
Generated API presentation changeとして扱わなければならない（MUST）。

### COMPAT-RELEASED-006 — Axis model

各semantic changeは、単一overall booleanへ縮退せず、次の4 typed axis classificationを持つ
`CompatibilityChange`としてreportしなければならない（MUST）。

| Axis | finite classification |
| --- | --- |
| Generated API | `unchanged`, `additive`, `breaking`, `review_required` |
| Source / Migration | `not_required`, `supported_operation`, `destructive_authorization_required`, `manual_action_required`, `review_required` |
| Artifact / Binary | `unchanged`, `rebuild_required`, `cross_schema_interoperability_not_guaranteed` |
| External Contract | `not_assessed`, `external_policy_required` |

unknown、unmatched、またはsemantic modelだけでは安全に分類できないchangeをcompatible/unchangedへ
潰してはならない（MUST NOT）。`supported_operation`はexisting migration operationの表現可能性を
示すだけで、Plan / Applyの実行または成功保証ではない。`review_required`はreasonを伴わなければ
ならない（MUST）。

`rebuild_required`はcanonical source/dataまたはgenerated presentationが変化し、current coherent
artifact setを作り直す必要があるが、比較対象のserialized schema contract自体は変化しない場合に
使用する。serialized field shape、type representation、key shape、Reference contract、または
MasterMemory query loweringが変化する場合は`cross_schema_interoperability_not_guaranteed`を使用する。
この値は旧binaryとcurrent generated C#の組合せを安全と認める意味を持たず、current coherent artifact
setの再buildが必要であることを含む。

### COMPAT-RELEASED-007 — Generated API classification

Generated API axisは、current codegen naming / Type System / Table / Key / Reference loweringから導出した
generated C# public surface consumerへのimpactを表さなければならない（MUST）。少なくともTable type、
Table property、Custom Type constructor/property、Value Object API、Enum / Flags member、Primary / Secondary
query、Reference helper identifier / return shapeを対象にする。existing public type/memberのremove、
rename、parameter/return/type/modifier/nullability/callable signature changeは`breaking`候補として扱い、
actual surfaceに応じて分類しなければならない（MUST）。Table field、Enum/Flags member、Secondary queryの
additionはactual generated surfaceがadditiveである場合`additive`としてよい（MAY）。field nameの不一致は
rename inferenceなしにremove/addへ分類する。C# identifier、property、query、Reference helperの導出は
既存codegen / Table / Type System ownerのshared helperを使用し、compatibility moduleやfrontendへ別の
naming tableを作ってはならない（MUST NOT）。

Custom Type fieldのadd/remove/type/modifier changeはgenerated propertyまたはconstructor parameter
surfaceを変えるため`breaking`とし、declaration reorderもconstructor order changeとして`breaking`とする。
Value Object underlying change、Enum / Flags underlying change、またはEnum / Flags memberのremove/value
changeは`breaking`とする。Value Object conversionのpublic implicit surfaceは、implicit conversionの追加を
`additive`、既存conversionの削除を`breaking`として分類してよいが（MAY）、いずれもactual generated
surfaceをreasonへ示さなければならない（MUST）。

### COMPAT-RELEASED-008 — Source / Migration classification

Source / Migration axisはbaseline sourceからcurrent source/schemaへ進むauthoring impactを表し、existing
Schema Migration / Type Migration operationとpreconditionを参照しなければならない（MUST）。Rename/Dropが
Reference dependency等でfail closedする場合、operation名の存在だけを理由に`supported_operation`と
してはならず（MUST NOT）、destructive authorization/manual action/reviewを報告しなければならない。
Compatibility analysisはread-onlyで、migration Plan / Apply、YAML rewrite、Build、Publish、Git commit/tag、
artifact/receipt writeを暗黙実行してはならない（MUST NOT）。

### COMPAT-RELEASED-009 — Artifact / Binary boundary

schema/data/build inputのsemantic changeは、current coherent artifact setの`rebuild_required`として
reportしてよい（MAY）。schema shapeまたはserialized representationに関係するchangeは
`cross_schema_interoperability_not_guaranteed`を表現しなければならない（MUST）。旧MasterMemory binaryと
新generated C#、または異なるschemaのC#とbinaryの組合せをcompatibleと保証してはならない（MUST NOT）。
MasterMemory / MessagePack binary internalsをcomparison analyzerで解析・再実装してはならない（MUST NOT）。

### COMPAT-RELEASED-010 — External Contract boundary

save data、network protocol、external database、external API等のlong-lived contractはcanonical
Masterdata snapshotだけからcompatible/breakingと推測してはならない（MUST NOT）。External axisは少なくとも
`not_assessed`を返し、Enum numeric value、MessagePack key、型変更等が外部policy確認を要する場合は
`external_policy_required`としてreportしてよい（MAY）。v1はexternal contract analyzer、wire identity、
numeric value reservation、save migrationを導入しない。

### COMPAT-RELEASED-011 — Structured report and evidence

`CompatibilityReport`はbaseline/current project metadata、deterministically ordered `changes`、および
change sequenceからderivedされたaxis summaryを含まなければならない（MUST）。各changeは少なくともsubject
kind/owner/member、change kind、baseline/current locator、4 axis result、reason、related Requirement IDを
recoverできなければならない（MUST）。summaryは根拠changeへ辿れる情報を失ってはならず、filesystem traversal
orderやHashMap iteration orderへ依存してはならない（MUST NOT）。invalid input diagnosticとvalid snapshot間
のbreaking change reportを同じconceptへ混ぜてはならない（MUST NOT）。

### COMPAT-RELEASED-012 — Shared operation and adapters

comparison semantics、entity matching、axis classification、summary aggregationはshared Rust core / application
operationが所有しなければならない（MUST）。CLI / Tauri Desktop / frontendはそのoperationをadapterとして
使用し、独自のdiff、rename inference、classification、axis aggregationを実装してはならない（MUST NOT）。
Option Aのexplicit inputに必要なadditive CLI surfaceとして、baseline/current pathを必須にする
`masterdata compatibility --baseline PATH --current PATH [--json]` commandを公開しなければならない
（MUST）。Tauri Desktopも両pathを受け取るread-only `compatibility_report` commandを同じapplication
operationへlowerしなければならない（MUST）。frontendにreport panelを追加する場合も、shared reportの
presentationに限定しなければならない（MUST）。

## Classification matrix（代表例）

| change | Generated API | Source / Migration | Artifact / Binary | External |
| --- | --- | --- | --- | --- |
| Table add | additive | manual_action_required | cross_schema_interoperability_not_guaranteed | not_assessed |
| Table remove | breaking | manual_action_required | cross_schema_interoperability_not_guaranteed | not_assessed |
| Table `csharpName` change | breaking | not_required | rebuild_required | not_assessed |
| Table field add | additive | supported_operation | cross_schema_interoperability_not_guaranteed | not_assessed |
| field remove | breaking | destructive_authorization_required | cross_schema_interoperability_not_guaranteed | external_policy_required |
| field type / modifier change | breaking | manual_action_required | cross_schema_interoperability_not_guaranteed | external_policy_required |
| MessagePack key change | unchanged | not_required | cross_schema_interoperability_not_guaranteed | external_policy_required |
| Table field declaration reorder | unchanged | not_required | rebuild_required | not_assessed |
| Primary Key shape change | breaking | manual_action_required | cross_schema_interoperability_not_guaranteed | external_policy_required |
| Secondary add | additive | manual_action_required | cross_schema_interoperability_not_guaranteed | not_assessed |
| Secondary remove / uniqueness change | breaking | manual_action_required | cross_schema_interoperability_not_guaranteed | external_policy_required |
| Secondary declaration reorder | unchanged | not_required | rebuild_required | not_assessed |
| Reference add | additive | supported_operation | cross_schema_interoperability_not_guaranteed | not_assessed |
| Reference remove / return shape change | breaking | manual_action_required | cross_schema_interoperability_not_guaranteed | not_assessed |
| Reference `csharpName` change | breaking | not_required | rebuild_required | not_assessed |
| Enum/Flags member add | additive | supported_operation | cross_schema_interoperability_not_guaranteed | external_policy_required |
| Enum/Flags member remove | breaking | destructive_authorization_required | cross_schema_interoperability_not_guaranteed | external_policy_required |
| Enum/Flags member numeric value change | breaking | manual_action_required | cross_schema_interoperability_not_guaranteed | external_policy_required |
| record value only | unchanged | not_required | rebuild_required | not_assessed |
| path/format/comment/file split only | no change | no change | no change | no change |

The matrix is representative acceptance evidence, not a second owner for the four axis vocabulary. Exact
operation preconditions remain with Schema Migration / Type Migration.

## Read-only and non-scope

Compatibility analysis must not modify canonical YAML bytes, `masterdata.toml`, canonical artifacts,
artifact-set receipt, publish targets, Git state, or version metadata. It does not introduce stable Table,
Field, Type, Enum, Reference IDs; release/schema manifest; tombstone or rename lineage; semantic-version
enforcement; cross-schema binary guarantee; external wire contract engine; or Reference-aware automatic migration.

## Acceptance evidence

Focused Rust tests must cover unchanged and formatting/path-only snapshots, file split, data-only change,
Table/field/type/Enum/Flags/Primary/Secondary/Reference changes, generated naming, unmatched entities,
project mismatch, invalid input, deterministic ordering, migration classification, and read-only safety.
Application, CLI, and Tauri tests must demonstrate both explicit inputs reach the shared operation and report
serialization preserves structured axis results. Frontend tests, if a report panel is present, cover rendering
only; entity matching and classification remain outside frontend code.

## Open Questions

None for Released Compatibility v1. Exact Rust struct names, JSON field spelling within the typed report,
console formatting, Tauri panel placement, and Git-ref materialization adapters are implementation details
provided they preserve this contract.

## 非目標

- cross-schema MasterMemory / MessagePack binary compatibility guarantee
- external save/network/database compatibility engine
- persistent stable member IDs, release manifests, rename lineage, tombstones
- semantic-version enforcement, automatic version bump, Git tags/releases
- Reference-aware automatic migration or arbitrary migration scripting
- Web / Browser / Native Host product surface
