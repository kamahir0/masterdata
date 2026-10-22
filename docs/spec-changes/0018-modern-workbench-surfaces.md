# 面と階層で整理するモダンworkbench

Status: Applied

Approval mode: Agent-autonomous
Objective / request basis: 2026-10-22 HumanのUI刷新・実装・検証・通常Git deliveryの委任。別pass reviewにBlockingなくorchestrationが承認。

## Basis

2026-10-22 Human Requirement: Desktop / Browser共通の主要領域を余白・背景階層・角丸で整理し、現代的なIDEの統一感と編集効率を両立する。コンパクトさ、緑accent、Ant Design / Motion、編集・選択・focus・scroll・Undo・dirtyの操作モデルを維持する。表のセルのカード化と無関係refactorは対象外。

参照は[VS Code Modern UI](https://code.visualstudio.com/blogs/2026/10/15/modern-ui)、[JetBrains Islands](https://blog.jetbrains.com/platform/2025/12/meet-the-islands-theme-the-new-default-look-for-jetbrains-ides/)。説明に加えVS Codeの新旧workbench・default / compact・pill / connected tabの画像、Islandsの全体像とsofter / tab / organizedのGIFの新旧frameを比較した。VS Codeは余白過多、pillによる内容との分断、status項目間隔、resize発見性をfeedbackから修正している。Islandsは細い溝と角丸の面でEditor / tool windowを分け、active tabを強調する。これらは設計根拠であり製品authorityではない。

## Agent Decision / behavior

- window全体を無彩色の低い背景、Explorerを補助面、editorを主要面とする。外周とpane間は小さい共通gap、主要面は共通radiusと控えめな輪郭を使う。表内の行・列・セル間隔は維持する。タイトルと全体toolbarは背景側へまとめ、file / mode / 表内操作の順に階層を付ける。
- 選択file tabはeditorと同じ背景で下側をつなげ、上辺の緑の目印・文字色で識別する。非選択は控えめなhover、keyboard focusは内側のoutlineで表す。preview italic、dirty dot、close、tab scroll / wrap、退出motionを保持する。複数行では選択面のcueを保持し、tab帯を高さ制限内でscrollする。
- Problemsはeditorの下の独立した補助面としてgapで区切る。内容を覆わず既存の開閉motionとjumpを保つ。nested / 設定 / 結果のDrawerにも共通の面・角丸を適用し、入力と既存のmask / closeの規則を保つ。
- resizeの溝を常時認識できる控えめなhandleにし、hover / focus / dragを緑で示す。既存のpointer・keyboard・double clickと幅保存を維持する。Explorer非表示では空の溝を残さず、復元で検索・展開・draftを保持する。
- radius・spacing・surface・borderのsemantic tokenをLight / DarkとAntDで共用する。狭幅では外周を縮め、paneの最小editor幅を確保する。追加の密度設定は設けない（既存のcompactを基本とするため）。

## Acceptance / validation

1. 同じsample・画面サイズで改修前後のスクリーンショットを比較する。1480×960と1000×680、Light / Darkで面の境界、文字、選択tab、focus、hoverが判別でき、主要操作が重ならず表の最初の行が300px未満に見える。Browserのより狭い幅でもtoolbarとmodeに到達できる。
2. Explorerの表示 / 非表示、pointer / keyboard resize、深い階層・長い名前・drag targetを確認する。表示変更でfocusやdraftを奪わず、表の実幅に追従する。
3. 9個以上のfile tabのscroll / wrapと切替、preview / dirty / close / keyboard / motionの回帰を確認する。選択tabの下に境界を引かず、editor面と同色にする。
4. 大きな表の縦横scroll・固定header・末尾・セル編集とUndo、nested / Problems併用でclippingやscrollのresetがない。補助面の高さは編集を圧迫し過ぎない。
5. Light / Darkのnestedと両設定、hover / focus / 選択 / drag / reduced motionを実画面と既存UI回帰で確認する。Desktop hostとBrowserの代表的な編集 / 保存を確認し、runtime別のBuild / Publish境界を保つ。

保存・形式・外部I/O・権限・認証は変更しない。実データに接触せずsampleの隔離copyで検証する。未決定事項: None。Human gate: None（選択済みObjective内の可逆な表示設計）。canonical ownerはDesktop UI-003、Browser WEB-002から共有する。

## Review

確定したProposedを別passでchallenge review。装飾による領域圧迫、wrapの上段tabとeditorの距離、panel開閉時のviewport再計測、角丸でfocus ringをclipする危険、Explorer hidden時の残gap、runtime固有操作の保持を検討した。wrapは同色と緑の目印で選択を保ち、focusは内側、Problemsはflex siblingで領域を確保し、compact gapを採用する。保存・sessionモデルは変更しない。Blocking Issues / Non-blocking Issues / Questions: None。Approved as Proposed: Yes。Autonomous approval eligible: Yes（依頼の表示改善内、観察可能なacceptanceあり、data safety / security boundary変更なし）。Human gate: None。

Approved behaviorは[Desktop UI-003](../specs/desktop.md#ui-003--作業領域と種類の表現)と[Browser WEB-002](../specs/browser.md#web-002--編集sessionと操作体験)へ適用。実装・実画面・回帰検証の対象Candidateと結果は[Development State](../execution-state.md)を参照。
