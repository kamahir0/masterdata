# Current Objective

## Role

この文書はHuman-selectedなcurrent priorityとDONE boundaryのownerである。observable semanticsはcanonical specification、resume地点は[Development State](execution-state.md)、implementation realityはcode / tests / Gitで確認する。

## Objective

**Unified Table Editorを日常的に軽快に使える編集面へ仕上げる。通常の編集状態でgridの位置を動かさず、Table文脈に沿ったSave / Undoと、短い直接操作を整える。**

## Completion slices

- Desktop操作とcurrent implementationからDaily Table authoringのfriction inventoryを作り、既存の安全契約とGUI仕様を照合する。
- 通常のdirty / schema draft / diagnostic / validation stateでgridを動かさない編集面と、文脈に沿った高頻度操作を実装する。
- Save / UndoのTable文脈を設計し、必要なHuman gateを解消した範囲でshared application / GUIへ適用する。
- focused regression、repository check、DesktopのGolden Pathで検証する。

## Canonical requirements

- [Unified Table Editor](gui/table-editor/spec.md)、[Data Editor](gui/data-editor/spec.md)、[Grid Authoring](gui/data-editor/grid-authoring.md)、[Field Declaration Mutation](specs/field-declaration-mutation.md)、[Authoring Batch](specs/authoring-batch.md)

## Explicit non-scope

- Source scalar semantics、MasterMemory binary format、Build / Publish semantics、Settings / Delivery全面再設計、Explorerのdomain tree化、visual theme全面刷新、安全契約の弱体化。
