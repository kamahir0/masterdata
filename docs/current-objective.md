# Current Objective

## Role

この文書はHuman-selectedなcurrent priorityとDONE boundaryのownerである。observable semanticsはcanonical specification、resume地点は[Development State](execution-state.md)、implementation realityはcode / tests / Gitで確認する。

## Objective

**Desktop GUIのApplication User Settings / UI State（Theme preferenceおよびRecent Projects）について、WebView localStorageからOS標準のper-user application data / config領域をauthorityとするnative persistenceへ移行し、安全なmigrationを提供する。**

## Completion slices

- Tauri native storage backend（OS標準のper-user application data/config directory配下のJSON永続化）とTauri command boundaryの実装。
- Frontend（Theme preference および Recent Projects）のnative storage読み書きへの切り替え。
- legacy localStorageからの安全・idempotentなone-time migrationの実装。
- 起動時（初回描画前）のTheme解決・適用を非同期native storageと整合させ、theme flashを防止。
- 既存ThemeおよびRecent Projectsの挙動・UX・境界（Project dirty state影響なし、Project外・.masterdata外、Core/App影響なし等）の維持。
- focused regression tests（Rust unit tests, GUI component/integration tests）およびrepository checksの完了。

## Canonical requirements

- [Project layout](specs/project-layout.md) — `PROJECT-CONFIG-007`
- [Color Theme](gui/color-theme/spec.md) — `GUI-THEME-001`, `GUI-THEME-002`, `GUI-THEME-003`, `GUI-THEME-004`, `GUI-THEME-005`, `GUI-THEME-006`, `GUI-THEME-007`
- [Project Workflow](gui/project-workflow.md) — `GUI-PROJECT-002`

## Explicit non-scope

- ProjectごとのTheme override、custom theme、user-defined color palette。
- syntax highlighting themeの独立選択、OS high contrast theme、CLI color scheme、Project共有theme。
- `masterdata.toml`へのTheme追加、`.masterdata/**`へのApplication-wide preference保存。
- unrelatedなProject Local State persistence。
- generic settings frameworkの先行構築、`masterdata-core` / `masterdata-app`へのGUI preference semantics追加。

