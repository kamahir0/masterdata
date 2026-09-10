# Development State

Stage: correction-ready
Candidate: c3e668e6c2b10137435eb81e162a3681ea0e1a84

## Blocking findings

### [P1] closure判定に使用したsourceの変更をstale preflightが見逃す

- 対象: `crates/masterdata-core/src/migration.rs` の `source_inputs` 構築と、`crates/masterdata-core/src/migration_commit.rs` の `preflight_source_snapshot`。
- Authority: [Schema Migration仕様](specs/schema-migration.md)の `MIGRATION-006`、`MIGRATION-015`、`MIGRATION-016`、`MIGRATION-017`。
- 原因: `source_inputs` は変換前後の最終closureに含まれるfileだけを記録する。closure判定では他fileのtable/type identityも読んでいるが、それらのexact contentはcommit直前に検査されない。file pathのmembership比較だけではidentity変更を検出できない。
- 再現: `item` のschema/dataと `other-data.yaml`（`table: other`）を読みAddField `label: string`、initializer `Potion`をplanする。その後同じ `other-data.yaml` を `table: item`、`records: [{id: 99}]` に変更し、元のsnapshotとdry-runでcommitする。
- 実測: commitは `Success` を返し元のitem schema/dataを更新するが、変更されたfileのrecordには `label` がない。完全なNEW migrated source setにならず、stale planをmutation前に拒否するcontractに違反する。一時integration probeで `result.is_err()` が失敗することを確認した。
- 修正範囲: closureへの採否判定に使ったinputも含めてsnapshot依存を追跡し、必要なexact contentをmutation前に検査する。既存のunrelated diagnosticを許容する規則と、plan後のsource変更を拒否する規則を混同しない。`unrelated_source_change_does_not_block_commit` はこの区別に照らして再検証する。
- 必要なregression: 既存fileのtable identityが対象へ変わる場合をstaleとして拒否し、全sourceがcommit前のbytesのまま残ること。closure判断に使ったtype/schema入力についても追跡漏れを確認する。
- Rationale / evidence: preflightのWHYは目的として妥当だが、現行の追跡範囲ではその主張を満たせない。関連commentとregressionを同じcorrective work packageで整合させる。
- Non-scope: 新しいMigration operation、CLI/GUI wiring、public recovery format、Approved semantics変更、unrelated refactor。

## Human decision needed

None.
