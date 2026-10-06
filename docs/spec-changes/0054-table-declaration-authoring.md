# 仕様変更: Table declaration authoring

Status: Applied

## Affected Specifications

- 新規owner: `docs/specs/table-declaration-mutation.md`、`TABLE-DECL-001..006`。
- [Table / Key](../specs/table-and-keys.md)、[Reference](../specs/index-and-reference.md): 既存declarationの意味を再利用する。
- [Unified Table](../gui/table-editor/spec.md) `GUI-UNIFIED-004..005`: advanced detailの要求を具体化する。
- [Migration](../specs/schema-migration.md) `MIGRATION-002 / 009..011 / 016`: v1 operation setを増やさず、Plan / dirty / commit safetyを共有する。

## Source Evidence and Classification

Requirement: Human-selected Clean-room Rewriteはschema / keys / referencesのauthoring completenessを要求する。Approved `GUI-UNIFIED-004`はMessagePack key、Key、Referenceをstructural operationに分類し、Reference add/edit/removeとexplicit destructive authorizationを要求している。

Constraint: YAML + shared Rustがauthority。record values、occurrence identity、無関係bytesを変更せず、frontendへ型・Reference解決やwrite authorityを移さない。一般Git client、独立Overview、persistent advanced toolbarを増やさない。

Agent Decision: Table declaration authoringを独立operation familyとして定義する。Migration v1を拡張する案は`MIGRATION-002`と矛盾する。普通の可逆type draftへ混ぜる案は`GUI-UNIFIED-004`のPlan / authorization境界を失うため採用しない。source-preserving candidateと既存native source-set commit safetyの再利用は内部architecture choiceである。

## Proposed Delta

新規ownerは以下だけを所有する。declaration syntax、name、key capability、Reference cardinality / optionality / helper名、Build制約は既存ownerに従う。

### TABLE-DECL-001

Table declaration authoringは、existing fieldのMessagePack key変更、Primary Keyのordered component置換、Secondary Keyのadd/edit/remove、Referenceのadd/edit/removeを提供する（MUST）。logical Table identityとschema sourceをshared serviceが解決し、既存entryはcaptured sourceのexact occurrenceで特定する（MUST）。MessagePack key、generated indexNo、Reference target、pathを別のlogical identityの代用にしない（MUST NOT）。

### TABLE-DECL-002

mutationは対象schema sourceの必要なdeclaration spansだけを変更する（MUST）。record source bytes、inline records、fields / sibling declarationsの非対象presentationを保持し、既存flow syntaxの安全な局所変更と新規block mappingは`SOURCE-EDIT-018`に従う。安全なsource location・ownership・postconditionを証明できなければfail closedとする（MUST）。table全体をgeneric serializeしてはならない（MUST NOT）。

### TABLE-DECL-003

Planはsaved source / configのcaptured resolution closureから導出し、command、affected physical sources、before / after、resolved declarationとdiagnosticsを理解可能にする（MUST）。dirty source / dependencyがある場合、saved Planとの区別を表示し、affected write sourceには既存Save / Don't Save / Cancel guardを適用する。無関係draft / historyを破棄したり、semantic diagnosticsだけでdependent sourceをdirtyにしてはならない（MUST NOT）。

### TABLE-DECL-004

new / changed declarationとその変更で影響するKey / Reference resolutionはexisting canonical validatorで検証し、ambiguous / invalid declarationまたは解決不能なdependencyではcommitしない（MUST）。Project-wide error-freeやrecord-level key uniqueness / missing-reference diagnosticの解消をoperation成功の前提にしない（MUST NOT）。recordのvalue変換・自動repairを追加せず、Buildは既存strict validationに従う。追加のMessagePack key upper boundやcross-schema binary compatibilityを導入しない。

### TABLE-DECL-005

commitは明示的なPlan authorization後、actual current namespace / identity / bytesとcaptured closureのfresh checkで認可する（MUST）。stale Planはrejectし、silent re-plan / auto-applyを行わない（MUST NOT）。Reference / Secondary Key removalは削除対象と影響を確認したexplicit destructive authorizationを要求する（MUST）。Success / Conflict / Failure / Outcome Unknown / NotAttempted / Recovery Requiredとpartial physical-source base advancementは既存native write safety contractを共有し、Unknownをblind retryしない（MUST NOT）。committed diskをsource-local authoring Undoで巻き戻さない。

### TABLE-DECL-006

advanced Table detailは既存GUIのprogressive disclosureに従い、field MessagePack key、ordered Key components / uniqueness、Reference name / source fields / target / helper overrideを操作できる（MUST）。cardinality、optionality、effective C# helper名、diagnosticsはshared Rustのresolved presentationから示す（MUST）。詳細を開いたことだけでdirtyにせず、ordinary selectionのproject-wide workゼロ、bounded controls、single active editorを維持する。Frontendでdeclarationの意味を再構築しない（MUST NOT）。

## Compatibility

existing source syntax、identity、field/key/reference semantics、v1 CLI Migration operation set、generated C# / binary rulesを変更しない。既存declarationを変更する明示authoring入口だけを追加する。coherent artifact setの再Buildは既存contractどおりで、last successful receiptのPublishを現在sourceのvalidityで失効させない。

## Acceptance and Implementation Impact

- comment / quote / CRLF / flow / inline record / sibling bytesのexact preservationとunsafe failure時の不変。
- duplicate PK中のrecord occurrence保持、Key component order、nonunique / composite Referenceのshared resolution。
- add/edit/remove、duplicate Reference name、bad target、helper collision、dependent Key removal拒否、explicit destructive authorization。
- invalid unrelated source / record diagnostics、dirty guard、stale Plan / changed binding、write faultsとdraft保持。
- actual Desktop detail / focus / Cancel、generated C# compileとactual MasterMemory consumerをfinal Candidateで照合する。

新実装のspecific operation、bounded detail projection、native safety reuse、Ant Design contextual detailを対象とする。oracle inputは変更しない。新規ADRは不要で、ADR 0009のauthority / lifetimeを維持する。

## Open Questions / Approval Eligibility

None. Autonomous approval eligible: Yes。Human gate: None。
既存Approved GUI capabilityのObjective-local具体化で、persisted format / CLI互換性 / authority boundary / product scopeを変更せず、上記acceptanceを実行可能に切れる。

## Review / Approval Record

Review-spec: Blocking Issues / Non-blocking Issues / Questions: None identified。
Approved as Proposed: Yes。Eligible: Yes。Human gate: None。

Fresh challenge passはHuman intent、cross-spec consistency、normative strength、testability、backward compatibility、unresolved ambiguity、implementation leakage、scope、documentation ownershipを確認した。Migration v1のoperation setを変更せず、普通のtype draftとstructural authoringを分離する。declaration-time resolutionとrecord-level Build validationの区別は既存Save / validity分離を維持する。native安全境界と発生するdeclarationだけが新scopeであり、persisted schema / consumer contractを変更しない。

Approval mode: Agent-autonomous。Basis: Human-selected Clean-room RewriteとApproved GUI-UNIFIED-004..005。Review result: Approved as Proposed / Blockingなし / Human gateなし。Canonical application: TABLE-DECL-001..006をTable Declaration Mutation ownerへ適用。既存source / CLI / consumer semanticsは維持する。
