# 仕様変更0027: Advanced Authoring — Computed View v1

Status: Applied

## Affected Specifications

- [Computed View仕様](../specs/computed-view.md): persisted `kind: view`、bounded scalar expression、shared resolution/evaluation、Overview/query、source authoring、migration/build boundaryのcanonical ownerを追加する。
- [Schema language](../specs/schema-language.md): document kind routingへ`view`を追加する。
- [Authoring Query](../specs/authoring-query.md): resolved computed scalarを既存query capabilityへ接続するroutingを追加する。
- [Table Overview GUI](../gui/table-overview/spec.md): saved snapshot上のread-only computed projectionを追加する。
- [Schema Migration](../specs/schema-migration.md): RenameFieldのsafe expression token patch、DropFieldのcomputed dependency fail-closedをroutingする。
- [Released Compatibility](../specs/compatibility/released-compatibility.md): authoring-only view changeがruntime API/binary changeではないことをroutingする。

## Source Evidence

- Current ObjectiveはP5 expression / computed viewをAdvanced Authoringのproduction-ready sliceとして明示している。
- RFC 0008は保存済みsnapshotからのread-only authoring projectionを採用し、P5 computed viewをDeferredとしている。
- 既存source envelopeは`kind: schema|data|type`のtyped dispatch、YAML/Git source authority、shared Rust core/applicationを採用している。
- Overviewは保存済みsource snapshot、`AuthoringValue`、shared Authoring Queryを使い、frontendでdomain semanticsを複製しない。
- Schema Migrationはsource-preserving patch、stale/lost-update protection、Reference dependency fail-closedを既に所有している。

## Adopted Agent Decisions

1. View definitionはTable schemaへ埋め込まず、独立した`kind: view` YAML documentとする。これによりruntime schema fieldとauthoring projectionのidentity/source mutationを分離し、既存Buildがviewをartifactへlowerしない境界を明示できる。
2. expressionはhuman-readable text scalarとして保存し、shared Rust内でsource-span付きrecursive-descent parserとtyped ASTへlowerする。structured AST YAMLをpublic formatにせず、Git diffとsafe token patchを優先する。
3. v1はsingle-level expressionとし、computed columnから別computed columnを参照しない。これによりcycleを構文上排除し、aggregate/join/query languageへ拡張しない。
4. null semanticsはexplicit null-aware equality、`??`、ternary、strict null propagationとし、implicit coercion・truthiness・SQL three-valued logicを導入しない。
5. CLIの新public commandは必須ではないため追加せず、Desktop Table Overviewとshared application operationを主要surfaceとする。必要なcore operationはCLI/Tauriが再利用可能な位置に置く。

## Requirements

- `ADV-VIEW-001` — explicit view documentをparse/discoverし、typed AST/modelとproject diagnosticsへ接続する。
- `ADV-VIEW-002` — view identity、target Table、column identity、expression grammar、source spanを決定的に解決する。
- `ADV-VIEW-003` — existing Type System / AuthoringValueを再利用し、strict scalar typing、null、invalid、checked arithmeticをshared Rustで評価する。
- `ADV-VIEW-004` — saved snapshotのbase rowsへviewを評価し、base+computed columnsをOverviewへread-onlyで返す。
- `ADV-VIEW-005` — computed scalarを既存Authoring Queryへlowerし、frontendでfilter/search/sort semanticsを再実装しない。
- `ADV-VIEW-006` — source-preserving create/edit/remove、stale rejection、no implicit Build/Publish/Gitを提供する。
- `ADV-VIEW-007` — RenameFieldは安全なexpression tokenだけ追随し、DropFieldとambiguous patchはfail closedする。
- `ADV-VIEW-008` — Build/codegen/binaryへcomputed valueを混入させず、existing projectsのruntime surfaceを保持する。
- `ADV-VIEW-009` — deterministic diagnostics/result order、query/profile composition、invalid view isolationを検証する。

## Compatibility / Non-scope

このchangeはadditive authoring capabilityであり、既存Table/data/type syntax、generated C#、MasterMemory binary、
artifact receipt、released stable identityを変更しない。runtime computed field、scripting、aggregate/join、stable
member ID、Webは対象外である。

## Acceptance evidence

canonical specのVerification節に従うfocused Rust/application/GUI/build regressionを実施する。Compatibility analyzerは
view-only changeをruntime generated API/binary breakingと分類しない。source bytes、config、artifact、publish target、
Git stateをOverview/Plan/analysisで変更しない。

## Approval / Application Record

Approval mode: Agent-autonomous. `refine-spec` と `review-spec` のblockingなし、Human gateなしを確認し、
Computed View canonical ownerをApprovedへ適用した。Human-selected Current Objective: Advanced Authoring。
詳細proposalではなく、このaudit recordとcanonical owner / Requirement ID / Git historyをcurrent traceabilityとする。
