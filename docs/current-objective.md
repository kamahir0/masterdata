# Current Objective

## Objective

製品像に沿ったデスクトップGUIを、仕様化から実装・実操作検証・配布可能なローカルアプリまで完成させる。

選択根拠: 2026-10-17 Human request「実装を開始する」「legacy2を参考にすることで妥当な回答を補完・導出」「完成まで自走して」。通常の製品・技術判断はlegacy2の参考情報と新しい製品像から導出し、Humanへの再確認を要求しない。

追加scope: 同日Humanが示した、Ant Designのようなフレームワークの広範な活用と、ほとんどの操作に伴うアニメーションによる豊かなフィードバックを製品像へ反映する。

## Completion boundary

- プロジェクト作成・open・保存、Explorer、表・型の直接編集、共通の値入力、ネスト・配列・nullable、検索、Undo / Redo、診断。
- キー・参照、タグ・出力プロファイル、C# / MasterMemory成果物のBuild / Publish。
- Ant Designを広く使う統一されたUI、豊かなmotion、キーボード操作、軽快な切替と入力保持。
- ユーザーのファイルを保護する保存・外部変更検出、テスト、実際のDesktopでの代表作業、ローカルアプリ、CIと通常のcommit / push。

## Non-scope

CLI、Gitクライアント機能、共同編集、AIチャット、plugin基盤、公開release、課金サービス、旧仕様の機械的移植。

実データの意図的破棄や公開release等は、通常の設計判断の委任とは区別する。
