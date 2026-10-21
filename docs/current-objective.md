# Current Objective

## Objective

2026-10-22 Human requestに基づき、Explorerのdrag移動が同名衝突等で拒否されるとき、その理由と対処が分からずbugに見える問題を解消する。移動拒否規則と既存データを保持し、操作場所で説明する。

## Completion boundary

変更仕様の別pass review・承認、drag先判定と理由表示の統一、衝突 / 自己・子孫 / root / gapと取消・回復のfocused validation、画面確認、self-review、通常commit / push、CI確認、開発版更新。

## Non-scope

置換・上書き・folder merge、データ形式・保存規則・CLIの変更、公開release、Unity起動。
