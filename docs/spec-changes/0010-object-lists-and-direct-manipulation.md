# 項目編集・ファイル操作・表とネストの直接操作

Status: Applied

Approval mode: Agent-autonomous

## Basis

2026-10-19 Humanの追加レビュー（MasterRef / profileの境界と削除、仮想表の削除、Enum選択の開閉、追加先、tab / Explorer menu、drag移動と階層線、固定列、nested遷移、theme icon、行選択）を対象とする。直前の「MasterRefの表示名はどこで決まるか」という質問は表示名設定の追加指示として扱わない。

## Behavior / Agent Decision

- MasterRefと出力profileは共通の項目枠で編集する。項目の名前と、右端の「項目全体を削除」操作をheaderに置き、内部設定を同じ境界・余白・背景で囲む。安定identityを使い、追加・削除・Undoで他項目の入力を取り違えない。副キーも同じ枠を用いる。
- 検索できる単一選択は共通の操作規則を使う。開いた空の入力領域を再clickすると閉じ、入力すれば絞り込み、Esc / 選択で閉じる。検索中の文字編集や複数選択のtag操作を妨げない。Enum / MasterRef / 型 / profileへ適用する。
- 仮想表は可視行 / 列の退出と後続配置を連動させる。削除に伴うscroll終端の自動補正でも位置移動を無言で省略しない。sticky列やheaderをsnapshotで覆わず、画面外の全recordをmountしない。data / Undoは即時、連続操作・reduced motionを保持する。
- レコード追加先は複数sourceがあるときだけ「追加先ファイル」として示す。選択肢にfile名・pathを示し、既存recordの移動ではなく新規recordの所属先だと説明する。一つしかなければ余分な選択を置かない。保存形式やsource構造は変えない。
- file tabとExplorerのfileは同じcontext menuを使い、名前変更・削除・パスをコピーを提供する。folderだけ作成を追加する。renameはExplorerを表示し祖先を開いてinline入力へ移る。copyは現在のdraft pathに対応する絶対path（project root + sources）をコピーする（Agent Decision）。未保存内容は暗黙保存・破棄せず、削除・renameは既存の可逆draft操作とする。
- Explorerのdrag先判定と表示を一本化する。folder中央はそのfolderをhighlight、file上やnode間は明瞭な挿入線で親folderを示す。空白・見出し部分へのdropはsources rootでExplorer全体をhighlightする（Agent Decision）。祖先への移動を許可し、自己・子孫・衝突は拒否する。同じ場所へのdropは変更なし。hover展開、取消とUndoを保持する。
- 展開folderの下端から子孫の範囲へ細い階層線を伸ばし、末端は子node方向へ直角に折ってicon手前で止める。triangle hoverの背景はtriangle付近に収める。resize境界は表のsticky headerより前で操作できる。
- 表の行操作とタグを左の固定領域へまとめる。固定領域のcontrol以外をclickすると行全体を選択し、Ctrl / Cmdで追加解除、Shiftで範囲選択できる。行indexは選択可能textにしない。値入力・タグ編集・menu・dragは各操作を維持し、gridのコピー / Delete / Undoを保持する。
- 配列 / Customを潜る・戻るときは方向の分かる短いslideとfadeを示す。旧paneは退出中に入力・accessibility対象にせず、現在paneは待たずに操作できる。編集中の値のcommit / Esc、全breadcrumb、reduced motionを保持する。
- theme選択は文字に太陽・月・PC iconを併記する。PC内設定とproject draftは分離する。

## Validation / scope

項目追加・rename途中・削除・Undo、全単一選択の二度click / typing / Esc、縦横scroll終端の表削除と即時Undo、追加先の単一 / 複数sourceと保存先、tab / Explorer menuの一致とclipboard / hidden Explorer rename、folderを祖先 / rootへ移動・自己 / 子孫 / 衝突拒否・gap / folder表示一致・取消・Undo、長いtreeのvirtual scrollと階層線、resize hit test、固定タグ / menuと行選択・keyboard、深いnested往復と途中入力、light / dark / reduced motionをfocused testと実画面で確認する。保存・Build形式、MasterRef名の生成規則、公開release、ユーザーdraftの暗黙保存・破棄は対象外。

## Review

別passでdraft保護、共通menuの対象同一性、同じ場所 / 祖先 / 子孫の区別、clipboardの対象path、virtual終端補正、入力中の検索とslide中の旧paneの非対話性をchallenge reviewした。Blocking / Questions: None。Approved as Proposed: Yes。Autonomous approval eligible: Yes（Humanが列挙した改善内で、formatと既存dataの保存保護を保持）。Human gate: None。
