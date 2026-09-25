# GUI仕様: Color Theme

Status: Approved

## 目的

Desktop applicationの表示テーマとしてLight / Dark / Systemの3種類を提供する。

Theme preferenceはmasterdata Projectのsemantic stateではなく、利用者個人のapplication preferenceとして扱う。Project、CLI、Build、Validation、Publishの結果へ影響してはならない。

## 用語

`ThemePreference` は利用者が選択した設定値を表す。

```text
system
light
dark
```

`EffectiveTheme` は実際に描画へ適用するthemeを表す。

```text
light
dark
```

`system` 自体は描画themeではなく、OS preferenceから`EffectiveTheme`を決定する指定である。

## 規範要件

### GUI-THEME-001

Desktopは以下の3種類のTheme preferenceを提供しなければならない（MUST）。

- `System`
- `Light`
- `Dark`

初期値は`System`とする（MUST）。

`System`ではOSのlight / dark appearance preferenceに従って`EffectiveTheme`を決定しなければならない（MUST）。

OS preferenceを取得できない場合は`Light`をfallbackとする（MUST）。

### GUI-THEME-002

Theme preferenceは[Project layout](../../specs/project-layout.md)の`PROJECT-CONFIG-007`が定義する
Project非依存のApplication User Settings / UI Stateとして、OS標準のper-user application data / config
領域へ永続化しなければならない（MUST）。

Theme preferenceを以下へ保存してはならない（MUST NOT）。

- `masterdata.toml`
- Project配下のYAML
- `.masterdata/`
- その他Project repositoryへ含まれるfile
- WebView/browser storageをdurable canonical persistence authorityとするstorage

legacy WebView storageにTheme preferenceが存在する場合、one-time migration sourceとして読み取ってもよい
（MAY）。migration後のcanonical valueはapplication-owned user-local storageから取得しなければならない
（MUST）。

Theme preferenceの変更によってProject dirty state、Project Settings dirty state、Save All、Project close guardを発生させてはならない（MUST NOT）。

Projectを切り替えてもTheme preferenceを維持しなければならない（MUST）。

CLIはTheme preferenceを読み書きする必要を持たない。

### GUI-THEME-003

利用者がTheme preferenceを変更した場合、applicationを再起動せず即座に表示へ反映しなければならない（MUST）。

`Light`または`Dark`が明示選択されている間、OS appearanceの変更によってapplication themeを変更してはならない（MUST NOT）。

`System`が選択されている間は、application実行中にOS appearanceが変更された場合もthemeを追従しなければならない（MUST）。

Theme preferenceの変更によって現在のProject、selection、editor buffer、dirty state、dialog等の作業状態を失ってはならない（MUST NOT）。

### GUI-THEME-004

保存済みTheme preferenceはcanonical Application preference storageからapplication起動時に復元しなければならない（MUST）。

可能な限り初回content描画より前に`EffectiveTheme`を決定し、起動時にLight themeが一瞬表示されてからDarkへ切り替わる等の不要なtheme flashを避けなければならない（SHOULD）。

保存値が存在しない場合は`System`として扱う。

未知または不正な保存値は`System`として扱わなければならない（MUST）。

### GUI-THEME-005

Light / Darkは単純なbackground colorの反転ではなく、application全体で一貫したsemantic color tokenによって表現しなければならない（MUST）。

少なくとも以下はLight / Dark双方で識別可能でなければならない。

- application background
- panel / elevated surface
- text / secondary text
- border / separator
- selected item
- hover / focus
- input
- disabled state
- diagnostics
- Diff
- destructive action
- success / warning / error state

component固有のhard-coded colorによってDark themeで可読性を失ってはならない（MUST NOT）。

### GUI-THEME-006

Light / Dark双方でkeyboard focusを視覚的に識別できなければならない（MUST）。

状態を色だけで表現してはならない（MUST NOT）。

既存のdiagnostic severity、dirty state、selection等の意味をtheme変更によって変えてはならない（MUST NOT）。

OSのhigh contrast / forced colors対応は本仕様の必須範囲には含めない。

### GUI-THEME-007

Theme preferenceはProject Settingsとは分離されたApplication Settingsから変更できなければならない（MUST）。


control上では現在の選択値として以下を明示する。

```text
Appearance
  System
  Light
  Dark
```

`System`選択時に現在解決されている`EffectiveTheme`を内部的に保持してよいが、保存値を自動的に`Light`または`Dark`へ置き換えてはならない（MUST NOT）。

## Application state model

概念上のstateは以下とする。

```text
ApplicationPreferences
└── theme: ThemePreference
        ├── system
        ├── light
        └── dark
```

theme resolution:

```text
theme == light
    -> EffectiveTheme::Light

theme == dark
    -> EffectiveTheme::Dark

theme == system
    -> OS prefers dark
        ? EffectiveTheme::Dark
        : EffectiveTheme::Light
```

`EffectiveTheme`はderived stateであり、永続化する必要はない。

## Architecture boundary

Theme preferenceとtheme resolutionはGUI application concernとする。

`masterdata-core`へTheme概念を追加してはならない（MUST NOT）。

`masterdata-app`のProject/application serviceへTheme semanticsを追加する必要はない。

永続化はDesktop application hostのApplication preference storageが所有する。React/WebView層は
WebView storageをdurable canonical authorityとして所有してはならない（MUST NOT）。exact native API、
file名、serialization formatは`PROJECT-CONFIG-007`のboundary内でimplementationへ委ねる。

React componentは個別にOS preferenceを問い合わせず、application-levelで解決された共通の`EffectiveTheme`を利用しなければならない（SHOULD）。

## Scope外

以下は本仕様では定義しない。

- ProjectごとのTheme override
- custom theme
- user-defined color palette
- syntax highlighting themeの独立選択
- OS high contrast theme
- CLI color scheme
- Project共有theme

将来Project単位のappearance preferenceが必要になった場合は、`masterdata.toml`ではなく
`PROJECT-CONFIG-007`のProject Local Stateとして`.masterdata/**`配下へ保持する方向で別途仕様化する。

## 受け入れ証拠

少なくとも以下を検証する。

1. 保存値なしでSystemとして起動する。
2. Lightを選択すると即時Lightになり、再起動後もLightになる。
3. Darkを選択すると即時Darkになり、再起動後もDarkになる。
4. System選択中にOSをLight→Darkへ変更すると実行中に追従する。
5. Light / Dark明示選択中はOS変更へ追従しない。
6. Project切替後もthemeが維持される。
7. theme変更でProject/YAML/Project Settingsがdirtyにならない。
8. 不正な保存値で起動してもSystemへfallbackする。
9. Light / Dark双方で主要screen、dialog、form、Explorer、editor、diagnostic、Diffが利用可能である。
10. theme変更前後でeditor bufferやselectionが失われない。
11. canonical Theme preferenceがProject tree、`.masterdata/**`、WebView storageではなくOS側のapplication-owned user-local storageへ保持される。
