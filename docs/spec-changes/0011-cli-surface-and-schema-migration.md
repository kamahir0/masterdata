# 仕様変更: CLI surfaceとSchema Migration v1の提案

Status: Proposed

<!-- Lifecycle: Draft -> Proposed -> Approved -> Applied。Human Approval前はcanonical
     Approved specificationを変更しない。 -->

## Affected Specifications

- `docs/specs/cli.md` — 新規、`Status: Proposed`、CLI terminologyとcommand surfaceのowner
- `docs/specs/schema-migration.md` — 新規、`Status: Proposed`、Migration v1 semanticsのowner
- `docs/specs/build-pipeline.md` — `Status: Approved`、変更しない。build/publish/receiptの既存ownerを参照する
- `docs/specs/runtime-hosts.md` — `Status: Approved`、変更しない。Operation/Capability/host boundaryを参照する
- `docs/specs/project-layout.md` — `Status: Approved`、変更しない。project identityとsource path semanticsを参照する

既存Approved specificationへ未承認のsemantic deltaを直接適用せず、新規Proposed ownerへ
隔離する。

## 根拠と分類（Source Evidence and Classification）

このchangeは、YAMLをcanonical source of truthとするprojectに対して、public CLI taxonomyと
将来のschema-aware deterministic migrationの境界をimplementation前にreview可能にする
ためのものです。

### Human decisionとして記録する提案入力

今回のrequestで示された次の内容を、Human Approval済みcanonical contractとは扱わず、
このproposalのreview対象として記録する。

- CLI terminologyはOperation、CLI Command、Capabilityを分離する。
- target public command surfaceは`init`、`doctor`、`validate`、`generate`、`build`、
  `publish`、`migrate`とする方向で提案する。
- 現行の`project-info`は削除せず、target surfaceとの差をImplementation Gapとして記録する。
- `generate`はvalidation後にC# generationで停止するが、materialization先は未決定とする。
- `publish`はimplicit build/revalidationを行わず、last successful receipt-valid canonical
  artifact setを既存Approved pipelineで配布する。
- `build --publish`はbuild成功後だけpublishするcompositionとして提案する。publish failure
  はbuild済みcanonical artifactをrollbackせず、aggregate failureを返す。
- Schema Migration v1のsemantic operationは`AddField`、`RenameField`、`DropField`に限定する。
- SQL-like public syntaxは固定せず、semantic Command ASTとexecution semanticsを先にreviewする。
- DropFieldにはexplicitでmachine-actionableなdestructive authorizationを要求する。
- Migrationはtransformed projectを既存canonical parser/type/table/selection/full validationへ
 戻してから、source commitを行う。
- existing recordsがあるAddFieldにはexplicit constant initializerを要求し、expression、
  reference、function、implicit default/null、recordごとのAI-generated valueを許可しない。
- MigrationのYAML rewriteはsource-preservingかつdeterministicとし、unaffected fileはbyte-for-byte
  unchanged、affected fileは不要なcomments/formatting等を保持する。全体再serializeを通常成功
  pathにしない。
- patched sourceはcanonical parserで再parseし、expected transformed semantic resultと整合確認
  してからfull validationする。
- RenameField / DropFieldのtargetはlogical Table identityとcurrent field nameで解決し、
  MessagePack `key`や新しいField IDをidentityにしない。
- commit直前にaffected source filesのlost updateを検出し、stale planならmutationを開始しない。
- commit failureはrollback成功ならOLD、rollback failureなら`Recovery Required`として扱い、
  silent continuationをしない。
- user-facing recommended project directory layoutは別議題へdeferし、今回決めない。

上記はこのchangeをHuman Approval済みとして示す記録ではない。`Status: Proposed`を維持し、
canonical specificationへの適用は明示的承認後にのみ可能とする。

## 提案する差分（Proposed Delta）

### CLI owner

`docs/specs/cli.md`をCLI surfaceのcanonical owner候補として追加する。新しいRequirement
familyは`CLI-001`から始め、OperationとCommandの用語、提案command surface、validate、
generate、build、publish、build --publish、migrateの境界、current implementation gapを
定義する。

CLI specificationは、既存Approved build/publish requirementsをコピーしない。`BUILD-ARTIFACT-*`、
`ARTIFACT-SET-*`、`PUBLISH-*`、`PUBLISH-PATH-*`、`PUBLISH-EXEC-*`をownerとして参照する。
同様にruntime hostのcapability、CLI direct path、Native application serviceの意味は
`RUNTIME-HOST-*`と既存ADR/RFCを参照する。

### Schema Migration owner

`docs/specs/schema-migration.md`をMigration v1 semanticsのcanonical owner候補として追加する。
新しいRequirement familyは`MIGRATION-001`から始め、YAML source authority、v1 operation scope、
semantic Command model、determinism、full-project revalidation、Add/Rename/Dropの意味、
destructive authorization、plan/dry-run、source-preserving rewrite、lost-update protection、
multi-file source commit safety、host boundaryを提案する。refinementでは
`MIGRATION-014`〜`MIGRATION-016`を追加する。

