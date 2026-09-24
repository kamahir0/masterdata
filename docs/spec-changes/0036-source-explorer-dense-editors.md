# 仕様変更: Source Explorerと連続編集画面

Status: Applied

## Affected Specifications

- `docs/gui/app-shell.md` `GUI-SHELL-NAV-001`
- `docs/gui/explorer/spec.md` `GUI-EXPLORER-001`, `GUI-EXPLORER-INT-001`
- `docs/gui/data-editor/spec.md` `GUI-DATA-LAYOUT-006`, `GUI-DATA-KEY-002`, 新規`GUI-DATA-LAYOUT-007`
- `docs/gui/table-editor/spec.md` 新規`GUI-TABLE-LAYOUT-005`
- `docs/gui/type-editor/spec.md` 新規`GUI-TYPE-LAYOUT-004`

## 根拠と分類（Source Evidence and Classification）

- Human Decision: 左ペインはsource folder配下の素直なfile treeに徹し、VS Code ExplorerをUIの基準とする。既存のTable / Type分類を左ペインへ残さない。
- Human Requirement: Legacy Editorの実挙動とkeyboard操作を踏まえて、Table等の編集画面を再設計し、合意方針を実装する。
- Observation: LegacyのData gridは固定行高で多数recordを同時に見られ、複合値を選択時に局所的なeditorへ展開する。現行GUIは複合値の入力を常時展開し、行高と情報量が増える。Legacyの一部キー挙動には欠点があり、そのままcontract化しない。
- Agent Decision: Project areaは上部Project menu、Table OverviewはSchema / Data headerの文脈操作から開く。source rootは複数設定を保ったままtree rootとして表示する。
- Agent Decision: Schema / Typeの既存Migration安全契約を維持し、行起点のactionから一時的なoperation / Plan / Diff面へ進む。

## 提案する差分（Proposed Delta）

- `GUI-SHELL-NAV-001`: 左ペインの常設navigationはconfigured source rootとそのfile / folder hierarchyのみ。Table / Type / Project areaの並列groupを置かない。Project Settings / Delivery / BuildはProject menu等の上部command surfaceから、Table Overviewは選択中のTable schema / Data文脈から到達する。既存のdirty buffer / selection保持とkeyboard到達性は継続。
- `GUI-EXPLORER-001`: source rootを見出しとする密なtree。folder展開、file選択、dirty/loading/error状態が行で識別できる。New artifact / New folder、Refresh、Collapse、source moveはExplorer文脈のtoolbar / row action / keyboardで到達できる。artifact生成、path mutation、folder inventoryは既存のshared application authorityを使う。
- `GUI-EXPLORER-INT-001`: file選択でtyped editorを開く。treeのArrow Up/Down/Left/Right、Home/End、Enter、F2相当のkeyboard操作を一貫して扱い、編集領域へfocusを移せる。
- `GUI-DATA-LAYOUT-007`: record gridの定常行高を複合値の内部要素数に依存させない。column headerにfield nameとread-onlyのtype/key情報を表示する。scalar値はcellを選択・edit時に入力し、array / struct等は定常時に要約、選択時に一時的なeditorで編集する。editorは対象cellの文脈を明示し、閉じても他cellの選択・dirty bufferを失わない。
- `GUI-DATA-KEY-002`: Arrowでcell selection、Enter/F2でedit開始、Tab/Shift+Tabで横移動、Enter/Shift+Enterで縦移動、Escapeで未確定の現在cell editを破棄する。complex editorをkeyboardで開いた時は最初の操作可能controlへfocusし、内部controlにもTabで到達できる。file Save shortcutは従来どおりactive fileに限定する。
- `GUI-TABLE-LAYOUT-005` / `GUI-TYPE-LAYOUT-004`: field/member listは選択radio列と常時並ぶAdd/Rename/Drop clusterを持たず、行またはlist文脈からactionを開始する。operation input、Plan / Diff / Applyは必要時に開く一時的な面へ置き、閉じればlistと起点focusへ戻る。Migrationのshared Plan、Diff、stale判定、dirty-file gate、destructive authorizationは既存Requirementを維持する。

## 互換性（Compatibility）

UI navigation / presentationの変更。source format、identity、CLI、Tauri command、shared domain semantics、保存・Migration contractは不変。以前のgroup navigationを前提にしたGUI操作の手順は変わるがpersisted dataの互換性に影響しない。

## 受け入れと実装への影響（Acceptance and Implementation Impact）

- 複数source root、空folder、invalid source、dirty stateをtreeで識別し、folder/pathをdomain identityとして解釈しない。
- Data gridで複合値を含む複数recordが一定行高で並び、mouse / keyboardでscalarと複合値の編集を続けられる。Escape後は該当cellの未確定値がdirty bufferへ混入しない。
- Schema / Typeの行からactionを開き、Plan / Diff / Applyまでkeyboardで到達できる。affected dirty fileとdestructive gateは有効。
- Project menuからProject area、Schema / Data文脈からTable Overviewへ到達し、画面移動のみでdirty bufferを破棄しない。

## 未解決事項（Open Questions）

None. Popover / drawerのlibrary component、正確なrow heightやiconは実装詳細。

## レビュー（Review）

Fresh review: Blocking Issues: None identified. Non-blocking Issues: exact icon / popover sizeは実装詳細。Questions: None identified. Approved as Proposed: Yes. Autonomous approval eligibility: Eligible: Yes; Human gate: None. Intent fidelity: Humanが指定したsource treeと編集密度に一致。Cross-spec consistency: source semantics、Data Save、Migration Plan / Diff / Applyを維持。Requirement IDはunique。Testability: tree、定常行高、keyboard、dirty gateを操作で確認可能。Backward compatibility: persisted data / CLI / domain semanticsは不変。Documentation ownership: navigationはapp shell / Explorer、編集挙動は各editor spec。

## 承認記録（Approval Record）

Approval mode: Agent-autonomous. Basis: Human-selected Current Objectiveと本turnの「その方針で実装」。Review result: Blockingなし、Human gateなし、non-breaking UI変更、success / failureの検証可能性あり。Canonical application: `docs/gui/app-shell.md`, `docs/gui/explorer/spec.md`, `docs/gui/data-editor/spec.md`, `docs/gui/table-editor/spec.md`, `docs/gui/type-editor/spec.md`。
