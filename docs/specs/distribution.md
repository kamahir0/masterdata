# Distribution, Release and Public Documentation

Status: Approved

Approval mode: Agent-autonomous
Basis: 2026-10-22 Human requestによる「MasterDataリポジトリを一般公開できる完成したOSSプロジェクトとして仕上げる」Objective。別pass reviewによりBlockingなし、ドラフトReleaseによるHuman gate保護を確認し自律承認。

## 根拠と範囲

### 1. 解決する問題と対象シナリオ
- **初見の訪問者**:
  - リポジトリに訪れた瞬間、MasterDataが「テーブルデータと型定義を同一ワークスペースで編集し、型安全なC# / MasterMemoryバイナリを生成するマスターデータエディタ」であることを3秒で理解できる。
  - インストールなしで即座にブラウザ版（GitHub Pages）を開き、サンプルデータを読み込んで操作感を試せる。
  - 実際のUI（Light / Darkテーマ、テーブル編集画面）をスクリーンショットで事前に確認できる。
- **Desktopユーザー**:
  - GitHub Releasesから自身のOS（macOS Apple Silicon / Intel、Windows x64、Linux）に合った配布物をダウンロードし、インストール・起動できる。
  - OSの未署名アプリ警告（macOS Gatekeeper、Windows SmartScreen）を回避して安全に起動する手順が明記されている。
  - Build / Publish機能を利用する際に必要な環境（.NET 8 SDK）の有無を理解して利用できる。
- **Webユーザー**:
  - Chrome / Edge等の対応ブラウザでローカルフォルダを開き、オフラインかつ安全に作業・保存・復旧できる。
  - ブラウザ版の機能境界（Build / Publish機能はDesktop / CLI専用であり、Web版はデータ編集・検証・保存に専念）を迷わず把握できる。
- **CLIユーザー（AIエージェント・CI自動化）**:
  - `masterdata` バイナリまたは `npm run cli` / `npx tsx cli/main.ts` で、AIエージェント（Claude Code等）やCIがプロジェクト検査・検証・一括編集・Build / Publishを実行できる。
- **開発者・コントリビューター**:
  - 一般利用者向けのREADMEから開発者向けの手順・仕様・ワークフロー・ADRへの導線が明確に分離され、最短で開発環境（Node.js, Rust, .NET SDK）を構築・検証できる。

### 2. 根拠の区別
- **Human Decision / Requirement**:
  - リポジトリを一般公開できる完成したOSSプロジェクトとして仕上げる。
  - READMEの全面見直し（価値、機能、実機スクリーンショット、Web版リンク、クイックスタート、各利用方法、対応環境、制限、開発者導線）。
  - `One knowledge, one owner` を遵守したドキュメント整理。
  - GitHub Releases、バージョン、Desktop版配布方法、CI/CD自動化、インストール・更新手順。
  - GitHubリポジトリの仕上げ（Description、Website、Topics、License等）。
  - 新規利用者がREADMEだけで試せる実利用による最終確認。
  - 正式なGitHub Release公開や外部配布などのHuman gate操作は承認を取得する。
- **Agent Decision**:
  - 新規仕様として `docs/specs/distribution.md` を作成し、配布・リリースおよび公開ドキュメントのライフサイクルを管理する。
  - スクリーンショットはPlaywrightテストから自動キャプチャし、実アプリ・実データ（`samples/Adventure`）に基づき最新UIと一致させる。
  - リリースCI/CDワークフロー（`.github/workflows/release.yml`）はドラフトRelease（`draft: true`）として作成する。これによりビルド成果物の自動生成・検証までを自律的に行い、公開（Publish）をHuman gateとして安全に分離する。
  - 配布バイナリにはDesktopアプリ（macOS DMG/zip、Windows NSIS/zip、Linux AppImage/deb）に加えて単体CLIバイナリ（`masterdata`）を含め、AIエージェントやCIでの即時利用を可能にする。

### 3. Non-scope
- 既存のGUI編集モデル、保存・Undo・復旧トランザクション、YAML/TOMLデータ形式の変更。
- 有料のApple Developer Program / Microsoft Authenticode証明書によるコード署名（未署名バイナリとしての安全な起動手順の提供を標準とする）。
- 無関係な新機能追加や外部ホスティングサービスへの新規依存追加。

---

## DIST-001 — 公開READMEと情報アーキテクチャ

