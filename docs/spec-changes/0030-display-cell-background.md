# 動的書式のセル背景色

Status: Applied

2026-11-12 Human Requirement。関数列・テーブル書式の背景色を、表のセルの四辺まで連続して表示する。通常列の入力周囲や空欄、関数列の短い表示値の左右にも同じ色を適用する。書式の条件を外したときは通常のセル背景に戻す。

[Grid評価](../specs/display-evaluation.md)の書式合成を維持し、選択枠・フォーカス・エラー表示を背景の上で識別できるようにする。通常列と関数列で実際のセルの色、選択操作、条件変更後の復帰を確認する。

Approval mode: Agent-autonomous。別passで、通常列と関数列の適用範囲、条件解除時の復帰、操作状態の視認性を確認。Blocking / Questions: None。
