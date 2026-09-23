# 仕様変更0030: Product Simplification — Released Compatibility退役

Status: Approved

## Affected Specifications

- [Released Compatibility v1](../specs/compatibility/released-compatibility.md): current product capabilityから退役し、canonical implementation authorityを終了する。
- [Compatibility仕様index](../specs/compatibility/README.md): compatibility familyをcurrent product surfaceとして維持しない。
- [Table identity](../specs/compatibility/table-identity.md): current Table identityは既に`SCHEMA-TABLE-002`がcanonicalに所有するためduplicate ownerを終了する。
- [Field identity](../specs/compatibility/field-identity.md): retired Field ID historyはspec-change 0003 / Git historyへ寄せ、current canonical treeのcompatibility familyから除く。
- [Enum identity](../specs/compatibility/enum-identity.md)、[Index identity](../specs/compatibility/index-identity.md):未承認Draftのexternal/released compatibility方向をcurrent treeから除く。
- [CLI surface](../specs/cli.md): `masterdata compatibility` public commandを削除する。
- [Table Editor](../gui/table-editor/spec.md)、[Type Editor](../gui/type-editor/spec.md): Migration PlanからReleased Compatibility report attachmentを削除し、Plan / Diff / diagnostics / destructive authorization / stale safetyは維持する。
- [Computed View](../specs/computed-view.md)、[Index / Reference](../specs/index-and-reference.md)、その他Released Compatibilityへのroutingだけを持つowner: retired subsystemへのroutingを除く。各domain semantics自体は変更しない。
- [Specification index](../specs/README.md)、README、Product Vision等のcurrent routing: release-to-release compatibility analyzerをcurrent product surfaceとして案内しない。

## Source Evidence and Classification

### Human Decision

2026-09-23 JST、Humanは次の新機能へ進む前にProduct Simplification & Scope CleanupをCurrent Objectiveとして選択し、不要・過剰・誤って昇格した機能を十分なコストを掛けて徹底的に除去する方針を指定した。

同conversationでReleased Compatibilityについて、半端なimpact-analysis subsetを残すより、独立したrelease compatibility機能を全て無くし、editor/toolが将来影響を推測しない単純なモデルへ戻す方向を選択した。Current ObjectiveにもReleased Compatibilityを独立product capabilityとして全retireする方向が記録されている。

### Confirmed retained constraints

- source mutationそのもののcorrectness、source-preserving patch、lost-update / stale detection、rollback / Recovery Required、destructive authorizationはretireしない。
- Table / Type MigrationのPlanとbefore/after Diffはretireしない。
- Buildはcurrent canonical sourceからcoherent generated C# / MasterMemory binary setを生成し、Build / Publish safetyを維持する。
- Table identity、MessagePack key、Primary / Secondary Key、Reference等のcurrent-schema semanticsはそれぞれ既存ownerが保持する。
- save data、network、external database等のexternal compatibilityをMasterDataが推測する別subsystemを新設しない。
- Git-native Collaborationは仕様変更0029でRejected済みであり、本changeから代替Git integrationを追加しない。
- Programmable Viewの将来要件と現行Computed Viewの扱いは本changeの削除scopeに含めない。cleanup中にDSL拡張や次期runtime設計を行わない。

## Confirmed Decisions

1. Released Compatibility v1をcurrent productから全面退役する。
2. baseline/current snapshot comparison、4-axis classification、compatibility report、public compatibility CLI/Tauri operation、Migration Planへのcompatibility attachmentを全て削除する。
3. Released Compatibility専用のtype / DTO / adapter / renderer / test / documentation routingを「将来使うかもしれない」ために残さない。
4. Migration Planはcompatibility reportなしで、operation、target、destructive state、affected files/occurrences、migration-local diagnostics、before/after Diff、stale safetyを表示する。
5. current Table identityは`SCHEMA-TABLE-002`が既に所有しているため、Compatibility directoryを第二ownerとして残さない。
6. retired / rejected historyのrecoverabilityはcompact spec-change recordとGit historyで担保し、current canonical treeへ旧feature contractを残さない。

