# Current Objective

## Objective

値入力の共通枠が横幅とnull操作領域を管理し、null・非null・型別部品の変更でも表とネスト内の入力幅が自動で揃う設計へ修正する。

選択根拠: 2026-10-18 Human request「設計修正」と直前の共通レイアウトに関する合意。

## Completion boundary

共通枠の仕様・実装、型とnull状態を切り替える実際のUIでの幅・操作確認、focused validation、self-review、通常commit / pushとCI。

## Non-scope

値の意味・保存形式の変更、アプリ全体の再設計、既存の未保存内容の変更。
