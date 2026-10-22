# Current Objective

## Objective

2026-10-22 Human requestに基づき、MasterDataをインストール不要の静的ブラウザアプリとして実装し、実用可能な状態まで検証する。ローカルProjectを選択し、初回読込後はメモリで編集、明示Save時のみディスクへ反映する。既存GUIの編集UX、合理的なコード共有、データを失わない保存を重視し、Desktop / CLIの品質を維持する。

## Completion boundary

[PR #1](https://github.com/kamahir0/masterdata/pull/1)の未承認提案と現行仕様・コードの照合、仕様の改善・別pass review・承認、静的アプリとBrowser I/O実装、保存・競合・中断回復と互換性の検証、実ローカルProjectを使ったGUI操作・修正、Desktop / CLI回帰、self-review、通常commit / pushとCI確認。

## Non-scope

BrowserのBuild / Publishおよび関連UI、サーバー保存・アカウント・同期、公開release / deployment、Unity起動。Build / Publishは既存CLI / Tauri版へ委ねる。
