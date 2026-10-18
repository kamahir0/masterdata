# ファイル中心のcompact workbench

Status: Applied

Approval mode: Agent-autonomous

## Basis

2026-10-18 Human要求: VS Code風の一時tab / double clickでの定着、Cmd+Wで現在fileを閉じる、安全な未保存確認、Explorer開閉、独立したeditor設定（Light / Dark / System、一行scroll / 複数行tabs）、無彩色darkと緑accent、上部の面積削減。

VS Codeの起動中画面を観察: 約32pxのtitlebarとtab帯、細い境界、面の明暗差、内容に重ならない補助UI。preview / 編集時の定着 / wrapは[公式説明](https://code.visualstudio.com/docs/editing/getting-started/userinterface)も参照した。VS Code自体の設定や編集中fileは変更しない。

## Behavior

- Explorer fileのsingle clickは最大一つのpreview tabを開く。file名はitalic、次の未open fileでcleanなpreviewだけを置換する。既存の定着tabを選んでもpreviewは保持する。Explorer file / tabのdouble clickで定着する。編集されたpreview、元からdirtyなfile、新規作成fileは自動定着し、別fileの選択でdraftを失わない。移動前に入力途中の値をcommitする。
- close iconとCmd/Ctrl+Wは指定 / 現在file tabだけを閉じ、最後のtabを閉じてもprojectとwindowを保持する。dirtyならfile名を示して「保存」「保存しない」「キャンセル」を選べる。保存はそのfileだけ。破棄はそのfileを保存済み状態へ戻し、未保存の新規fileは取り除く。他file・project設定・folder draftを暗黙保存・破棄しない。破棄はProject Undoで回復できる。破棄先pathが別draftに占有されている場合も理由を示して保持する。保存失敗 / 競合 / 保存中の追加入力では未保存のままtabを閉じず、入力を保持する。OSのwindow close / 終了 / project切替は従来通り全体の未保存確認を使う。
- Explorerは常時到達できるtoggleと表示menuで開閉する。検索、展開、選択、fileの表示と入力を保持し、hiddenなExplorerはkeyboard操作対象にしない。既存Build shortcut Cmd/Ctrl+Bは維持し、Explorer toggleはCmd/Ctrl+Shift+Eとする（Agent Decision: 既存shortcutとの衝突回避）。
- editor設定はproject設定と別の入口・画面にし、PC内で保存する。themeはLight / Dark / System（初期System）、System時はOS変更へ追従する。tabは一行scroll（初期）/ 自動折り返し。大量の折返しはtab帯内を縦scrollして編集領域を保持する。Explorer visibility、既存motion設定もeditor preference。これらはprojectの保存対象・dirty・Undoに含めず、変更で編集sessionをreloadしない。
- darkは黒に近い無彩色、lightは白・灰色のsurface、共通の緑accentを使う。active tab・選択・focusを区別し、背景や境界でpaneを控えめに分ける。error / warningの意味色は保つ。
- title / 全体操作帯 / file tabs / file内modeをcompactにする。大きいfile見出しと重複したpathは圧縮し、定義名の直接編集と型表示は保持する。Open / Save / Undo / Redo / Build / Publish / 結果 / 両設定は発見可能な入口を維持する。最小window幅でも操作を重ねず、折返しtabとpanelで表のscroll計測を壊さない。panel開閉は短いmotion、入力を待たせずreduced motionを尊重する。

Agent Decision: previewの編集時自動定着、file単位discardのUndo、設定の初期値・保存scope、最後のtab後は空editor、折返しtab帯の高さ上限は、入力保護と広い作業領域のために採用する。既存ユーザー向けcompatibility costはない。実際のユーザーdraft・外部fileの安全性は維持する。

## Validation

preview置換 / 両double click / 編集時定着 / 入力途中の遷移、clean・dirty・新規tabのclose（Save / Discard / Cancel / save失敗 / 他file draft / 保存中編集）、最後のtabとwindow closeの区別。Explorerを隠して復元。設定変更・再load・System theme変更中のdraft保持。両tab表示、keyboard操作、横幅不足、両themeとcompact高さの実画面、既存の編集・nested・保存・motion回帰、native menuと3OS CI。

## Review

別passでpreview置換前のcommit、file保存と全体closeの分離、new file破棄、path衝突、保存中編集、設定変更時のsession保持をchallenge review。Blocking / Questions: None。Autonomous approval eligible: Yes（選択済みObjective内、外部作用は既存Save境界）。Human gate: None。Approved behaviorは[Desktop Authoring](../specs/desktop.md)へ適用。
