# Table Declaration Mutation仕様

Status: Approved

このownerは[Unified Table](../gui/table-editor/spec.md)の低頻度structural declaration authoringを定義する。[Table / Key](table-and-keys.md)、[Reference](index-and-reference.md)がdeclaration semanticsを所有する。Migration v1のoperation setを変更せず、[Migration safety](schema-migration.md)、[source preservation](source-edit.md)、[field declaration draft](field-declaration-mutation.md)と境界を分離する。承認記録は[仕様変更0054](../spec-changes/0054-table-declaration-authoring.md)。

## 規範要件

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

## 受け入れ証拠

source exact bytes、flow / CRLF / quote / inline / sibling preservation、Key order / Reference resolution、dirty guard / stale closure / binding / fault / destructive authorization、actual Desktop detailとfinal Candidate consumerを照合する。record-level invalid draftをProject-wide error-free gateへ変えない。
