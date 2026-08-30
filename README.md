# masterdata

`masterdata` は Unity + MasterMemory を対象にした、YAML-firstのローカルファーストなマスターデータ開発システムです。CLIとTauri GUIは、同じRust application serviceとcoreを直接利用します。

このリポジトリはproduct Type Systemの実装前段階です。project discovery、設定読込、YAML文書の分類、基本検証、C#生成の足場、CLI、Tauriアプリシェル、.NET bridge smoke test、実際のMasterMemory v3を使う独立technical spikeまでは動作します。schema-drivenのproduction binary buildは、仕様が承認されるまで意図的に未実装です。

## アーキテクチャ

```text
       masterdata-cli                 Tauri GUI
              \                         /
               \                       /
                +-- masterdata-app ---+
                       /    |    \
                      /     |     \
     masterdata-core  codegen  masterdata-dotnet
```

主要な責務は次の通りです。

- `masterdata-core`: project解決、`masterdata.toml`、YAML document/data model、validation、build plan
- `masterdata-codegen-csharp`: schema ASTからC# scaffoldを生成する独立境界
- `masterdata-dotnet`: .NET SDKと将来のMasterMemory builderを呼び出す唯一のRust adapter
- `masterdata-app`: CLIとTauriが共有するproject/validate/build orchestration。domain semanticsは持たない
- `masterdata-cli`: application serviceを使うCLI。GUIやCLIにdomain logicを重複させない
- `apps/gui`: TypeScript + React frontendとTauri v2 shell。backend commandはapplication serviceを呼ぶ
- `xtask`: repository固有の開発コマンドをRustで集約

最終的なbuild pipelineは次を目指します。

```text
resolve project
  -> load config
  -> discover YAML
  -> parse schema/data
  -> semantic validation
  -> resolve types/indexes/references
  -> generate C#
  -> schema hash
  -> compile/reuse .NET builder
  -> build MasterMemory binary
  -> validate binary
  -> atomic output replace
```

## 前提条件

- Rust stable（`rust-toolchain.toml`で `rustfmt` / `clippy`も指定）
- .NET SDK 8以上（`global.json`は8.0系を選択）
- Node.js 20以上とnpm
- GUI開発時のTauri v2依存関係
  - macOS: Xcode Command Line Tools
  - Windows: WebView2、Visual Studio C++ Build Tools
  - Linux: GTK/WebKitGTK等。CIではworkflowが導入する

Tier 1はmacOS arm64とWindows x64です。Linux x64ではRust core、CLI、CIをサポートし、GUI配布はbest effortとします。path separatorやshell固有の処理をdomain logicに持ち込まない方針です。

## セットアップ

```bash
git clone <repository-url>
cd masterdata
npm ci
cargo build
```

GUI依存のインストール方法はTauriの各OS向け公式手順に従ってください。依存関係を確認するには次を実行します。

```bash
cargo xtask doctor
```

## 開発コマンド

```bash
# CLIでdevelopment fixtureをコピーし、project-infoとvalidateを実行
cargo xtask cli

# fixtureをコピーしてTauri GUIを起動
cargo xtask gui

# fixture discovery -> validation -> C# plan -> .NET builder smoke test
cargo xtask test-integration

# 実際のMasterMemory v3 Source Generator -> binary -> reload/lookup spike
cargo xtask mastermemory-spike

# specification / RFC / change metadata, IDs, and relative links
cargo xtask check-specs

# implementation rationaleの明示referenceを検証
cargo xtask check-rationale

# commit前のspec / rationale / format / clippy / Rust test / frontend check / GUI compile / integration
cargo xtask check-all

# development projectをfixtureから再生成
cargo xtask dev-reset
```

直接CLIを使う場合:

```bash
cargo run -p masterdata-cli -- init ./my-masterdata --id game.masterdata --name "Game Master Data"
cargo run -p masterdata-cli -- --project fixtures/minimal project-info
cargo run -p masterdata-cli -- --project fixtures/minimal validate
cargo run -p masterdata-cli -- --project fixtures/minimal build --dry-run
```

`build --dry-run`はvalidationとC#生成計画を表示します。通常の`build`は全stageが成功するまで最終generated outputへ書き込まず、schema-drivenのMasterMemory binary生成が未実装の現在は明示的なエラーで停止します。失敗したbuildがpartialな最終成果物を残すことはありません。独立したtechnical spikeは `cargo xtask mastermemory-spike` で実行できます。

## ProjectとYAMLの規約

project rootは`masterdata.toml`で識別します。明示されたproject pathが最優先で、指定がなければ現在ディレクトリから親方向へ探索します。Unityの`Assets/`検出だけでproject identityを決めません。

YAMLの配置ディレクトリはtable identityを決めません。source rootは探索範囲に過ぎず、各ファイルの`kind`と`table`が意味を宣言します。同じtableのdataは複数ファイルへ分割できます。

```yaml
kind: data
table: item
records:
  - id: 1001
    name: Potion
```

identity/compatibilityの仕様は[互換性仕様のindex（compatibility specification index）](docs/specs/compatibility/README.md)を参照してください。table identityは現在`Implemented`、Field Identityは旧modelのretired historyとMessagePack `key` ownerへのroutingを含む`Deprecated`、Enum/Flags type semanticsとTable/Key semanticsも`Approved`です。Index identityは`Draft`として管理しており、実装未完了の領域を含みます。

## リポジトリガイド

- [プロダクトビジョン（Product vision）](docs/product/vision.md)
- [用語（Terminology）](docs/product/terminology.md)
- [仕様index（Specification index）](docs/specs/README.md)
- [仕様ワークフロー（Specification workflow）](docs/contributing/specification-workflow.md)
- [repository skills（repository skill）](skills/)
- [GUIアプリシェル（GUI app shell）](docs/gui/app-shell.md)
- [アーキテクチャ判断（Architectural decision）](docs/adr/)
- [agent向けルール（Agent rules）](AGENTS.md)
- [最小fixture（Minimal fixture）](fixtures/minimal/README.md)

仕様はコードと同じくGit管理する第一級成果物です。未確定事項は各文書の`Status: Draft` / `Proposed`または`Open Questions`に残し、AIが自動で`Approved`へ進めません。

## ライセンス

このリポジトリはMIT Licenseです。
