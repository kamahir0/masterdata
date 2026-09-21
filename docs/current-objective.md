# Current Objective

## Role

この文書はHuman-selectedなcurrent priorityとDONE boundaryのownerである。observable semanticsはcanonical specification、resume地点は[Development State](execution-state.md)、implementation realityはcode / tests / Gitで確認する。

## Objective

**Schema Evolution & Migrationをproduction-readyにし、日常のschema/type evolutionをReference・compatibility・Desktop authoringと一貫したshared semanticsで安全にPlan / Preview / Applyできる状態へ到達する。**

## Completion slices

### Safe schema evolution

- Schema Migration / Type Migrationの既存safe operationを基盤に、Reference dependencyを含むrename等の安全に自動追随できるケースをsource-preservingに処理する。
- destructive / ambiguous / conversion-policy-requiredなchangeは推測せずfail closedし、既存authorization / rollback / Recovery Required contractを維持する。
- stable Field ID、rename lineage、path identity等を導入せず、current logical symbolsとexplicit migration intentをauthorityとして使う。

### Impact and authoring workflow

- Migration Plan / Diffから、Reference dependencyやgenerated API / released compatibility impactを利用者が理解できるようshared semanticsを接続する。
- Table Editor / Type Editorはshared application/core operationを使い、frontendへdependency resolution、compatibility classification、YAML rewriteを複製しない。
- routine schema evolutionがraw YAML手編集へ不必要に戻らず、Plan → review → Apply → refreshed workspaceまで完結する。

### Coverage and product hardening

- Objective内で見つかるmigration coverage gapのうち、既存Approved semanticsから一意に決められるsafe/additive operationは同一runでspecify / implement / verifyする。
- arbitrary data conversion、stable release identity、external wire compatibility等のmaterial product decisionが必要な領域はHuman gateへ戻す。
- focused regression、fresh review、repository checks、exact Candidate、required remote CI reconciliationまで完了する。

## Explicit non-scope

- arbitrary migration scripting / SQL-like language。
- implicit AI-generated data conversionや推測によるreplacement。
- persistent stable Table / Field / Type / Enum / Reference ID、rename lineage、tombstone。
- cross-schema MasterMemory binary compatibility guarantee、external save/network/database compatibility engine。
- semantic-version enforcement、automatic release/tag/publish。
- P5 expression / computed / programmable view。
- Web / Browser product surfaceの再導入。

## Audit

2026-09-22 JST、Humanは開発速度向上のためCurrent Objectiveを細かなfeature単位ではなく複数sliceを含む大きなproduct outcomeで切り、sub-feature / StageごとにHumanへ戻らず本物のHuman gateまたはObjective completionまで自律実行する方針を選択した。Released Compatibility v1完了後の次ObjectiveとしてSchema Evolution & Migrationのproduction-ready化を開始する。
