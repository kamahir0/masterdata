# Current Objective

## Role

この文書はHuman-selectedなcurrent priorityとDONE boundaryのownerである。observable semanticsはcanonical specification、resume地点は[Development State](execution-state.md)、implementation realityはcode / tests / Gitで確認する。

## Objective

**Advanced Authoringをproduction-readyにし、保存済みMasterdata snapshotから安全にderived informationを作るComputed Viewを、Git-reviewableなdefinition、shared Rust semantics、Desktop Table Overview/query workflow、schema evolution safetyまで一貫して利用できる状態へ到達する。**

## Completion slices

- [Computed View仕様](specs/computed-view.md)に従う`kind: view` persisted definitionとbounded typed scalar expression。
- shared Rust parser / type checker / deterministic evaluator、null・invalid・arithmetic diagnostics。
- source-preserving view create/edit/remove、stale/lost-update protection、RenameField追随とDropField fail-closed。
- 保存済みOverviewへread-only computed columnsを表示し、既存Authoring Queryのsupported search/filter/sortへ接続する。
- view definitionをMasterMemory schema、generated C#、binary、artifact receipt、runtime fieldへ混入させない。
- focused regressions、fresh review、exact Candidate、required remote CI reconciliation。

## Explicit non-scope

- generated C# computed property、MasterMemory binary field、Unity runtime evaluator。
- embedded scripting、filesystem/network/environment side effect。
- aggregate、group-by、arbitrary join、cross-project query、recursive Reference traversal。
- persistent stable member ID、rename lineage、released compatibility identity、external wire compatibility。
- computed resultのsource materialization、Data Editorでのcomputed cell編集、Web / Browser / Native Host。

## Audit

2026-09-22 JST、Released Compatibility v1完了後のHuman priorityとしてAdvanced Authoringを開始した。仕様変更0027はAgent-autonomous review/applicationにより[Computed View仕様](specs/computed-view.md)へ適用済み。P5 computed viewはauthoring-only projectionとして扱い、既存runtime artifact contractを変更しない。