`SCHEMA-KEY-*`、`SCHEMA-TABLE-*`、Type System、YAML subset、Build Selectionの既存semanticは
それぞれのownerを維持し、Migration specから再定義しない。

### CLIとMigrationのcomposition

`masterdata migrate`というtop-level operation nameはCLI taxonomyへ提案するが、subcommand
grammar、SQL-like language、JSON plan schema、global output/exit contractは固定しない。
Migrationはsource snapshotを変換するsemantic Operationであり、生成artifactのauthorityや
publish operationではない。

`masterdata build --publish`は、既存Approved build/publish pipelineのsemanticを複製せず、
build success only → publishというcomposition boundaryだけを提案する。

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
- `project-info`はtarget canonical command setに含めない提案だが、削除・rename・代替
  namespaceは決めない。
- `generate`、`publish`、`migrate`、`build --publish`、migration engineは未実装である。
- Migrationは将来、source YAMLの意図的な変換を行うため、実装時にはsource compatibility、
  backup/recoveryを別途検証する必要がある。source-preserving rewrite、lost-update protection、
  `Recovery Required`はこのProposed refinementで候補contractとして追加する。
- canonical binary、generated C#、artifact receipt、external publish targetはMigrationの
  authorityではない。`build`と`publish`の責務分離、receipt validation、B8 path safety、
  multi-target execution semanticsを変更しない。
- user-facing project directory layoutは変更せず、既存Approved project root、artifact、cache
  semanticsだけを参照する。

## 受け入れと実装への影響（Acceptance and Implementation Impact）

### Planned evidence

Human Approval前であり、次のevidenceはすべて未実装・未通過である。

- CLI command surface、Operation owner、Capability boundaryのarchitecture/integration evidence
- `validate`のartifact/publish target no-mutation evidence
- `generate`のmaterialization決定後のcanonical artifact set non-corruption evidence
- `build --publish`のbuild failure、publish failure、partial result、no cross-rollback evidence
- Migration deterministic plan、full-project revalidation、dry-run no-mutation evidence
- AddField key preservation、constant initializer、禁止されたimplicit/default valueのevidence
- RenameFieldのMessagePack key preservationとresolved key/index reference evidence
- DropFieldのexplicit destructive authorizationとdependency failure evidence
- source-preserving rewrite、patched source semantic round-trip、unaffected byte preservationのevidence
- commit直前のlost-update rejection evidence
- multi-file commitのOLD / NEW / Recovery Required state evidence

### Implementation Gap

このproposalをApprovedへ進めても、以下が直ちに実装済みになるわけではない。

- `generate` commandとそのmaterialization policy
- external `publish` runtimeとartifact-set receipt runtime
- `build --publish` composition
- `migrate` parser、semantic engine、plan/dry-run、YAML rewrite、source commit
- `project-info`のtarget surface整理
- Build Profile CLI wiring、Reference runtime、cache等の既存別gap

実装は、Human Approval後にcanonical docsへAppliedされたRequirementだけをauthorityとして
開始する。

## 未解決事項（Open Questions）

最低限、次をapproval-blockingなreview itemとして残す。

- OQ-A: `generate` C# outputのmaterialization先、既存outputとの関係、cleanup
- OQ-B: concrete `migrate` public argument grammarとCLI/GUI/Web/AI adapter input surface
- OQ-C: concrete source patch implementation mechanism（CST、lossless parser、rope等）
- OQ-D: concrete staging、journal、backup、rollback、Recovery Requiredからのrecovery mechanism
- OQ-E: user-facing recommended project directory layout。`.masterdata/generated/csharp`等を
  今回のcanonical/recommended layoutにしない
- OQ-F: migration plan/resultのstructured diagnostics、JSON、stdout/stderr、exit code contract
- global CLI `--json`、short options、deprecation/versioning、Build Profile syntax、SQL-like grammar

これらを実装都合で暗黙に選択してはならない。

## レビュー（Review）

- 必要なreview: CLI public surfaceとSchema Migration v1 semanticsの分離、CLI-011とstandalone
  publishの整合、AddField initializer、source-preserving rewrite、field target identity、
  lost-update protection、Recovery Required state、既存Approved build/publish/runtime
  contractsとの参照関係、OQ-A〜OQ-Fの十分性。
- independent reviewで発見したCLI-011 / publish conflictを修正し、initializer、rewrite
  fidelity、field identity、lost update、recovery stateのrefinementをこのProposed changeへ
  反映した。
- Human Approval: 未実施。`Status: Proposed`から変更してはならない。
- canonical apply: 未実施。`docs/specs/build-pipeline.md`、`project-layout.md`、
  `runtime-hosts.md`の未承認semantic変更は行っていない。

## 承認記録（Approval Record）

Human maintainerによる明示的なApprovalはまだ記録しない。