## New / Changed Requirements

### RETIRE-COMPAT-001 — No release compatibility product surface

current productは、2つのProject snapshotを比較してGenerated API / Source Migration / Artifact Binary / External Contractを分類するReleased Compatibility operationを提供してはならない（MUST NOT）。

CLI、Tauri、GUI、shared application/coreのいずれにも、そのoperation専用のpublic surfaceまたはdisabled placeholderを残してはならない（MUST NOT）。

### RETIRE-COMPAT-002 — Migration remains local to the requested mutation

Table / Type Migration Planは、requested mutationのbefore/after source、affected files / occurrences、migration-local validation / diagnostics、destructive stateを表示しなければならない（MUST）。

Plan作成またはApplyのsuccess/failureを、release-to-release Generated API / Binary / External compatibility classificationへ依存させてはならない（MUST NOT）。

### RETIRE-COMPAT-003 — No replacement impact-analysis subsystem

Released Compatibility退役の代替として、別名のGenerated API breaking detector、binary impact classifier、external contract predictor、release snapshot matcherを本Objective内で新設してはならない（MUST NOT）。

compiler、Build、tests、consumer側の検証が観測するfailureを、MasterDataが事前のrelease policy classificationとして再実装しない。

### RETIRE-COMPAT-004 — Remove dead ownership and implementation closure

Released Compatibilityだけに存在理由を持つcore module、application adapter、CLI/Tauri command、GUI component、serialization DTO、tests、fixtures、docs routing、Requirement referencesは削除しなければならない（MUST）。

削除後にunused compatibility abstraction、stub、feature flag、dead enum、commented codeを残してはならない（MUST NOT）。

### RETIRE-COMPAT-005 — Preserve core editing and delivery safety

本changeはsource-preserving authoring、Schema / Type Migrationのdependency/precondition validation、stale/lost-update rejection、destructive authorization、Recovery Required、Build / Publish receipt/path/ownership safety、Unity delivery semanticsを変更してはならない（MUST NOT）。

Reference dependencyやComputed View dependencyなど、requested migrationを正しく完了するためのcurrent-workspace dependency ruleはrelease compatibility analysisではなくmutation correctnessとして維持する。

## Implementation Impact

実装エージェントは少なくとも次のdependency closureを確認し、専用surfaceを除去する。

- `crates/masterdata-core/src/compatibility.rs` とexports、focused compatibility tests。
- `crates/masterdata-app/src/compatibility.rs`、`NativeApplicationService::analyze_compatibility`、app tests。
- `masterdata-cli compatibility --baseline/--current`、renderer、CLI tests。
- Tauri `compatibility_report` commandとserialization test。
- `MigrationCompatibilityView`、Table / Type Plan DTOの`compatibility` field、candidate comparison呼び出し。
- `apps/gui/src/MigrationCompatibilityImpact.tsx` とTable/Type Editor wiring/tests。
- `COMPAT-RELEASED-*` requirement references、Released Compatibility canonical doc/index/routing。
- `docs/spec-changes/0024-released-compatibility-v1.md` と`0026-migration-plan-compatibility-impact.md`はcurrent authorityではないため、retirement traceabilityだけのcompact audit recordへ縮退してよい。
- Compatibility directoryに残るDraft/Deprecated historyを整理し、current Table identity ownerを`SCHEMA-TABLE-002`へ一本化する。
- README / specs index / GUI specs / related spec routingからstale compatibility guidanceを除去する。

具体的なprivate function名やfile splitはimplementation detailであり、削除closureを満たす範囲でagentが決めてよい。

## Compatibility Impact

これは意図的なproduct surface removalである。

