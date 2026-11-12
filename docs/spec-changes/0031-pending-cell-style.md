# 再計算待ちのセル書式

Status: Applied

2026-11-13の偶数行の背景色に関する報告に基づく。編集中の同じ表・行・列で、再計算を待つ間は最後に表示した書式を保持し、条件解除・計算失敗を含む新しい結果が届いた時点で切り替える。初表示のセルに別行の色を引き継がない。値・診断・コピーには現在の計算結果を使う。Workerの応答を保留させ、編集とレコード追加の途中状態をGridで検証する。

Approval mode: Agent-autonomous。新規行、仮想行の再利用、条件解除とコピー時の鮮度をレビューし、[Grid評価](../specs/display-evaluation.md)に適用。Blocking / Questions: None。
