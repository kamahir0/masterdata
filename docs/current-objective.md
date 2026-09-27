# Current Objective

## Role

この文書はHuman-selectedなcurrent priorityとDONE boundaryのownerである。observable semanticsはcanonical specification、resume地点は[Development State](execution-state.md)、implementation realityはcode / tests / Gitで確認する。

## Objective

**Desktop GUIの日常的なTable authoringを、保存fileの分割方法に左右されない単一の表面へ再設計する。ColumnとRowを直接操作できるようにし、通常成功時のschema変更手順を短くしながらshared Rustの安全契約を維持する。**

## Completion slices

- 現在のGUI、shared Rust、Legacy版の操作モデルを調べ、Table単位のtarget interactionと必要なApplication/Core APIを決める。
- inline recordsと分離Data sourceの両方を、同じTable編集面で扱う。複数record sourceは明示的に切り替える。
- column追加、inline rename、type / Nullable / Array変更、row追加、cell / complex value編集、TSV paste、Undo/Redo、Saveを一連の操作として成立させる。
- 通常成功時のschema変更をPlan / Diff / Applyの必須手順から外し、衝突・破壊・失敗・復旧時の安全な判断導線を維持する。
- 大量recordのbounded rendering、keyboard / focus / selection / Problems移動を保ち、focused test、repository check、Desktop実操作で検証する。

## Canonical requirements

- [GUI app shell](gui/app-shell.md)、[Explorer](gui/explorer/spec.md)、[Table Editor](gui/table-editor/spec.md)、[Data Editor](gui/data-editor/spec.md)、[Grid Authoring](gui/data-editor/grid-authoring.md)
- [Schema Migration](specs/schema-migration.md)、[Field Declaration Mutation](specs/field-declaration-mutation.md)、[Source Record Edit](specs/source-edit.md)、[Authoring Batch](specs/authoring-batch.md)、[Source Creation](specs/source-creation.md)

## Explicit non-scope

- YAML保存形式、Table identity、MasterMemory binary formatの変更。
- source-preserving rewrite、lost-update protection、dirty protection、rollback / recoveryの弱体化。
- Explorerをdomain treeにする変更、frontend独自のYAML / domain mutation、Build / Publish / Gitの暗黙実行。