- `masterdata compatibility` commandは削除する。
- Tauri `compatibility_report` commandとMigration Plan responseの`compatibility` fieldは削除する。
- Released Compatibility JSON report shapeは維持しない。
- canonical YAML、`masterdata.toml`、generated C# / MasterMemory binary format、Build/Publish artifact shapeは変更しない。
- existing source projectsはReleased Compatibility featureを使用していない限りsource migrationを必要としない。

本projectはpre-release development中のcleanupとしてこのsurface removalを受け入れ、compatibility shimを導入しない。

## Acceptance Evidence

- core / app / CLI / Tauri / frontendからReleased Compatibility専用module/symbol/command/componentが消えている。
- Table / Type Migration Planがcompatibility payloadなしでPlan / Diff / destructive confirmation / stale re-plan / Applyを継続できる。
- Reference-aware RenameField等のmutation correctness regressionが通る。
- Build / Publish / receipt / Unity integration regressionsが通る。
- `cargo xtask check-specs` と`cargo xtask check-rationale`でretired Requirement/doc referenceがcurrent authorityとして残っていない。
- repository-wide searchで`COMPAT-RELEASED`、`compatibility_report`、`MigrationCompatibilityImpact`等のcurrent implementation referenceが0になる。compact historical audit record内の名称は許容する。
- `cargo xtask check-all`が成功する。
- exact Candidateをfresh `review-code`し、required remote CIをreconcileする。

## Audit Findings Outside This Change

全体監査の結果、次は本changeでretireしない。

- P1–P4の日常authoring / schema editing / source path mutation: 根本的なauthoring workflowへ直接寄与するためretain。
- Reference v1: master-data relationship / integrityというdomain機能でありretain。scope縮小は別Human decisionなしに行わない。
- Schema / Type Migration: safe authoring correctnessとしてretain。
- Build / Publish / receipt: canonical artifactをUnityへ安全に届けるcore deliveryとしてretain。
- Unity Integration: product identityがUnity + MasterMemory向けであるためretain。
- `dotnet/spike`: product featureではなくADR 0003と`check-all`が要求する実MasterMemory dependency/API verification assetであり、このcleanupではretain。
- Web / Native Host: 既に仕様変更0022でretired。production code再導入なしを維持する。
- Computed View: 将来のProgrammable View要件はHuman intentに存在する一方、current DSL設計の扱いは別product decisionを要するため、本changeでは実装削除を行わず凍結する。

## Open Questions

None for Released Compatibility retirement.

Computed Viewの現行実装をcleanupで同時retireするかは本changeのauthorityではない。別Human decisionなしにimplementation agentが削除してはならない。

## Potential ADRs

None。Released Compatibilityの採用architectureを置き換える新architectureは導入しない。

## Approval Eligibility

Autonomous approval eligible: No

Human gate: Product capability removal / public CLI and adapter surface removal。ただし2026-09-23 JSTのHuman-selected cleanup directionとReleased Compatibility全retire方針によりdecisionは既に提供済み。

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

### Autonomous approval eligibility

- Eligible: No
- Human gate: product capability / public surface removal
- Rationale: the required Human decision is already provided by the Current Objective and conversation. Approval therefore proceeds as Human-approved, not agent-autonomous.

### Review dimensions

Intent fidelity、internal/cross-spec consistency、normative strength、testability、backward compatibility、unresolved ambiguity、implementation leakage、documentation ownershipを再確認した。意図的breaking removalは明示済みで、core mutation safetyをretainする境界もtest可能。旧Requirement IDは再利用せず、retired historyをcompact audit/Git historyでrecover可能にすることをimplementation reviewで確認する。

## Approval Record

Approval mode: Human.

Approved: 2026-09-23 JST.

Basis: Human-selected Product Simplification & Scope Cleanup Objective、およびReleased Compatibilityを半端に残さず独立機能として全面退役する方針。

Application / implementationはimplementation agentのreviewable branch / PRへ委譲し、merge前にfresh review-codeを行う。
