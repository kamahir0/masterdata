# 完全な業務・汎用ツールとしてのポジショニング刷新と「ゲーム」限定表現の排除

Status: Approved
Approval mode: Agent-autonomous
Basis: 2026-10-22 Humanの明示的選択（パターンB: 完全な業務・汎用ツールとして銘打つ）。別passでintent fidelity、汎用マスターデータツールとしての整合性、既存機能への影響、検証可能性をreviewし、Blockingなし。Human gate: None within delegated scope。

## 根拠と意図

- Human Requirement: パターンB（完全な業務・汎用ツール「型安全なマスターデータ統合エディタ（C# / MasterMemory）」として銘打つ）の選択（2026-10-22）。
- 背景:
  本製品の技術スタック（YAML/TOML、C# 12 / .NET 8、MasterMemory、MessagePack、Tauri、React）およびUI機能は、完全にゲームエンジン非依存かつ汎用的なリレーショナル・マスターデータ管理システムとして成立している。
  しかし、READMEやCLIヘルプ、package.json、ドキュメント類に「ゲーム専用」を想起させる銘打ち（「ゲームデータ管理」「ゲームマスターデータ編集」等）が残っており、業務システムや一般のC#開発への導入を狭める要因となっていた。
  これを改め、業務システム、Web API、デスクトップアプリ、ゲーム開発まで幅広く対応する「型安全なマスターデータ統合エディタ」としてポジショニングを刷新する。

## 変更内容 (Agent Decision)

### 1. パッケージ概要 (`package.json`)
- `"description": "ゲームマスターデータ編集ツール"` から `"description": "型安全なマスターデータ統合エディタ（C# / MasterMemory）"` へ変更。

### 2. CLIヘルプテキスト (`src-tauri/src/cli.rs`)
- `HELP_TEXT` の冒頭を「構造を理解するゲームデータ管理・編集・ビルドツール」から「構造を理解するマスターデータ管理・編集・ビルドツール」へ変更。

### 3. 設定画面の入力例プレースホルダー (`src/ui/settings.tsx`)
- C#出力先: `../Unity/Assets/MasterData/Generated` → `../Generated`
- バイナリ出力先: `../Unity/Assets/StreamingAssets/masterdata.bytes` → `../Data/masterdata.bytes`

### 4. ドキュメントおよびREADMEの刷新
- `README.md`:
  - 冒頭概要を「マスターデータを表と型を行き来しながら直感的に編集し、型安全なC#コードとMasterMemoryバイナリを自動生成するクロスプラットフォームのマスターデータ統合エディタ。Web API、デスクトップ業務システム、ゲーム開発まで幅広く対応します。」へ刷新。
  - 各種説明で「ゲーム向け」となっていた表現を「アプリケーション向け」「マスターデータ」へ統一。
- `docs/specs/browser.md`:
  - 「ゲーム向け成果物の生成・配置」を「C# / MasterMemory成果物の生成・配置」へ変更。
- `docs/specs/distribution.md`:
  - 概要説明文・メタデータを汎用マスターデータ統合エディタとして統一。
- `docs/product/direction.md`:
  - 製品像を「業務システムからゲーム開発まで対応するマスターデータ統合エディタ」として位置づけ直す。

## Acceptance

1. `package.json` および CLIヘルプ（`masterdata --help`）に「ゲーム」という限定語が含まれず、「型安全なマスターデータ統合エディタ」等の汎用的な説明になっていること。
2. 設定画面のPublish先プレースホルダーがUnity固定ではなく汎用的なパス例になっていること。
3. `README.md` および関連仕様書において、ゲーム専用ではなく業務システム・Web API等を含む汎用ツールとして位置づけられていること。
4. TypeScript型チェック、Unitテスト、CLIテスト、UIテストがすべて正常に通過すること。

## Verification Plan

- `npm run check` (TypeScript型チェック)
- `npm test` (Unitテスト52件)
- `cargo check --manifest-path src-tauri/Cargo.toml` (Rust側型チェック)
- `npm run test:native` (ネイティブCLIテスト)
- `npx playwright test -g "welcome|relationship"` (UIテスト)
- `python3 tools/check_workspace.py` & `git diff --check`