1. **構成順序**:
   1. **タイトルとバッジ / クイックリンク**:
      - プロジェクト名、一行要約、Web版へのリンクボタン、最新Release、License、CI状態。
   2. **MasterDataとは（価値と特徴）**:
      - スプレッドシートや手書きYAMLの課題（型安全性、差分管理、Git競合、MessagePackキー番号管理）の解消。
      - テーブルと型のシームレスな往復編集、未完成データの許容、即時Undo/Redo。
      - 高速なMasterMemory C#コードおよびバイナリの自動生成。
   3. **スクリーンショット**:
      - LightテーマおよびDarkテーマでの実際のテーブル編集画面（実機キャプチャ）。
   4. **今すぐブラウザで試す（30秒クイックスタート）**:
      - Web版URL（https://kamahir0.github.io/masterdata/）。
      - 「サンプルで試す」または `samples/Adventure` フォルダ選択による即時体験。
   5. **利用形態別の利用ガイド**:
      - **Desktop版**: ダウンロード、インストール、起動方法、未署名アプリの開き方、Build / Publishに必要な .NET 8 SDKの案内。
      - **Web版**: 対応ブラウザ（Chromium系推奨）、機能境界、ローカル保存と復旧の安全性。
      - **CLI版**: 主なコマンド（`inspect`, `validate`, `build`, `publish`, `record`, `tx`）、AIエージェント連携のメリット。
   6. **データ構造とプロジェクト**:
      - `masterdata.toml` と `sources/`、Git親和性（YAML AST保持、コメント・順序保持）。
   7. **開発・コントリビューション**:
      - 開発環境要件、ローカル起動（`npm start` / `scripts/local-app`）、テスト、仕様書・ADR・ワークフローへの導線。
   8. **ライセンス**:
      - MIT License。

2. **One knowledge, one ownerの徹底**:
   - READMEは「利用者のための入口とクイックガイド」に専念する。
   - 内部アーキテクチャの詳細は `docs/specs/desktop.md` / `browser.md` / `cli.md`、設計経緯は `docs/adr/`、ワークフロー規律は `AGENTS.md` / `docs/execution-workflow.md` をcanonical ownerとし、重複した長大な内部解説をREADMEに置かない。

---

## DIST-002 — 実機スクリーンショットの自動生成

1. Playwrightテストスクリプト（`tools/capture-screenshots.mjs`）を作成し、実アプリをヘッドレス起動して以下を自動撮影する：
   - `docs/images/screenshot-light.png`: サンプルプロジェクト（`samples/Adventure`）のアイテムテーブル編集画面（Lightテーマ）。
   - `docs/images/screenshot-dark.png`: 同画面（Darkテーマ）。
2. 画像はPNG形式で保存し、GitHub上で鮮明にプレビューできるよう適切な解像度（1440x900等の標準デスクトップサイズ）で生成する。

---

## DIST-003 — 配布・リリース自動化（CI/CD）

1. **ワークフロー定義**: `.github/workflows/release.yml`
   - **トリガー**: `push: tags: ['v*']` および `workflow_dispatch`。
   - **マトリックス**:
     - `macos-latest` (Apple Silicon / aarch64) および必要に応じ Intel (x86_64)
     - `windows-latest` (x64)
     - `ubuntu-latest` (x64)
   - **ビルド処理**:
     - Node.js 24、Rust toolchain、.NET 8 SDKのセットアップおよび依存キャッシュ（npmキャッシュ、Swatinem/rust-cacheによるCargo依存クレートキャッシュ）。
     - `npm ci`、`npm run check`（早期型検査）。
     - Tauriビルド（`beforeBuildCommand` にてWebバンドル・CLI hostを自動生成）:
       - macOS: `npm run pack:release -- --bundles app,dmg -- --locked`
       - Windows: `npm run pack:release -- --bundles nsis -- --locked`
       - Linux: `npm run pack:release -- --bundles appimage,deb -- --locked`
     - 単体CLIバイナリのアーカイブ作成（`masterdata-v*-<target>.tar.gz` / `.zip`）。
   - **ドラフトRelease作成**:
     - `softprops/action-gh-release` または `gh release create --draft` を使用。
     - 各OSのインストーラー・バンドル・CLIバイナリ、SHA256チェックサムファイルをドラフトReleaseに添付する。
     - リリースノートの自動生成（コミットログまたは主要機能サマリー）。
2. **Human gate**:
   - ドラフトReleaseの作成・ビルドまでは自動化・自律実行可能。
   - ドラフトから「公開（Publish）」への移行、および外部告知はユーザーの承認を取得する。

---

## DIST-004 — 未署名バイナリの起動・導入手順

各プラットフォームのOSセキュリティ機構に対する正規の手順をREADMEに明記する：
1. **macOS**:
   - 初回起動時に「開発元を検証できないため開けません」と表示された場合：
     - Finderで `MasterData.app` を右クリック（またはControl+クリック）し、「開く」を選択。
     - またはターミナルで `xattr -d com.apple.quarantine /Applications/MasterData.app` を実行。
2. **Windows**:
   - Microsoft Defender SmartScreenで「WindowsによってPCが保護されました」と表示された場合：
     - 「詳細情報」をクリックし、「実行」を選択。
3. **Linux**:
   - AppImageの場合: `chmod +x MasterData-*.AppImage` を実行して起動。

---

## DIST-005 — GitHubリポジトリメタデータの整備

1. **Description**:
   - 「テーブルデータと型定義を同一ワークスペースで編集し、型安全なC# / MasterMemoryバイナリを生成するマスターデータエディタ（Desktop / Web / CLI）」
2. **Homepage**:
   - `https://kamahir0.github.io/masterdata/`
3. **Topics**:
   - `game-development`, `gamedev`, `master-data`, `tauri`, `react`, `rust`, `mastermemory`, `unity`, `csharp`, `yaml`
