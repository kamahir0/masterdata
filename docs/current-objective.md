# Current Objective

## Objective

2026-10-22 Human requestに基づき、実装済みのMasterDataブラウザ版をGitHub Pagesで静的公開し、公開URL（HTTPS）から正常に利用できることを確認する。既存の `npm run build:web` を活用したGitHub Actionsによる自動デプロイを構成し、Viteの配信パス、権限、静的アセット読み込みを確認・検証する。

## Completion boundary

GitHub Actions による GitHub Pages 自動デプロイワークフローの構成、Vite配信パスと静的アセット解決の検証、既存ブラウザ版・Desktop・CLIの保護（公開無関係部分の不変更）、commit / pushとGitHub PagesデプロイCIの成功確認、公開HTTPS URLでの実際の起動確認、テスト用ローカルプロジェクトを用いたChromeでの読込・編集・保存の検証、workspace / whitespace checkのパス、公開URLと検証結果の報告。

## Non-scope

編集UI、保存・復旧処理、Desktop / CLIなどの公開に無関係な部分の変更、実データへの接触・変更、Safari / Firefoxなど未対応ブラウザのサポート拡張。
