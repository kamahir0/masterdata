# Current Objective

## Role

この文書はHuman-selectedなcurrent priorityとDONE boundaryのownerである。observable semanticsはcanonical specification、resume地点は[Development State](execution-state.md)、implementation realityはcode / tests / Gitで確認する。

## Objective

**Unified Table EditorのColumn / Record Row drag挿入線を整え、移動先の境界を一意に認識できるようにする。**

## Completion slices

- 現行実装とMasterData-Legacyを比較し、挿入線の位置・範囲を仕様へ反映する。
- Column / Record Rowの挿入線を修正し、既存mutation、history、focus、sticky / virtualizationを維持する。
- focused regression、画面確認、repository check、required CIで検証する。

## Canonical requirements

- [Unified Table Editor](gui/table-editor/spec.md) `GUI-UNIFIED-008`
- [Grid Authoring](gui/data-editor/grid-authoring.md) `GUI-GRID-007`

## Explicit non-scope

- Array item drag、shared source mutation、Save、Search / Filter / Sort、visual themeの再設計。
