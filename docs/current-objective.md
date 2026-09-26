# Current Objective

## Role

この文書はHuman-selectedなcurrent priorityとDONE boundaryのownerである。observable semanticsはcanonical specification、resume地点は[Development State](execution-state.md)、implementation realityはcode / tests / Gitで確認する。

## Objective

**Desktop GUIの日常的なMasterData編集を、source fileを選ぶExplorerと対象を直接操作する編集面へ再設計する。通常操作は短く、検証・影響確認・復旧は必要な場面だけに示し、shared Rustの安全契約を維持する。**

## Completion slices

- Data Editorをデータ中心の画面へ整理し、大量recordでも選択・編集・keyboard操作・Problems移動が成立するgridへする。
- Explorerのfile文脈から短い手順で有効なsourceを作り、作成後に編集を開始できるようにする。
- Table schemaとrecordの編集を同じ画面の対象から開始でき、Migrationの安全なPlan / Applyと影響確認へ接続する。
- Complex Value / Type編集の繰り返し操作と常設情報を減らし、keyboardと異常時の導線を保持する。
- related specification changes、focused regression evidence、repository checks、Desktop実操作での確認を完了する。

## Canonical requirements

- [GUI app shell](gui/app-shell.md)、[Explorer](gui/explorer/spec.md)、[Source Creation](gui/source-creation/spec.md)
- [Data Editor](gui/data-editor/spec.md)、[Grid Authoring](gui/data-editor/grid-authoring.md)、[Table Editor](gui/table-editor/spec.md)、[Type Editor](gui/type-editor/spec.md)
- [Source Creation](specs/source-creation.md)、[Schema Migration](specs/schema-migration.md)、[Authoring Batch](specs/authoring-batch.md)

## Explicit non-scope

- YAML source format / domain identity / MasterMemory binary formatの変更。
- source-preserving rewrite、lost-update protection、rollback / recoveryの弱体化。
- Project全体の新しいdomain tree、source root外のfile探索、GUI独自のYAML/domain処理。
- Publish / Buildのdomain contract変更、Programmable View、Web版GUI。
