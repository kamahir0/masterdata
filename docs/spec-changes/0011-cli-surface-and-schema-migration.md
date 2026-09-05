# 仕様変更: CLI surfaceとSchema Migration v1のcanonical化

Status: Applied

<!-- Lifecycle: Draft -> Proposed -> Approved -> Applied。Human Approval後に承認済みdeltaを
     canonical specificationへ適用したaudit recordである。 -->

## Affected Specifications

- `docs/specs/cli.md` — 新規、`Status: Approved`、CLI terminologyとcommand surfaceのowner
- `docs/specs/schema-migration.md` — 新規、`Status: Approved`、Migration v1 semanticsのowner
- `docs/specs/table-and-keys.md` — `Status: Approved`、SCHEMA-TABLE-006へdata record mapping orderのdeltaを適用
- `docs/specs/build-pipeline.md` — `Status: Approved`、変更しない。build/publish/receiptの既存ownerを参照する
- `docs/specs/runtime-hosts.md` — `Status: Approved`、変更しない。Operation/Capability/host boundaryを参照する
- `docs/specs/project-layout.md` — `Status: Approved`、変更しない。project identityとsource path semanticsを参照する

既存Approved specificationへ適用したdeltaと、変更しない既存ownerの範囲をこのchangeに記録する。

## 根拠と分類（Source Evidence and Classification）

このchangeは、YAMLをcanonical source of truthとするprojectに対して、public CLI taxonomyと
将来のschema-aware deterministic migrationの境界をimplementation前にreview可能にする
ためのものです。

### 承認された入力（Approved Scope）

Human maintainerが明示的に承認した、今回canonical化するcontract scopeを記録する。

- CLI terminologyはOperation、CLI Command、Capabilityを分離する。
- canonical public command surfaceは`init`、`doctor`、`validate`、`generate`、`build`、
  `publish`、`migrate`とする。
- 現行の`project-info`は削除せず、target surfaceとの差をImplementation Gapとして記録する。
- `generate`はvalidation後にC# generationで停止するが、materialization先は未決定とする。
- `publish`はimplicit build/revalidationを行わず、last successful receipt-valid canonical
  artifact setを既存Approved pipelineで配布する。
- `build --publish`はbuild成功後だけpublishするcompositionとする。publish failure
  はbuild済みcanonical artifactをrollbackせず、aggregate failureを返す。
- Schema Migration v1のsemantic operationは`AddField`、`RenameField`、`DropField`に限定する。
- SQL-like public syntaxは固定せず、semantic Command ASTとexecution semanticsをpublic syntaxに
  先行するsemantic contractとして定義する。
- DropFieldにはexplicitでmachine-actionableなdestructive authorizationを要求する。
- Migrationは必要なsource / schema / type closureをresolveし、operation-specific postcondition
  とsafe source commitを確認する。Project全体error-freeはMigration successの条件にしない。
- Migrationと無関係であることを安全に判定できる既存diagnosticは許容し、対象との関係を
  判定できないunclassifiable sourceはfail closedする。
- Build Selectionの全profile、unfiltered selection、specific selected-dataset validationを
  Migration success gateにしない。
- existing recordsがあるAddFieldにはexplicit constant initializerを要求し、expression、
  reference、function、implicit default/null、recordごとのAI-generated valueを許可しない。
- AddField initializerはPrimitive / Value Object / Enumのscalar、Nullableのexplicit null/value、
  Array / Flagsのsequence、Custom Typeのmappingなど、target typeのApproved canonical data
  representationとしてvalidなconstant valueとする。
- AddFieldはschema fields sequence末尾と各record mapping member末尾へappendし、既存順序を
  reorderしない。
- MigrationのYAML rewriteはsource-preservingかつdeterministicとし、unaffected fileはbyte-for-byte
  unchanged、affected fileは不要なcomments/formatting等を保持する。全体再serializeを通常成功
  pathにしない。
