# 仕様変更0031: Computed View v1退役とProgrammable View将来要件の分離

Status: Approved

## Why

現在のComputed View v1は、将来必要なProgrammable ViewというHuman intentを先行して具体化した実装だが、独自expression DSL、parser、typed AST、type checker、evaluator、DSL-aware migration等をMasterData自身が所有する設計になっている。

2026-09-23 JSTのHuman reviewで、将来要件は「Excel関数・条件付き書式に近いauthoring支援を、独自DSLを保守せず、汎用プログラミング言語/runtime方向も含めて高い自由度で実現する」ものとして再確認された。現行Computed View v1はその将来方向を固定すべき基盤ではなく、低優先の将来機能に対する方向違いの先行実装である。

Product Simplification & Scope Cleanupでは、現行実装を温存して将来互換性負債にするより、現在のfeature surfaceを全面退役し、将来再設計時に既存DSL contractを背負わない状態へ戻す。

## Human Decision

2026-09-23 JST、Humanは以下を選択した。

- Programmable Viewという将来要件自体は保持する。
- 現在のComputed View v1実装・persisted format・独自DSL semanticsは今回のcleanupで全面退役する。
- 将来のProgrammable Viewは、汎用プログラミング言語/runtimeを含めてゼロベースで再設計できる状態にする。
- 今回のcleanupでは次期language/runtime、aggregate、conditional formatting等の新設計を行わない。

## Affected Specifications

- [Computed View仕様](../specs/computed-view.md): current product authorityから退役する。
- [Authoring system v1 RFC](../rfcs/0008-authoring-system-v1.md): Programmable Viewの将来要求は残してよいが、現行Computed View v1のDSLを将来方向として扱うroutingを除く。
- [Specification index](../specs/README.md): Computed Viewをcurrent implemented capabilityとして案内しない。
- Schema / Type Migration: View expression追随・View dependency precondition等、Computed View専用integrationを除く。
- Project discovery / validation / source creation / GUI routing / Table Overview / Authoring Query: `kind: view`およびcomputed-column専用surfaceを除く。
- Released Compatibilityとのroutingは仕様変更0030の退役と合わせて除く。

## Confirmed Decisions

1. `kind: view` persisted documentはcurrent product formatから削除する。
2. Computed View名、target Table、`columns[].expression`等のv1 source contractを維持しない。
3. 独自expression grammar、parser、AST、type checker、evaluator、operator/null/arithmetic semanticsを全て削除する。
4. View CRUD、preview、source-preserving View patcher、stale lifecycle等、Computed Viewだけのauthoring surfaceを削除する。
5. Table OverviewのView selector、computed columns、computed-value query integrationを削除する。
6. RenameField / DropField / Type Migration等のView専用dependency処理を削除する。
7. BuildがViewを無視するためだけのspecial-case、View専用diagnostic、tests、fixtures、DTO、Tauri command、GUI component、source-creation optionを削除する。
8. 将来要件はcurrent persisted/API compatibility contractとして残さない。Product VisionまたはRFC上の低優先future directionとしてのみrecoverableにする。
9. 将来Programmable Viewを実装するとき、現行Computed View v1 source format / DSL / diagnostic / rename semanticsとのbackward compatibilityを要求しない。

## New / Changed Requirements

### RETIRE-VIEW-001 — No current Computed View product surface

current productは`kind: view` document、Computed View CRUD、computed-column projection、または独自expression evaluationを提供してはならない（MUST NOT）。

core、application、CLI/Tauri、GUI、Build/validation/migrationのいずれにも、Computed View v1専用public surfaceまたはdisabled placeholderを残してはならない（MUST NOT）。

### RETIRE-VIEW-002 — Remove the custom expression subsystem

Computed View v1専用のgrammar、parser、AST、type checker、evaluator、operator semantics、expression-aware rename/token span logic、diagnostics、testsを削除しなければならない（MUST）。

将来再利用を理由にprivate/internal moduleとして温存してはならない（MUST NOT）。

### RETIRE-VIEW-003 — Remove persisted-format and editor coupling

Project document classification、source creation、Explorer/editor routing、Table Overview request/response、migration dependency handlingからComputed View v1固有のformat / fields / operationsを削除しなければならない（MUST）。

削除後、通常のTable/Data/Type/Reference/Migration/Build workflowがViewの存在を前提にしてはならない（MUST NOT）。

### RETIRE-VIEW-004 — Preserve generic infrastructure only when independently used

