# 視認できる削除feedback

Status: Applied

Approval mode: Agent-autonomous

## Basis / decision

Humanは削除アニメーションが見えないことを報告し修正を指示した。実画面の途中frameでは、旧snapshotが80ms時点で約18%まで薄くなり、追加ボタンが先に移動して重なることを確認した。

Agent Decision: 削除feedbackを約300msとし、退出領域の縮小と後続要素・末尾操作の移動を同じ補間で結ぶ。単なる残像より、何が消え、空間がどう詰まったかを認識できることを優先する。横方向の列削除も同様。dataの削除は即時に一つのUndoへ反映し、入力・Undo・連続操作を待たせない。snapshotは非対話・非accessibleで、完了・取消・navigationで回収する。画面外の対象を増殖させず、reduced motionでは即時反映する。tabの既存motionは変更しない。

## Acceptance / validation

Enum / Flags・Custom field・表の行 / 列・配列・Explorerで削除対象の退出が視認でき、後続項目や移動する追加操作が瞬間移動しない。途中frameの輪郭・不透明度と後続要素の座標、実時間の描画を確認する。即時Undo、連続削除、reduced motion、画面外・scrollの回帰も確認する。保存形式、Build、ユーザーdraftは変更しない。

## Review

別passで即時draft反映、Undo、連続操作、非対話snapshot、virtual表示と末尾操作の位置関係を確認。Blocking / Questions: None。Approved as Proposed: Yes。Autonomous approval eligible: Yes（選択済みの削除feedback修正内）。Human gate: None。
