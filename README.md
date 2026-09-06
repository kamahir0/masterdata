# masterdata

`masterdata` は Unity + MasterMemory を対象にした、YAML-firstのローカルファーストなマスターデータ開発システムです。CLIとTauri GUIは、同じRust application serviceとcoreを直接利用します。

このリポジトリはschema-drivenなMasterMemory binary buildを中心に、project discovery、typed YAML AST、Type System、Table/Key
validation、Build Selection、C#生成、.NET bridge、artifact-set receipt、CLI、Tauri app shellを同じworkflowで扱います。Reference、
builder cache、released binary compatibility、Unityへの最終配置は別scopeです。observable contractとlifecycleは各canonical
specification、current implementation realityはcode / tests / Gitを参照してください。

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

- `masterdata-core`: project解決、`masterdata.toml`、typed YAML document model、Type Systemのsymbol resolution/validation、build plan
- `masterdata-codegen-csharp`: resolved Type System modelとTable row scaffoldからC#を生成する独立境界
- `masterdata-dotnet`: .NET SDKとstaged MasterMemory builderを呼び出す唯一のRust adapter
- `masterdata-app`: CLIとTauriが共有するproject/validate/build orchestration。domain semanticsは持たない
- `masterdata-cli`: application serviceを使うCLI。GUIやCLIにdomain logicを重複させない
- `apps/gui`: TypeScript + React frontendとTauri v2 shell。backend commandはapplication serviceを呼ぶ
- `xtask`: repository固有の開発コマンドをRustで集約

最終的なbuild pipelineは次を目指します。

```text
resolve project
  -> load config
  -> discover YAML
  -> parse schema/data/type
  -> semantic validation and resolve types
  -> resolve indexes/references
  -> generate C#
  -> schema hash
  -> compile schema-specific .NET builder
  -> build MasterMemory binary
  -> validate binary
  -> hash staged canonical C# and binary
  -> stage and validate artifact-set receipt
  -> atomic canonical artifact-root switch
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

# fixture discovery -> validation -> production binary build -> .NET bridge smoke test
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

`build --dry-run`はvalidationとC#生成計画を表示します。通常の`build`は、project-localな`.masterdata/output/`へ完全なcanonical artifact set（`csharp/`、`masterdata.bytes`、`.masterdata-artifact-set.json`）をstageし、実MasterMemory builderとbinary reload validation、receipt validationが成功した後にroot単位で切り替えます。`build --publish`はfull buildが成功した場合だけ、同じreceipt-valid artifact setをstandalone `publish`と同じsemanticsでexternal targetへ配布します。`build`単体は外部publish targetを暗黙には更新しません。既存artifact setのread-only receipt validationもapplication serviceから利用できます。旧`build.output`と`build.binary_output`はmigration diagnosticで拒否されます。独立したtechnical spikeは `cargo xtask mastermemory-spike` で実行できます。

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

identity/compatibilityのcanonical contractとlifecycleは[互換性仕様のindex（compatibility specification index）](docs/specs/compatibility/README.md)と
各owner specificationを参照してください。READMEはimplementation statusの一覧を保持せず、実装の現状はcode / tests / Gitから
確認します。

## リポジトリガイド

- [現在の開発目的（Current Objective）](docs/current-objective.md) — current priorityの入口。仕様や実装statusの代替ではない
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
