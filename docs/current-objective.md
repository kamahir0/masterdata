# Current Objective

## Objective

MasterDataリポジトリを一般公開できる完成したOSSプロジェクトとして仕上げる。初見の第三者が製品を即座に理解し、Web版ですぐに試し、Desktop版やCLIを迷わず導入・継続利用できる公開品質を達成する。

## Completion boundary

1. **READMEの全面的な見直し**:
   - 製品の価値（ゲームマスターデータ管理、表と型の往復編集、Git親和性、MasterMemory / C#バイナリ自動生成）の明確化。
   - 実際のアプリから自動撮影した高品質スクリーンショットの掲載（Light / Darkテーマ、テーブル編集画面）。
   - 公開Web版（https://kamahir0.github.io/masterdata/）への即時アクセス導線。
   - サンプルプロジェクト（`samples/Adventure`）を用いたクイックスタート手順。
   - 利用形態別（Web版、Desktop版、CLI、開発者）の明確なガイド、対応環境、制約事項、セキュリティ・プライバシー説明。
   - 開発者・AIエージェント向け情報へのクリーンな導線分離。
2. **実アプリからのスクリーンショット生成**:
   - Playwrightを活用し、実データ・実コンポーネントを用いたスクリーンショット（Light/Dark）を自動撮影・配置。
3. **GitHubリポジトリ設定の仕上げ**:
   - Description、Homepage URL（Web版）、Topics（タグ）の適切な設定。
4. **配布・リリースの整備（CI/CD）**:
   - タグプッシュ等に対応したリリース自動化ワークフロー（`.github/workflows/release.yml`）の作成。各OS（macOS, Windows, Linux）のDesktop版成果物（DMG/zip, NSIS/msi, AppImage/deb）およびCLIバイナリのビルド・ドラフトRelease作成。
   - 一般利用者向けの未署名アプリ起動手順（macOS Gatekeeper、Windows SmartScreen）の明記。
   - バージョン採番（v0.1.0）の整合性確認。
5. **ドキュメントの整理**:
   - `AGENTS.md`、`docs/execution-workflow.md`、`docs/specs/`、ADRの整合確認。
   - `One knowledge, one owner` を守り、古い記述や矛盾を解消。
6. **実利用によるエンドツーエンド検証**:
   - READMEの手順通りに新規利用者がWeb版、Desktop版、CLI、Build/Publishを迷わず実行できることの実機検証。
7. **Human gateの提示**:
   - 正式GitHub Releaseの公開（publish）およびタグ作成に関する最終承認の準備と推奨案提示。

## Non-scope

既存のGUI編集モデル、保存・復旧トランザクション、データ形式（YAML/TOML）、Desktop/CLIの基本挙動の変更、無関係な新機能追加。
