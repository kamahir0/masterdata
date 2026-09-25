# 仕様変更: GUI Color Theme

Status: Applied

## Affected Specifications

- `docs/gui/color-theme/spec.md` (New canonical specification: `GUI-THEME-001` .. `GUI-THEME-007`)
- `docs/gui/app-shell.md` (`GUI-SHELL-LAYOUT-002`, `GUI-SHELL-STATE-001`, Open Questions)

## 根拠と分類（Source Evidence and Classification）

- Human Requirement: Desktop applicationの表示テーマとしてLight / Dark / Systemの3種類を提供する。
- Human Requirement: 初期値はSystemとし、OSのlight / dark appearance preferenceに従ってEffectiveTheme（lightまたはdark）を決定する。OS preferenceを取得できない場合はLightをfallbackとする。
- Human Requirement: Theme preferenceはProject非依存のuser-local Application preferenceとして永続化する。masterdata.tomlやProject配下のファイルへ保存してはならず、Project dirty state等を発生させない。Project切替後も維持する。CLIは読み書き不要。
- Human Requirement: Theme preference変更時はアプリ再起動不要で即時反映する。明示選択中はOS変更を無視し、System選択中はOS変更に追従する。変更によって作業状態を失わない。
- Human Requirement: アプリ起動時に保存値を復元し、不要なtheme flashを避ける。未知または不正な値はSystemとして扱う。
- Human Requirement: Light / Darkは一貫したsemantic color tokenで表現し、主要要素が識別可能であること。Dark themeで可読性を失わないこと。
- Human Requirement: keyboard focusの視覚的識別、状態を色だけで表現しない、既存の意味を変えないこと。
- Human Requirement: Project Settingsとは分離されたApplication Settingsから変更でき、Appearance（System, Light, Dark）を明示すること。System選択時に解決されたEffectiveThemeを内部保持してよいが、保存値を自動でLight/Darkに置き換えないこと。
- Human Constraint: Theme preferenceはmasterdata Projectのsemantic stateではなく、CLI/Core/Project/Build/Validation/Publishの結果に影響してはならない。
- Agent Decision: 永続化キーは`masterdata.theme-preference.v1`とし、`window.localStorage`へ`system`、`light`、`dark`を保存する。理由は既存のRecent Projectsと同様にbrowser/Tauri共通のuser-local storageとして安全・決定的に扱えるため。
- Agent Decision: テーマ適用は`document.documentElement`に`data-theme="light"` / `data-theme="dark"` attributeを付与し、CSS変数を切り替えるとともに、Ant Designの`ConfigProvider`へ`theme.defaultAlgorithm`または`theme.darkAlgorithm`を渡す。理由はHTMLレベルでの即時反映とAnt Designコンポーネント全体へのテーマ適用を両立し、初回描画時のtheme flashを防ぐため。
- Agent Decision: Application Settingsの導線として、Project未選択（Welcome）画面およびProject選択時の上部バー（Titlebar）から到達できるApplication Settingsダイアログを提供する。Projectが開いている間はTitlebarのコマンドバーから、開いていない間もTitlebarやWelcome画面から開ける。理由はProject有無にかかわらず利用者が即座にpreferenceを変更可能にするため。

## 確定事項（Confirmed Decisions）

- `ThemePreference`は`system` | `light` | `dark`。
- `EffectiveTheme`は`light` | `dark`。
- `masterdata-core`や`masterdata-app`のdomain/project serviceにはTheme概念を追加しない。GUI application concernに閉じる。
- Project dirty state、Save All、Close guardへ影響を与えない。

## 提案する差分（Proposed Delta）

### 新規仕様: `docs/gui/color-theme/spec.md`

- `GUI-THEME-001: Theme preference`
  Desktopは`System`、`Light`、`Dark`の3種類のTheme preferenceを提供する（MUST）。初期値は`System`（MUST）。`System`ではOSのlight/dark preferenceから`EffectiveTheme`を決定する（MUST）。OS preferenceを取得できない場合は`Light`をfallbackとする（MUST）。
- `GUI-THEME-002: Persistence scope`
  Theme preferenceはProject非依存のuser-local Application preferenceとして永続化する（MUST）。`masterdata.toml`、Project配下のYAML、`.masterdata/`等へ保存してはならない（MUST NOT）。Project dirty state等を発生させてはならない（MUST NOT）。Project切替後も維持する（MUST）。CLIはTheme preferenceを読み書きしない。