- patched sourceはcanonical parserで再parseし、expected transformed semantic resultと整合確認
  し、operation-specific postconditionを確認する。project-wide diagnosticsは別に収集できる。
- RenameField / DropFieldのtargetはlogical Table identityとcurrent field nameで解決し、
  MessagePack `key`や新しいField IDをidentityにしない。
- commit直前にMIGRATION-016が定義するbase project inputsのstale updateを検出し、不一致なら
  mutationを開始しない。
- commit failureはrollback成功ならOLD、rollback failureなら`Recovery Required`として扱い、
  silent continuationをしない。
- data record mapping member orderにはdomain semanticsを与えず、このchangeで
  `SCHEMA-TABLE-006`へ適用する。schema field declaration orderのpresentation semanticsとは
  分離する。
- Migrationはimplicit Formatterではなく、formatter operation、standard order、`$tags` placement
  を今回決めない。
- user-facing recommended project directory layoutは別議題へdeferし、今回決めない。

上記scopeは末尾のApproval Recordに記録したexplicit Human Approvalを根拠として、以下の
canonical specificationへ適用した。

## 適用した差分（Applied Delta）

### CLI owner

`docs/specs/cli.md`をCLI surfaceのcanonical ownerとして追加し、`Status: Approved`へ進めた。
`CLI-001`から`CLI-011`で、OperationとCommandの用語、canonical command surface、validate、
generate、build、publish、build --publish、migrateの境界、current implementation gapを定義する。

CLI specificationは、既存Approved build/publish requirementsをコピーしない。`BUILD-ARTIFACT-*`、
`ARTIFACT-SET-*`、`PUBLISH-*`、`PUBLISH-PATH-*`、`PUBLISH-EXEC-*`をownerとして参照する。
同様にruntime hostのcapability、CLI direct path、Native application serviceの意味は
`RUNTIME-HOST-*`と既存ADR/RFCを参照する。

### Schema Migration owner

`docs/specs/schema-migration.md`をMigration v1 semanticsのcanonical ownerとして追加し、
`Status: Approved`へ進めた。`MIGRATION-001`から`MIGRATION-017`で、YAML source authority、v1 operation scope、
semantic Command model、determinism、migration resolution closure、Migration Resolvable、
Add/Rename/Dropのoperation-specific postcondition、destructive authorization、plan/dry-run、
source-preserving rewrite、lost-update protection、multi-file source commit safety、host
boundaryを定義する。既存Requirement IDのrename、reassign、追加は行っていない。

`SCHEMA-KEY-*`、`SCHEMA-TABLE-*`、Type System、YAML subset、Build Selectionの既存semanticは
それぞれのownerを維持し、Migration specから再定義しない。

### CLIとMigrationのcomposition

`masterdata migrate`というtop-level operation nameはCLI taxonomyへ定義するが、subcommand
grammar、SQL-like language、JSON plan schema、global output/exit contractは固定しない。
Migrationはsource snapshotを変換するsemantic Operationであり、生成artifactのauthorityや
publish operationではない。

`masterdata build --publish`は、既存Approved build/publish pipelineのsemanticを複製せず、
build success only → publishというcomposition boundaryだけを定義する。

### Migration Resolvableのrefinement

独立reviewで、既存proposalのfull-project validation wordingが、Migrationと無関係な既存
diagnosticまでMigration blockingにすることを確認した。Migration successを、Project全体の
error-freeではなく、target・必要なsemantic closure・operation-specific postcondition・
safe commitを確定できるMigration Resolvable modelへ変更する。canonical parsing / semantic
resolutionはdiagnosticを生成してもよい。必要なstructure、logical Table / field symbols、type
declarations、operation-specific dependency、patch location / provenanceを安全かつ決定論的に
resolveでき、diagnosticがclosure外またはMigration operationと安全に無関係であると分類できる
限り、そのdiagnosticだけではMigrationをblockingにしない。一方、closureを構成できない、target
との関係を判定できない、またはunclassifiable sourceはfail closedする。