source-preserving edit、content identity、stale/lost-update detection、source commit、Authoring Query等のgeneric infrastructureは、Computed View以外のretained featureが実際に使用している場合は維持する。

Computed Viewだけが使用しているgeneric-looking abstractionは、実利用を確認した上でdeadなら削除しなければならない（MUST）。

### RETIRE-VIEW-005 — Preserve only the future product intent

将来のProgrammable View requirementは、current implementation authorityではないfuture product directionとしてのみ保持する。

そのfuture directionは、Excel関数・条件付き書式に類するderived authoring experience、高いprogrammability、独自DSL保守を避ける方向、汎用language/runtimeを含む再設計可能性を表してよい。

このcleanupで具体的なlanguage、runtime、sandbox、file format、API、aggregate semantics、conditional-formatting syntaxを決定してはならない（MUST NOT）。

## Implementation Closure

実装エージェントは少なくとも以下をdependency closureとして確認する。

- `crates/masterdata-core/src/computed_view.rs`
- `crates/masterdata-core/src/view_authoring.rs`
- core exports / document modelの`ViewDocument` / `ViewColumnDefinition` / `SourceDocument::View`
- validation / pipeline / source creation / migration / type migration / field mutationのView専用branch
- `crates/masterdata-app/src/computed_view.rs`
- Table Overviewの`view` request、computed column metadata/evaluation/query composition
- Tauri `open_computed_view` / `preview_computed_view` / `save_computed_view` / `remove_computed_view`
- `apps/gui/src/ComputedViewEditor.tsx`
- GUI routing、source creation option、tests
- Computed View専用fixtures / diagnostics / rationale / docs routing
- `docs/specs/computed-view.md` current authority
- spec-change 0027は必要最小限のretirement auditへ縮退可能

具体的file split/private namesはimplementation detailとする。

## Compatibility Impact

意図的なpre-release feature removalである。

- persisted `kind: view` documentsはcurrent productで認識・編集・評価しなくなる。
- Computed View GUI/Tauri/Overview surfacesは削除する。
- v1 DSL source compatibilityを維持しない。
- canonical Table/Data/Type YAML、Reference、Migration operation formats、generated C# / MasterMemory binary、Build/Publish artifact formatは変更しない。
- compatibility shim、legacy parser、automatic conversionを追加しない。

## Acceptance Evidence

- current codeからComputed View v1専用modules/symbols/commands/componentsが消えている。
- repository-wide current references to `ADV-VIEW-`、`E-VIEW-`、`ComputedView`、`computed_view`、`kind: view`が0になる。compact historical audit / future-direction proseは例外。
- Project discovery / validation / source creation / Migration / Overview / BuildにView専用branchが残らない。
- retained Table/Data/Type/Reference authoring、Migration、Build/Publish、Unity delivery regressionsが通る。
- `cargo xtask check-specs`
- `cargo xtask check-rationale`
- `cargo xtask check-all`
- fresh `review-code`でdead coupling・stale rationale・unrelated regressionがない。

## Explicit Non-scope

- 次期Programmable Viewの設計・実装。
- JavaScript / Lua / Rhai / WASM等のruntime選定。
- aggregate / group-by / conditional formattingの実装。
- new View file format。
- Git-native integration。
- Released Compatibilityの代替impact analyzer。

## Open Questions

None for retiring Computed View v1.

Future Programmable View design intentionally remains unresolved.

## Approval Eligibility

Autonomous approval eligible: No.

Human gate: implemented product capability / persisted format removal。

Human decision is already provided on 2026-09-23 JST.

## Review

Fresh review completed 2026-09-23 JST.

### Blocking Issues

None identified.

### Non-blocking Issues

None identified.

### Questions

None identified.

### Approved as Proposed

Yes.

### Review dimensions

Intent fidelity、retirement closure、future requirementとの分離、migration/build/reference safety、documentation ownership、testability、意図的breaking removalを確認した。将来Programmable Viewの具体設計を未決定のまま保つため、現行DSL contractをcompatibility shimとして残さないことを重要なacceptance conditionとする。

## Approval Record

Approval mode: Human.

Approved: 2026-09-23 JST.

Basis: HumanはProgrammable Viewの将来要件を保持しつつ、現行Computed View v1を今回のcleanupで全面退役し、将来は汎用language/runtimeを含めて再設計可能な状態へ戻す方針を選択した。

Application / implementationはimplementation agentのreviewable branch / PRへ委譲し、merge前にfresh review-codeを行う。
