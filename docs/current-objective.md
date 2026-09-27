# Current Objective

## Role

この文書はHuman-selectedなcurrent priorityとDONE boundaryのownerである。observable semanticsはcanonical specification、resume地点は[Development State](execution-state.md)、implementation realityはcode / tests / Gitで確認する。

## Objective

**Desktop編集画面の診断表示による意図しない高さの変化とProblemsの自動展開をなくし、Explorerのsource root直下を常時表示する。**

## Completion slices

- 編集中に診断が増減してもProblemsがユーザー操作なしに開閉せず、グリッド位置が移動しない。
- Explorerのsource root直下はCollapse操作やキーボード操作で隠れず、子フォルダの開閉とfile操作は維持する。
- GUI回帰テスト、repository check、Desktop表示で確認する。

## Canonical requirements

- [Data Editor](gui/data-editor/spec.md)、[Workspace Explorer](gui/explorer/spec.md)

## Explicit non-scope

- Source semantics、Save / Build、diagnostic内容、Explorerの子フォルダ構造の変更。
