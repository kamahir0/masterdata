# Current Objective

## Role

この文書はHuman-selectedなcurrent priorityとDONE boundaryのownerである。observable semanticsはcanonical specification、resume地点は[Development State](execution-state.md)、implementation realityはcode / tests / Gitで確認する。

## Objective

**Desktop applicationの表示テーマとしてLight / Dark / Systemを提供し、Project非依存のApplication preferenceとして即時反映・永続化する。**

## Completion slices

- GUI Color Themeの規範要件（GUI-THEME-001〜007）をApproved仕様へ反映する。
- Desktop GUIでThemePreference（System / Light / Dark）の選択、OS preference追従、localStorage永続化、初回描画時の復元を実装する。
- Application Settings surfaceでAppearance（System / Light / Dark）の選択UIを提供する。
- Light / Dark双方のsemantic color tokenとAnt Design ConfigProvider連携を整え、可読性とアクセシビリティを確保する。
- focused regression tests、GUI build、実操作確認、repository checksを完了する。

## Canonical requirements

- [Color Theme](gui/color-theme/spec.md) — `GUI-THEME-001`, `GUI-THEME-002`, `GUI-THEME-003`, `GUI-THEME-004`, `GUI-THEME-005`, `GUI-THEME-006`, `GUI-THEME-007`
- [GUI app shell](gui/app-shell.md)

## Explicit non-scope

- ProjectごとのTheme override、custom theme、user-defined color palette。
- syntax highlighting themeの独立選択、OS high contrast theme、CLI color scheme、Project共有theme。

