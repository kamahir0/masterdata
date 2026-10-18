# 値入力枠が幅と操作領域を管理する

Status: Approved

Approval mode: Agent-autonomous

## Basis / intent

2026-10-18 Humanはnullの作成ボタンと非null入力の幅について相談し、型別に毎回調整せず共通レイアウトが自動で揃える設計への修正を依頼した。nullかどうかを切り替えた際の視覚的な位置の変化を減らし、今後の型別部品追加でも同じ規則を使う。

## Proposal

- 表のセルとネスト内は同じ値入力枠を使う。枠が「利用可能な残り幅を使う値領域」と「nullableの場合に確保する固定幅のnull操作領域」を所有する。
- null / 未入力の作成ボタン、文字列、数値、Enum / Flags、参照、配列・Customの入口は値領域を満たす。同じ幅の枠なら状態や型別部品で入力領域の左右端を変えない。部品ごとの幅上限や配置側の型別補正を使わない。
- nullableな枠はnull状態でも操作領域を空けて保持する。空いた領域をfocus可能な操作にしない。boolは値領域の中で適切な大きさのswitchを使い、switch自体を引き伸ばさない。
- null作成・解除、Undo、入力・保存の意味は維持する。非nullableな枠には不要なnull操作の領域を要求しない。

Agent Decision: 「全型で同じ幅」は、同じ幅を与えられた入力枠の内部規則とする。異なる列やpaneそのものを同じ幅にはしない。既存の型判定は共有ValueControlで行い、共通枠が型別実装から配置責務を引き取る。

## Validation

代表的な文字列・数値・Enum / Flags・Value Object・配列・Custom・boolをGUIでnull化してUndoし、入力領域と右側操作の位置が変わらないことを確認。広いネスト内でも作成ボタンが値領域を使い、boolのswitchの大きさが表と同じことを確認する。Chromium / WebKit、既存回帰、workspace / whitespace check。

## Review

Blocking Issues / Non-blocking Issues / Questions: None. Approved as Proposed: Yes. Autonomous approval eligible: Yes（選択済みObjective内、値の意味や保存・permissionを変えず、状態ごとの領域と検証が明確）。Human gate: None.