Build Selectionはbuild-time selected logical datasetのownerであり、全profileのPK / Unique /
Reference validationをMigration success gateにしない。ただしclosure外と安全に分類できない
壊れたsource documentはunrelatedと推測せずblockingとする。

### CLI-011のrefinement

独立reviewで、CLI-011の「前段を飛ばしてはならない」という一般化が、current YAMLを
parse/validateしないApproved standalone publishと文面上衝突することを確認した。CLI-011の
適用範囲を`validate`、`generate`、`build`のsource-derived staged operationsへ限定し、
`publish`はpublish-eligible canonical artifact setをpreconditionとする別Operationである
ことを明記する。receipt/integrity validation、target preflight、target executionのownerは
既存`docs/specs/build-pipeline.md`のままとする。

## 互換性（Compatibility）

- 既存の`init`、`doctor`、`project-info`、`validate`、`build`実装はこのdocs-only changeで
  変更しない。
- `project-info`はtarget canonical command setに含めないが、削除・rename・代替
  namespaceは決めない。
- `generate`、`publish`、`migrate`、`build --publish`、migration engineは未実装である。
- Migrationは将来、source YAMLの意図的な変換を行うため、実装時にはsource compatibility、
  backup/recoveryを別途検証する必要がある。source-preserving rewrite、lost-update protection、
  `Recovery Required`はこのApplied changeでcanonical contractとして追加した。
- canonical binary、generated C#、artifact receipt、external publish targetはMigrationの
  authorityではない。`build`と`publish`の責務分離、receipt validation、B8 path safety、
  multi-target execution semanticsを変更しない。
- user-facing project directory layoutは変更せず、既存Approved project root、artifact、cache
  semanticsだけを参照する。

## 受け入れと実装への影響（Acceptance and Implementation Impact）

### Planned evidence

Human Approval済みだが、次のevidenceはすべて未実装・未通過である。

- CLI command surface、Operation owner、Capability boundaryのarchitecture/integration evidence
- `validate`のartifact/publish target no-mutation evidence
- `generate`のmaterialization決定後のcanonical artifact set non-corruption evidence
- `build --publish`のbuild failure、publish failure、partial result、no cross-rollback evidence
- Migration resolution closure、Migration Resolvable、operation-specific postcondition、
  unrelated diagnosticsの許容、unclassifiable sourceのfail-closed evidence
- Build Selection selected-dataset validationをMigration success gateにしないevidence
- Migration deterministic plan、patched source semantic round-trip、dry-run no-mutation evidence
- AddField key preservation、canonical constant representation、schema/data末尾append、禁止された
  implicit/default valueのevidence
- RenameFieldのMessagePack key preservationとresolved key/index reference evidence
- DropFieldのexplicit destructive authorizationとdependency failure evidence
- source-preserving rewrite、patched source semantic round-trip、unaffected byte preservationのevidence
- MIGRATION-016が定義するproject config、source file set membership、closure / postcondition判断で
  ロードしたcanonical source inputsを含むcommit直前のstale-update rejection evidence
- multi-file commitのOLD / NEW / Recovery Required state evidence
- schema field orderとdata mapping member orderを分離し、Formatter semanticsを混在させないevidence

### Implementation Gap

このchangeをAppliedにしても、以下が直ちに実装済みになるわけではない。

- `generate` commandとそのmaterialization policy
- external `publish` runtimeとartifact-set receipt runtime
- `build --publish` composition
- `migrate` parser、semantic engine、plan/dry-run、YAML rewrite、source commit
- `project-info`のtarget surface整理
- Build Profile CLI wiring、Reference runtime、cache等の既存別gap

実装は、今回Approvedになったcanonical specificationのRequirementをauthorityとして開始できる。

## 未解決事項（Open Questions）

次の事項は、承認済みobservable contractと矛盾しない形でdeferred future product decision、
implementation detail、またはimplementation gapとして残す。

