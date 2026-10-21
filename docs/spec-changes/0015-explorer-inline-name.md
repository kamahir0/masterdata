# Explorerの名前入力と外側click確定

Status: Applied

Approval mode: Agent-autonomous

## Basis / scope

2026-10-22 Human Report: フォルダ作成のinline入力でfocusが外れたようになり、別の場所をclickしても確定しない。Human Requirement: 名前入力中に外側をclickすると作成名を確定できる。閉じた親・深いネストも同様に操作できる。

## Behavior / Agent Decision

Explorerの作成 / renameは、親folderの展開等の表示更新で入力対象とfocusを失わない。名前入力欄の外側clickによるblurは有効な名前を確定してinline編集を終える。クリックした次の操作先へfocusを渡し、後から取り戻さない。Enterによる確定、Escapeによる今回の名前編集の取消も維持する。folderだけでなく共通のfile名編集も同じ挙動にする。確定は既存の可逆draft操作であり、diskへ暗黙保存しない。無効名は既存の理由通知と元のpathの保持を維持する。

## Acceptance / validation

閉じたfolder内の新規folder作成から展開完了後もkeyboard入力でき、外側clickで新しいpathが確定する。作成したfolder配下に続けてfolderを作る場合も同じ。source作成・既存項目rename、Enter / Escape、確定後のfocus、Undo / SaveをChromium / WebKitで検証する。通常のfolder開閉motionは維持し、固定待機時間のfocus再設定で補修しない。

## Unresolved / safety

None。保存・衝突・上書き規則は変更しない。名前の再入力を要求するmodal等は追加しない。

## Review

別passで新規作成と既存rename、folderとfile、展開中のfocus保持と次操作への引渡し、Enter / Escape / 無効名、保存とUndoへの影響をreviewした。固定delayや遅延focusで次の操作を奪わないこと、通常開閉motionは維持することを確認した。Blocking / Non-blocking / Questions: None。Approved as Proposed: Yes。Autonomous approval eligible: Yes（選択Objective内のinline編集の修正）。Human gate: None。
