# Current Objective

## Objective

2026-10-22 Human requestに基づき、Explorerの階層罫線が深いネストやノード移動後に誤接続・途切れを起こす不具合を修正する。既存の親子関係を示す罫線仕様に沿い、各親の最後の直下の子で枝を終え、ノードの最新の位置と同時に描画する。

## Completion boundary

親子・兄弟関係による罫線描画、triangle / iconとの非重複、移動・Undo・検索・開閉・virtual scroll時の追従、Chromium / WebKitのfocused検証と実画面確認、self-review、通常commit / push、CI確認、開発版更新。CIを妨げる既存のhost初回buildの循環依存とWindows Publishのdrive prefix検査は、既存behaviorを満たす最小限のbug fixで解消する。

## Non-scope

データ形式・移動規則・保存規則の変更、CLIの製品機能変更、公開release、Unity起動。
