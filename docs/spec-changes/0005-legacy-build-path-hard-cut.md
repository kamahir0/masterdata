# 仕様変更: legacy build path configurationをhard cutする（Specification change）

Status: Applied

<!-- Lifecycle: Draft -> Proposed -> Approved -> Applied、またはRejected。Approved canonical
     specificationを変更する前にhuman approvalが必要である。 -->

## Affected Specifications

- [`docs/specs/project-layout.md`](../specs/project-layout.md)、`PROJECT-CONFIG-003`から
  `PROJECT-CONFIG-006`および`PROJECT-PATH-001`。
- [`docs/specs/build-pipeline.md`](../specs/build-pipeline.md)、`BUILD-ARTIFACT-001`、
  `BUILD-ARTIFACT-003`、`BUILD-ARTIFACT-005`およびcanonical configuration migration guidance。

## 根拠と分類（Source Evidence and Classification）

基準commit `a9e3c125d06f6e8ad25fd4abbd48c82a2ea5c920` では、canonical artifact / publish target modelがApprovedとなった一方、
legacy `build.output`と`build.binary_output`の受理期間、自動migration、および既存artifactの扱いをOpen Questionとして残していた。

このtaskのHuman Approvalは、legacy configurationを新canonical configurationのcompatibility aliasとして扱わず、新implementationの導入時に
即時hard cutすることを明示的に承認した。これは次のDecisionとConstraintに分類される。

- `Decision`: canonical configurationは`build.artifact_dir`、`build.cache`、および0..N個の`[[publish.targets]]`を使用する。
- `Constraint`: `build.output`と`build.binary_output`は受理せず、それぞれmigration-awareなstructured diagnosticで拒否する。
- `Constraint`: legacy pathを`artifact_dir`またはpublish targetへ推測変換せず、legacy rejection時にfilesystem artifactを変更しない。
- `Decision`: `init`はcanonical build configurationを生成し、publish targetは初期configurationで省略できる。

旧`build.output`はcanonical project-local artifactだったのか外部C# publish destinationだったのかを旧pathだけから判定できない。旧
`build.binary_output`もcanonical binaryとexternal distributionの責務を混在させていた。このため、toolが意図を推測する自動変換は安全な
migrationにならない。

## 提案する差分（Proposed Delta）

### 1. Legacy field rejection

`docs/specs/project-layout.md`へ次のrequirementを追加する。

- `PROJECT-CONFIG-004`: legacy `build.output`が存在する場合は`E-CONFIG-LEGACY-BUILD-OUTPUT`、legacy `build.binary_output`が存在する場合は
  `E-CONFIG-LEGACY-BINARY-OUTPUT`を含むstructured migration diagnosticを返し、fieldを受理しない。diagnosticはlegacy keyと、
  `build.artifact_dir`または`[[publish.targets]]`を使う手動migration guidanceを識別できるものとする。generic unknown-key errorへ潰さない。
- `PROJECT-CONFIG-005`: legacy rejection時はbuild/publishを開始せず、legacy path、canonical artifact、またはexternal destinationをmove、delete、rename、
  writeしない。legacy fieldを新configurationへ自動変換しない。
- `PROJECT-CONFIG-006`: `init`は`build.artifact_dir = ".masterdata/output"`と`build.cache = ".masterdata/cache"`を生成し、legacy keyを生成しない。
  `publish.targets`は0..Nであり、初期configurationで省略できる。

両方のlegacy fieldが存在する場合、validatorは両方をstable orderでcollectしてよい。first-error modelを採用する場合は
`build.output`、`build.binary_output`の順に診断する。いずれの場合もbuild/publish workを開始しない。

### 2. Canonical document application

`docs/specs/build-pipeline.md`のmigration sectionを、hard cutと明示的手動migrationの説明へ更新する。legacy configを受理する期間、warning-only、
compatibility alias、one-release grace period、automatic conversion、migration commandをcanonical behaviorとして定義しない。

`BUILD-ARTIFACT-001`、`BUILD-ARTIFACT-003`、`BUILD-ARTIFACT-005`が定めるproject-local canonical root、固定canonical binary、build/publish separationと、
新しい`PROJECT-CONFIG-004`から`PROJECT-CONFIG-006`を同じcanonical configuration contractとして参照する。legacy artifact cleanupはこのdeltaの対象外であり、
既存artifactをdisk上に残すことを許可する。

### 3. Project layout lifecycle

`docs/specs/project-layout.md`は、旧configurationをcurrent implementationが受理している事実をimplementation gapとして記録しつつ、canonical
configuration contractを`Status: Approved`で保持する。`PROJECT-001`、`PROJECT-002`、`PROJECT-003`、`PROJECT-004`、`PROJECT-005`、`PROJECT-006`の
project marker、discovery、identity semanticsは変更しない。

Requirement IDはrename、reassign、duplicateしない。

## 互換性（Compatibility）

これはconfiguration surfaceに対する意図的なbreaking changeである。canonical configuration implementationは旧`build.output`と
`build.binary_output`を受理しない。旧configurationを使うprojectはstructured migration diagnosticを受け、ユーザーが旧pathの意図を確認して
`artifact_dir`または明示的なpublish targetへ書き換える必要がある。

このdeltaは旧configurationの自動変換、warning期間、compatibility alias、既存artifactの移動・削除、またはmigration commandを提供しない。
legacy artifactはconfiguration rejectionによって変更されない。YAML、Type System、Table/Key、MessagePack field `key`、project identity、external
publish path safety、released binary compatibilityは変更しない。

## 受け入れと実装への影響（Acceptance and Implementation Impact）

canonical documentsがApprovedになった後のimplementation taskでは、少なくとも次を確認する。

- `build.output`のみ、`build.binary_output`のみ、両方を含むconfigurationを、それぞれ適切なstructured migration diagnosticで拒否する。
- diagnosticにlegacy keyと、canonical artifactまたはpublish targetへの手動migration guidanceを含める。
- legacy rejection時にbuild、publish、init後のartifact処理を開始せず、既存canonical/legacy/external artifactを変更しない。
- `init`がlegacy keyを生成せず、publish targetなしでcanonical build可能な新configurationを生成する。
- 新configurationの`artifact_dir`、`cache`、および`publish.targets`を既存のApproved artifact/publish requirementsへ接続する。

このcanonical applicationはdocs-onlyであり、Rust、config parser/model、CLI、init implementation、Tauri、filesystem publisher、tests、fixtures、
current production behaviorを変更しない。受け入れmatrixのplanned testは、将来のimplementation evidenceが得られるまでpass済みとは扱わない。

## 未解決事項（Open Questions）

このdelta固有のOpen Questionはない。B8のexternal path equivalence、absolute publish path、parent creation、partial publish、publish-only trust、
Unity`.meta`、semantic hash/cache等は、影響を受けるcanonical specificationのOpen Questionsに残る。

## レビュー（Review）

このdeltaは、Approved canonical artifact / publish target modelに対してHuman maintainerが明示したlegacy configuration hard cutを記録する。
canonical documentsへ適用したが、実装、migration command、warning期間、compatibility alias、artifact cleanupを追加していない。

## 承認記録（Approval Record）

このtask inputにおいてHuman maintainerは、legacy `build.output`と`build.binary_output`を新canonical configuration implementationで即時rejectし、
自動migrationを行わず、structured migration diagnosticを返し、rejection時にfilesystem artifactを変更しない方針を明示的に承認した。
このapprovalを根拠として`project-layout.md`と`build-pipeline.md`へcanonical deltaを適用し、本artifactを承認済みdeltaのaudit recordとして
`Status: Applied`にした。
