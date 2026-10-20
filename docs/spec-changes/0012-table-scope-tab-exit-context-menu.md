# 表の追加先規則・tab退出・標準context menu

Status: Approved

Approval mode: Agent-autonomous

## Basis

2026-10-21 Human Proposal: 複数fileのrecordを同時表示しているときは追加不可にすれば「追加先ファイル」UIが不要になる。Human Report: 右端tab closeだけ退出が見えない、macOSの未定義右clickにReloadが表示される。

## Behavior / Agent Decision

- 表の表示対象file集合から追加可否と追加先を一意に導く。複数sourceの「すべてのデータ」では新規recordを追加しない。「追加先ファイル」は撤去する。追加buttonはdisabledとし、上部の表示選択で一つのfileを選ぶ案内をtooltip / accessible descriptionで示す。一つのsourceしかない表なら「すべてのデータ」でも従来通り追加できる。検索結果が一つのfileに偏っただけでは追加可否を変えない。
- 複数file表示での複製と、表示末尾を越えてrecordを増やす貼付けも同じ規則を使う。既存recordへの編集・範囲内貼付け・削除・移動は維持する。貼付けの超過は一括で拒否し、部分適用せず、file選択で回復できる理由を表示する。単一file表示では追加・複製・貼付け拡張をそのfileだけに行い、Undo / default / unique key割当・検索解除とfocusを保持する。
- 空のテーブルデータfileを作成したら、そのfileを開き、直ちにrecordを追加・入力・保存できる状態にする。表示対象fileがUndo等で消えたら「すべてのデータ」へ戻し、削除済みfileへの追加を防ぐ。
- tab closeは中間・右端・最後の一つで共通の退出を示す。閉じる対象の姿が短時間残り、fade / 幅の縮小を見せる。後続tabの移動だけに依存しない。退出tabは直ちに非対話・非accessible、残存tabの縦位置・即時操作、連続close / open、折返し・横scroll・reduced motionとdirty close保護を維持する。
- rendererの共通入口でWebView標準context menuのdefault actionを抑える。custom file / folder / tab menuへeventを伝える。標準Reloadが未保存draftを消す入口にならない。textのkeyboard shortcutとnative編集menuは維持する。汎用text context menuの新設は今回対象外。

## Acceptance / validation

複数file同時表示の追加・複製・超過貼付けの拒否と既存編集、file選択後の追加 / 複製 / 貼付け保存先、検索と空file、作成直後の表示・保存・Undo、単一source表の回帰。右端 / 最後tabの実frameで中間opacity・幅を観測し、残存tabの縦位置、連続操作、scroll / wrap / reduced motionを確認。welcome / 空白 / 表 / text input / Drawerの右clickで標準menuを抑え、Explorer / tabのcustom menu・通常keyboard編集を維持する。Chromium / WebKit focused testsとnative macOSの実操作、3 OS CIで確認する。

## Review

別passで複数fileと検索の区別、複製 / pasteからの迂回、空file作成後の保存対象、Undoでの選択先消失、最後tabの退出とaccessibility、custom menuへのevent伝播をchallenge reviewした。Blocking / Questions: None。Approved as Proposed: Yes。Autonomous approval eligible: Yes（Humanの改善提案内の可逆なGUI操作変更でdata formatとdraft保護を維持する）。Human gate: None。
