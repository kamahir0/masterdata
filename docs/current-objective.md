# Current Objective

## Role

この文書はHuman-selectedなcurrent priorityとDONE boundaryのownerである。observable semanticsはcanonical specification、resume地点は[Development State](execution-state.md)、implementation realityはcode / tests / Gitで確認する。

## Objective

**設定保存scopeをProject Settings / Project Local State / Application User Settings・UI Stateの3区分として確定し、Color Themeの永続化仕様をProject非依存のApplication storage authorityへ修正する。**

## Completion slices

- `PROJECT-CONFIG-007`で3つのpersistence scopeを明確化する。
- Project Local Stateのdefault namespaceを`.masterdata/**`、Project非依存Application User SettingsのauthorityをOS標準per-user application data/config領域として定義する。
- `GUI-THEME-002` / `GUI-THEME-004` / Color Theme architecture boundaryを`PROJECT-CONFIG-007`へ整合させる。
- current WebView `localStorage`実装との差分をcanonical statusへ反映し、実装済みと誤表示しない。
- specification changeをreviewし、canonical specificationへ適用する。

## Canonical requirements

- [Project layout](specs/project-layout.md) — `PROJECT-CONFIG-007`
- [Color Theme](gui/color-theme/spec.md) — `GUI-THEME-002`, `GUI-THEME-004`

## Explicit non-scope

- Application preference storageの実装変更。
- legacy `localStorage`からの実migration実装。
- Project Local State / Application User Settingsのexact file名、serialization format。
