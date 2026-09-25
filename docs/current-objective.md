# Current Objective

## Role

この文書はHuman-selectedなcurrent priorityとDONE boundaryのownerである。observable semanticsはcanonical specification、resume地点は[Development State](execution-state.md)、implementation realityはcode / tests / Gitで確認する。

## Objective

**Table schema YAMLにrecordsを同居させる1ファイル形式を追加し、従来の分離形式と共存させる。Data編集画面からTable schemaも編集できる一体的な編集体験を提供する。**

## Completion slices

- source formatと混在時のTable解決、record編集、Migrationの挙動をApproved仕様へ反映する。
- shared Rust core/applicationで1ファイル形式の読み込み、検証、作成、source-preserving保存、Migrationを実装する。
- GUIで1ファイル形式の作成・record編集と、Data編集画面からのschema編集を提供する。
- focused regression tests、GUI build、実操作、repository checks、fresh review、required remote CI reconciliationを完了する。

## Canonical requirements

- [Table / Keys](specs/table-and-keys.md) — `SCHEMA-TABLE-001`
- [Source Record Edit](specs/source-edit.md) — `SOURCE-EDIT-001`, `SOURCE-EDIT-002`
- [Source Creation](specs/source-creation.md) — `SOURCE-CREATE-004`, `SOURCE-CREATE-005`
- [Schema Migration](specs/schema-migration.md) — `MIGRATION-014`, `MIGRATION-015`
- [Data Editor](gui/data-editor/spec.md) — `GUI-DATA-LAYOUT-001`
- [Table Editor](gui/table-editor/spec.md) — `GUI-TABLE-LAYOUT-001`

## Explicit non-scope

- 既存sourceの自動変換、既存recordのschema fileへの移動、public APIのversion保守。
- YAMLとGit以外をsource authorityとする変更。
