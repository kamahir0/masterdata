# Current Objective

## Role

この文書はHuman-selectedなcurrent priorityとDONE boundaryのownerである。observable semanticsはcanonical specification、resume地点は[Development State](execution-state.md)、implementation realityはcode / tests / Gitで確認する。

## Objective

**Unified Table EditorのComplex Value編集を、直接・可逆なauthoringへ仕上げる。Array / Nullable / Enum / Flags / Custom Typeの意味ある操作をlocal bufferへ反映し、操作単位でUndo/Redoできるようにする。**

## Completion slices

- DesktopでComplex Valueの現行操作を観察し、Approved仕様とのfriction inventoryを作る。
- complex session全体のApply/Cancel依存を解消し、意味あるcontrol操作ごとのbuffer反映とfile-local Undo/Redoを実装する。nested text入力は確定単位で扱う。
- unknown / invalid source value、source-local lifecycle、keyboard/focus、nested Problems navigation、grid geometryを維持する。
- focused regression、Desktop Golden Paths、repository check、required CIで検証する。

## Canonical requirements

- [Unified Table Editor](gui/table-editor/spec.md)、[Data Editor](gui/data-editor/spec.md)、[Grid Authoring](gui/data-editor/grid-authoring.md)、[Record Mutation](gui/data-editor/record-mutation.md)

## Explicit non-scope

- 0044 Save model、source scalar semantics、schema header、Migration、Search / Filter / Sort、Row配置、clipboard codec、Explorer、Source Creation、Settings / Delivery、visual themeの再設計。
