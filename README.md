# MasterData

ゲームのマスターデータを、表と型を行き来しながら直感的に編集し、C#コードとMasterMemoryバイナリを自動生成するクロスプラットフォームエディタ。

デスクトップアプリ（macOS / Windows / Linux）、ブラウザ版（インストール不要）、およびAIエージェント・CI自動化向けのRustネイティブCLIを提供します。

[![Deploy Browser to GitHub Pages](https://github.com/kamahir0/masterdata/actions/workflows/pages.yml/badge.svg)](https://github.com/kamahir0/masterdata/actions/workflows/pages.yml)
[![Workspace](https://github.com/kamahir0/masterdata/actions/workflows/workspace.yml/badge.svg)](https://github.com/kamahir0/masterdata/actions/workflows/workspace.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

👉 **[ブラウザ版を今すぐ開く（インストール不要）](https://kamahir0.github.io/masterdata/)**

---

## プレビュー

<p align="center">
  <img src="docs/images/screenshot-light.png" alt="MasterData Light Theme" width="48%">
  <img src="docs/images/screenshot-dark.png" alt="MasterData Dark Theme" width="48%">
</p>

---

## 特徴

- **表と型のシームレスな往復編集**
  - テーブル（レコード一覧）と型定義（Enum / Flags / Custom構造体 / Value Object）を同じ画面で自由に行き来。
  - 作業途中の未完成なデータや診断エラーがあっても保存・別作業をブロックしません。
- **Git親和性の高いYAMLアーキテクチャ**
  - データはプレーンなYAMLファイルとして `sources/` 配下に保存。
  - コメント、キー順序、改行コード（LF / CRLF）、64bit整数精度（`ulong`）、`$tags` を完全保持。
  - テーブル定義（スキーマ）とデータファイルの分離に対応し、大規模チームでのGit競合を最小化。
- **C# / MasterMemoryバイナリの即時生成（Desktop / CLI）**
  - .NET 8を内蔵駆動し、編集中のドラフトから型安全なC#コードと高速バイナリ（MasterMemory）を直接生成・検証。
  - 生成されたアセットは設定した出力先（Unityプロジェクト等）へ安全にPublish。
- **強固な保存・復旧トランザクション**
  - 外部ファイル変更や競合を事前検知してデータ上書きを防止。
  - 複数ファイルの一括保存時もバックアップトランザクションにより自動復旧・ロールバックを支援。
- **AIエージェント・CIフレンドリーな設計**
  - Rustネイティブの単一バイナリCLIを同梱。
  - Claude Code等のAIコーディングエージェントがプロジェクト検査、検証、構造化編集、横断リネームを安全に実行可能。

---

## クイックスタート（30秒で体験）

1. **[ブラウザ版を開く](https://kamahir0.github.io/masterdata/)** （推奨: Google Chrome / Microsoft Edge）
2. スタート画面で **「サンプルで試す」** をクリック。
3. サンプルプロジェクト（冒険の道具箱）が作業用メモリ空間にロードされます。
4. アイテムやクエストの数値を編集したり、新しい列や型を追加して操作感を試せます。
5. Cmd+Z / Ctrl+Z でいつでもUndo / Redo可能です（元のサンプルは変更されません）。

---

## 利用方法

MasterDataは目的に応じて3つの形態で利用できます。

### 1. デスクトップアプリ（Desktop版）

完全なローカル環境でデータ編集からC# / MasterMemoryバイナリのビルド・Publishまで行える標準エディタです。

- **ダウンロード**: [GitHub Releases](https://github.com/kamahir0/masterdata/releases) より、お使いのOSに合ったインストーラーまたは実行ファイルをダウンロードしてください。
  - macOS: `.dmg` または `.app`（Apple Silicon / Intel）
  - Windows: `.exe`（インストーラー）
  - Linux: `.AppImage` または `.deb`
- **Build / Publish機能の要件**:
  - 通常のデータ編集・保存に特別な環境は不要です。
  - C#コード生成およびMasterMemoryバイナリ生成（Build / Publish）を実行する場合は、環境に **[.NET 8 SDK](https://dotnet.microsoft.com/download/dotnet/8.0)** 以降がインストールされている必要があります。

> [!NOTE]
> **未署名アプリの初回起動について**
>
> 本プロジェクトの公式配布バイナリはオープンソースのコミュニティビルドであり、商用証明書によるコード署名を行っていません。初回起動時は各OSのセキュリティ機構に従って起動してください。
>
> - **macOS**: 「開発元を検証できないため開けません」と表示された場合、Finderで `MasterData.app` を **右クリック（またはControl+クリック）して「開く」** を選択してください。次回以降は通常起動できます（またはターミナルで `xattr -d com.apple.quarantine /Applications/MasterData.app`）。
> - **Windows**: Microsoft Defender SmartScreenで警告が表示された場合、**「詳細情報」をクリックし、「実行」** を選択してください。
> - **Linux**: AppImageの場合は `chmod +x MasterData-*.AppImage` で実行権限を付与して起動してください。

---

### 2. ブラウザ版（Web版）

サーバーへのファイル送信を一切行わず、お使いのPC上のローカルフォルダをブラウザから直接安全に編集できます。

- **URL**: **https://kamahir0.github.io/masterdata/**
- **対応環境**: デスクトップ版 Chromium系ブラウザ（Google Chrome, Microsoft Edge等、File System Access API 対応環境）
- **使い方**:
  1. サイトを開き、「プロジェクトを開く」でローカルの `masterdata.toml` があるフォルダを選択。
  2. フォルダへのアクセス権限を許可します。
  3. 編集後、**保存（Cmd/Ctrl+S）** または **すべて保存（Cmd/Ctrl+Shift+S）** でローカルファイルへ直接書き込まれます。
- **制約事項**:
  - ブラウザのサンドボックス制約上、.NET SDKを必要とするBuild / Publish機能はブラウザ版では提供されません（データ編集・検証・保存に専念します）。Build / PublishにはDesktop版またはCLIをご利用ください。

---

### 3. CLI（コマンドラインツール）

AIエージェント（Claude Code等）による自律編集や、CIパイプラインでのデータ検証・ビルド自動化に最適なRust製ネイティブCLIです。

- **入手方法**: [GitHub Releases](https://github.com/kamahir0/masterdata/releases) の `masterdata-<target>.tar.gz` をダウンロード、またはソースからビルド（`cargo build --bin masterdata`）。
- **リポジトリ内からの実行**: `npm run cli -- <command>` または `npx tsx cli/main.ts <command>`

```sh
# プロジェクトの構造とテーブル概要を表示
masterdata inspect --project samples/Adventure

# スキーマ・型・キー重複・参照整合性を一括検証
masterdata validate --project samples/Adventure

# テーブル定義の作成
masterdata table create Monster --pk id

# 列の追加（MessagePackキー番号は自動採番）
masterdata table column add Monster hp int

# 定義・参照・レコードを跨ぐ横断リネーム
masterdata rename table OldMonster NewMonster

# 複数の編集操作を単一セッションで一括・安全に適用
masterdata tx --file operations.json

# C#コードとMasterMemoryバイナリの生成・検証
masterdata build --project samples/Adventure

# 成功した成果物をUnity等の出力先へ安全に配置
masterdata publish --project samples/Adventure
```

詳細なCLIコマンド仕様は [CLI Authoring and Automation](docs/specs/cli.md) を参照してください。

---

## プロジェクト構成

MasterDataプロジェクトは以下の構造で管理されます。

```text
my-game-data/
├── masterdata.toml         # プロジェクト設定（名前、出力プロファイル、Publish先）
├── sources/                # YAMLデータソース
│   ├── items/
│   │   ├── item.yaml       # テーブル定義およびレコード
│   │   └── item_data.yaml  # 分離データファイル（大量データ分割用）
│   └── types/
│       ├── rarity.yaml     # Enum型定義
│       └── stats.yaml      # Custom構造体型定義
└── .masterdata/            # 自動管理トランザクション・バックアップ（Git除外推奨）
```

- **`masterdata.toml`**: プロジェクトのルートを定義。出力プロファイル（開発用・本番用のタグ絞り込み）やPublish先ディレクトリを設定します。
- **`sources/`**: YAML形式でテーブルや型を定義。フォルダ階層は自由に整理できます。

---

## 開発・コントリビューション

リポジトリの開発やローカルビルドに参加する方は以下を参照してください。

### 必要環境

- **Node.js**: 22.12以降（v24推奨）
- **npm**
- **Rust**: 1.80以降（Tauri v2前提環境）
- **.NET SDK**: 8.0以降（Build / Publish機能のテストに必要）
- 各OSの [Tauri Prerequisites](https://v2.tauri.app/start/prerequisites/)

### クイックセットアップ

```sh
# 依存関係のインストール
npm ci

# 型チェックと静的Webビルド
npm run build:web

# デスクトップ版のホットリロード開発起動
npm start

# 単体テスト・結合テストの実行（.NET Buildテスト含む）
npm test

# ネイティブIPC・ホスト通信テスト
npm run build && npm run test:native

# Playwrightによる実UI回帰テスト
npm run test:ui

# 本番用デスクトップアプリのパッケージング
npm run pack:release
```

### 設計・仕様ドキュメントへの導線

One knowledge, one owner原則に基づき、詳細な設計・仕様は以下のドキュメントが管理しています：

| ドキュメント | 内容 |
| --- | --- |
| [AGENTS.md](AGENTS.md) | 開発者およびAIエージェントの開発規律とエントリーポイント |
| [Product Direction](docs/product/direction.md) | 製品の方向性、設計思想、解決する課題 |
| [Desktop Authoring](docs/specs/desktop.md) | デスクトップ版の編集モデル、保存、Build / Publish、標準UI |
| [Browser Authoring](docs/specs/browser.md) | ブラウザ版のファイルシステム連携、トランザクション安全保存、復旧規律 |
| [CLI Authoring and Automation](docs/specs/cli.md) | CLIのAIファースト操作体系、AST保持編集、横断リネーム仕様 |
| [Distribution & Release](docs/specs/distribution.md) | 公開ドキュメント、スクリーンショット、配布・リリース仕様 |
| [Architecture Decision Records](docs/adr/) | 主要な技術選定とアーキテクチャ判断の理由 |
| [Execution Workflow](docs/execution-workflow.md) | 開発ワークフロー、自律実行、Human gate基準 |

---

## License

本プロジェクトは [MIT License](LICENSE) のもとで公開されています。
