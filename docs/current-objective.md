# Current Objective

## Objective

2026-10-22 Human requestに基づき、Explorerのフォルダ作成でインライン名前入力のfocusが失われ、外側clickでも確定できなくなる問題を解消する。特に閉じた親・深いネストで、作成対象を見失わず名前を確定できるようにする。

## Completion boundary

focus喪失の再現と原因調査、必要な仕様整理、共通するinline rename lifecycleの修正、外側click / Enter / Escapeとネスト・source作成のfocused validation、画面確認、self-review、通常commit / push、CI確認、開発版更新。

## Non-scope

データ形式、保存規則、CLI、公開release、Unity起動。
