# 仕様変更 0041: Unified Table Editor

Status: Applied

## Why / adopted decision

Desktop Table authoringをinline / 分離Data sourceで共通のTable面へ統合し、column headerの直接操作と通常時の安全な自動commitを採用した。Explorerはsource file treeを維持する。field宣言変更は値を暗黙変換しない追加operationとした。

## Affected Specifications

- `docs/gui/table-editor/spec.md` — `GUI-UNIFIED-001..005`
- `docs/gui/data-editor/spec.md`、`docs/gui/explorer/spec.md` — record setとsource file navigation
- `docs/specs/field-declaration-mutation.md` — `FIELD-DECL-001..005`
- `docs/specs/schema-migration.md` — v1と追加operationの境界

## Approval provenance

Human-selected Objective内の非breakingな変更として、仕様reviewでBlockingなし、Human gateなしを確認しagent-autonomous approvalを適用した。canonical applicationとimplementationは本artifactを含む変更のGit履歴で追跡する。