- OQ-A: `generate` C# outputのmaterialization先、既存outputとの関係、cleanup
- OQ-B: concrete `migrate` public argument grammarとCLI/GUI/Web/AI adapter input surface
- OQ-C: concrete source patch implementation mechanism（CST、lossless parser、rope等）
- OQ-D: concrete staging、journal、backup、rollback、Recovery Requiredからのrecovery mechanism
- OQ-E: user-facing recommended project directory layout。`.masterdata/generated/csharp`等を
  今回のcanonical/recommended layoutにしない
- OQ-F: migration plan/resultのstructured diagnostics、JSON、stdout/stderr、exit code contract
- OQ-G: formatter operation、record standard formatting order、`$tags` placement、schema formatter
- global CLI `--json`、short options、deprecation/versioning、Build Profile syntax、SQL-like grammar

これらを実装都合で暗黙に選択してはならない。未決定事項を残したまま承認されたcontractの
実装を開始できるが、該当する未決定behaviorを実装前に追加承認なく確定してはならない。

OQ-CとOQ-Dは主にimplementation detailであり、mechanismを固定しない。OQ-A、OQ-B、OQ-E、
OQ-F、OQ-Gはdeferred product/public semantic decisionである。global CLI output、short
options、deprecation/versioning、Build Profile CLI wiring、SQL-like grammarは別のpublic
surfaceまたはimplementation gapとして扱う。

## レビュー（Review）

- 必要なreview: CLI public surfaceとSchema Migration v1 semanticsの分離、CLI-011とstandalone
  publishの整合、Migration Resolvableとdiagnostic boundary、Build Selection非依存、AddField
  initializerと末尾append、source-preserving rewrite、field target identity、lost-update
  protection、Recovery Required state、schema/data order owner、Formatter境界、既存Approved
  build/publish/runtime contractsとの参照関係、OQ-A〜OQ-Gの十分性。
- independent reviewで発見したCLI-011 / publish conflictを修正し、initializer、rewrite
  fidelity、field identity、Migration Resolvable、Build Selection boundary、lost update、
  recovery state、AddField placementをこのchangeへ反映した。
- `docs/specs/table-and-keys.md`のSCHEMA-TABLE-006へrecord mapping member orderのdeltaを適用し、
  schema field declaration orderのpresentation semanticsおよび`$tags` ownerは変更していない。
- Human Approval済み。`docs/specs/cli.md`と`docs/specs/schema-migration.md`を`Approved`へ進めた。
- canonical apply済み。`docs/specs/build-pipeline.md`、`project-layout.md`、`runtime-hosts.md`の
  既存semanticは変更していない。production implementationは未実施であり、Implementedではない。

## 承認記録（Approval Record）

Human maintainerは、`5945010c745287b0f0e8fe107c6fe4747e46afb8`時点の0011 proposalを最終reviewし、
Blockingなしを確認した後、「次に進む」と回答した。これは、このconversationにおけるexplicitな
Human Approvalであり、commitまたはpushから推測したものではない。

承認scopeは次のとおりである。

- CLI terminologyとcanonical command surface
- `generate` / `build` / `publish` / `build --publish`のresponsibility boundary
- Schema Migration v1の`AddField` / `RenameField` / `DropField`
- Migration Resolvable model
- diagnostic-tolerant resolutionとunclassifiable sourceのfail-closed
- Build Selection selected-dataset validation非依存
- AddField initializerとschema/data append placement
- RenameField / DropFieldのidentityとdependency semantics
- source-preserving deterministic rewrite
- stale-plan protection
- `Recovery Required`
- schema field orderとdata record mapping member orderの分離
- MigrationとFormatterの責務分離
- unresolved future decisionsを解決せずdeferしたまま承認すること

このapprovalを根拠として、`docs/specs/cli.md`、`docs/specs/schema-migration.md`、および
`docs/specs/table-and-keys.md`の承認済みdeltaをcanonical specificationへ適用した。
CLI runtime、Migration engine、YAML patch、source commit、tests、fixturesは未実装であり、
このspecification changeのStatusは`Applied`である。