- `GUI-THEME-003: Runtime application`
  利用者がTheme preferenceを変更した場合、再起動せず即座に反映する（MUST）。`Light`/`Dark`明示選択中はOS変更でthemeを変更してはならない（MUST NOT）。`System`選択中は実行中のOS appearance変更に追従する（MUST）。変更で作業状態を失ってはならない（MUST NOT）。
- `GUI-THEME-004: Startup`
  保存済みTheme preferenceは起動時に復元する（MUST）。可能な限り初回content描画前に`EffectiveTheme`を決定しtheme flashを避ける（SHOULD）。保存値なしまたは不正値は`System`として扱う（MUST）。
- `GUI-THEME-005: Visual semantics`
  一貫したsemantic color tokenによって表現する（MUST）。主要UI要素（bg, panel, text, border, selection, hover/focus, input, disabled, diagnostics, Diff, destructive, success/warning/error）が双方で識別可能であること（MUST）。
- `GUI-THEME-006: Accessibility`
  Light/Dark双方でkeyboard focusを視覚的に識別可能とする（MUST）。状態を色だけで表現しない（MUST NOT）。既存の意味を変えない（MUST NOT）。
- `GUI-THEME-007: Settings surface`
  Project Settingsとは分離されたApplication Settingsから変更できる（MUST）。Appearance (System / Light / Dark) を明示する。保存値を自動的にLight/Darkへ置き換えない（MUST NOT）。

### 既存仕様の更新: `docs/gui/app-shell.md`

- Open Questionsからtheme customizationを除去し、`docs/gui/color-theme/spec.md`へのポインタを設ける。
- Application Settingsへの到達性について言及を追加する。

## 互換性（Compatibility）

- Projectファイル形式（YAML/TOML）やCLI、生成コード、Binaryへの影響は一切ない（完全な互換性維持）。
- user-local storageの追加のみであり、既存のRecent Projects等のデータと競合しない。

## 受け入れと実装への影響（Acceptance and Implementation Impact）

- 保存値なしでSystemとして起動すること。
- Light選択で即時Light、再起動後もLightになること。
- Dark選択で即時Dark、再起動後もDarkになること。
- System選択中にOS appearanceが切り替わると即時追従すること。
- Light/Dark明示選択中はOS変更を無視すること。
- Project切替後もthemeが維持されること。
- theme変更でProject/YAML/Project Settingsがdirtyにならないこと。
- 不正な保存値で起動してもSystemへfallbackすること。
- Light/Dark双方で主要screen、dialog、form、Explorer、editor、diagnostic、Diffが利用可能であること。
- theme変更前後でeditor bufferやselectionが失われないこと。

## 未解決事項（Open Questions）

None identified.

## レビュー（Review）

### Blocking Issues
None identified.

### Non-blocking Issues
None identified.

### Questions
None identified.

### Approved as Proposed
Yes.

### Autonomous approval eligibility
- Eligible: Yes
- Human gate: None
- Rationale: Humanによって提示された要件に完全に忠実であり、Core/CLI/Projectへの破壊的影響がなく、testableな受け入れ基準が定義されている。

### Review dimensions
- Intent fidelity: 提示された要件・用語・状態モデル・境界・スコープ外を完全に保持。
- Internal consistency: 要求仕様、受け入れ証拠、互換性がすべて整合。
- Cross-spec consistency: app-shell等の既存仕様との重複なし。
- Terminology: ThemePreference, EffectiveThemeの定義に合致。
- Normative strength: MUST/MUST NOT/SHOULDを原案通り保持。
- Testability: すべての受け入れ証拠が自動テスト可能。
- Backward compatibility: 完全互換。
- Unresolved ambiguity: なし。
- Implementation leakage: なし。
- Unrequested behavior: なし。
- Documentation ownership: `docs/gui/color-theme/spec.md`に集約。

## 承認記録（Approval Record）

- Approval mode: Agent-autonomous
- Basis: Human-selected Current ObjectiveおよびHuman提示のGUI仕様案
- Review result: Blockingなし、material ambiguityなし
- Canonical application: `docs/gui/color-theme/spec.md`および`docs/gui/app-shell.md`へ反映

