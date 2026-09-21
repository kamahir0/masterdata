# Table Editor v1 実装とself-review

2026-09-13のimplementation work packageの記録。確定Candidateに対するfinal verificationは別passで行う。

## Scope

Approved [Table Editor](../gui/table-editor/spec.md)、[Schema Migration](../specs/schema-migration.md)、
[GUI app shell](../gui/app-shell.md)を対象とする。coreのRenameField / DropField、applicationのPlan保持と復旧確認、
Tauri command、ReactのTable表示・入力・Plan / Diff / Apply・既存Data Editorとの連携を実装した。

## Specification Conformance

Pass。Add / Rename / Dropはshared coreへ委譲し、RenameはMessagePack keyと対象外sourceを保持する。
Dropのkey依存は拒否し、Planとは別のdestructive execution authorizationを必要とする。
Applyはhostが保持したPlanを使用し、既存transactionのconfig / membership / exact bytes検査を継承する。
affected dirty / saving bufferはApplyを止め、unrelated dirty bufferは保持する。
Recovery RequiredではSave / Overwrite / Create / Apply / Buildを止め、hostが完全な既知OLDまたはNEW source setを確認してから解除する。

## Tests and Regression Evidence

- core `migration_field_mutation`: 4件。quoted name、key reference、literal content、コメント保持、依存拒否、compact recordの削除。
- application `table_authoring`: 5件。readonly Plan、stale拒否、destructive authorization、rollback / recovery、旧token無効化、ulong initializer。
- Tauri adapter: schema snapshotとstructured preflight diagnosticを確認。
- React: 全28件成功。新規Table Editor 5件とApp連携2件で入力変更時のPlan破棄、Drop確認、dirty buffer、再読込、復旧gateを確認。
- `cargo xtask check-all` 成功（fmt / clippy / Rust test / frontend check / native GUI test / integration smokeを含む）。
- `cargo xtask check-rationale` 成功。
- macOS Chrome、mock Tauri transportで1440×960 / 1024×768のschema表示、Rename入力、Plan / Diff、Applyへのfocus移動を確認。
- 既存MessagePack dependency warningあり。Windows / Linuxの実機GUI操作は未検証。

## Rationale Freshness

Fresh。source patchは意味的に期待する全documentと独立比較し、closure外の意図しない変更も拒否する。
literal scalarのコメント風文字列をdataとして扱う既存scannerの理由は引き続き有効。
復旧解除はparse成功だけではmixed source setを検出できないため、完全snapshot比較を保持する。
Tauriのsession lockはmutation終了まで保持し、Applyと他のGUI mutationの競合を防ぐ。
frontendは再読込前にaffected clean snapshotを退避し、obsolete schemaに対する編集を防ぐ。

## Evidence Integrity

- Requirement references: Table Editor / MIGRATION / GUI shellのcanonical ownerを確認。
- ADR/RFC references: ADR-0001 / ADR-0002の境界を維持。
- Regression test references: 上記focused testsと既存migration transaction testsを確認。
- Benchmark/external references: 性能改善の主張なし。browser確認はmock transportでありnative end-to-endの証明ではない。

## Architecture

boundary violationなし。frontendでYAML解釈やfilesystem transactionを実装せず、Tauriはapplication sessionを呼ぶ。
initializerはJSON value textをhostでparseして既存core valueへ変換し、JavaScriptの数値丸めを避ける。
Build / Publish / GitはMigration成功から自動実行しない。

## Findings

None identified（self-reviewで未解消のBlocking / Specification Gapなし）。

## Verdict

Ready to merge: Yes（implementation self-review）。spec statusは変更せず、Objective完了はfinal verificationへ委ねる。
