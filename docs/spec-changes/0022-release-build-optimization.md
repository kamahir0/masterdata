# リリースビルド（CI/CD）の高速化とビルドパイプライン最適化

Status: Approved
Approval mode: Agent-autonomous
Basis: 2026-10-22 ユーザーリクエスト「最適化して」に基づく。別passでintent fidelity、CI再現性、成果物の同等性・完全性、セキュリティ（アクションのハッシュ固定）をreviewし、Blockingなし。Human gate: None within delegated scope。

## 根拠と意図

- Human Requirement: リリースビルドの並行性・ビルド時間短縮に関する最適化（2026-10-22）。
- 背景:
  - 現在のリリースワークフロー（`.github/workflows/release.yml`）およびテストワークフロー（`.github/workflows/workspace.yml`）では、各OSマトリックスでの並行実行は行われているものの、Rust/Cargoの依存クレート（Tauri本体、WebKit/GTK、暗号系ライブラリ等）がキャッシュされず、毎回ゼロからフルコンパイルされている。
  - これにより1ジョブあたり18〜22分を要し、全体のリリース完了までに長時間を要している。
  - さらに、`CARGO_BUILD_JOBS: 2` によるCPU並行度の固定制限や、`release.yml` 内で `npm run build` が明示的ステップと `tauri build` 内の `beforeBuildCommand` の両方で重複して走る非効率が存在する。
  - 配布成果物（Desktopバンドル、CLIバイナリ、チェックサム）の同一性・完全性を維持したまま、これらを最適化してビルド所要時間を大幅に短縮する。

## 変更内容 (Agent Decision)

### 1. Rust / Cargo ビルドキャッシュの導入
- `.github/workflows/release.yml` および `.github/workflows/workspace.yml` に `Swatinem/rust-cache` を追加。
- `src-tauri -> target` のワークスペースマッピングを指定し、OS・Rustバージョン・Cargo.lockハッシュ単位で依存クレートを確実にキャッシュ。
- セキュリティおよびサプライチェーン安全基準に従い、コミットハッシュ固定を使用。

### 2. CPUコア並行度の最大化
- `release.yml` および `workspace.yml` から `CARGO_BUILD_JOBS: 2` の制限を撤廃。
- 各CIランナー（Ubuntu: 4 vCPU, Windows: 4 vCPU, macOS: 3-4 vCPU）の利用可能コア数をCargoに自動認識させ、並行コンパイルを最大化。

### 3. `release.yml` でのビルド重複排除
- `release.yml` の事前ステップ `Type check and web build` (`npm run build`) を、早期構文・型検査に専念する `Type check` (`npm run check`) に変更。
- Webアセット（`vite build`）およびCLI hostバイナリ（`prepare:host`）のビルドは、`pack:release` の `beforeBuildCommand` で一元的に1回のみ実行されるように整理。

### 4. ドキュメント（仕様書）の更新
- `docs/specs/distribution.md` の `DIST-003` にキャッシュ構成および最適化されたビルド手順を反映。

## Acceptance

1. `.github/workflows/release.yml` および `.github/workflows/workspace.yml` に `rust-cache` が適切に設定されていること。
2. `CARGO_BUILD_JOBS: 2` が削除され、ランナーのCPUコアが制限されていないこと。
3. `release.yml` で `npm run build` の二重実行が解消され、必要な型チェックが先行して行われること。
4. 生成される配布成果物（Desktopバンドル・単体CLIバイナリ・チェックサム）の収集ロジックに影響を与えないこと。
5. `python3 tools/check_workspace.py`、`git diff --check`、`npm run check` がすべて正常に通過すること。

## Verification Plan

- `python3 tools/check_workspace.py` による整合性検査
- `git diff --check` によるフォーマット検査
- `npm run check` によるTypeScript型チェック
- `cargo check --manifest-path src-tauri/Cargo.toml` によるRust側検査
- `.github/workflows/release.yml` および `.github/workflows/workspace.yml` のYAML構文・設定検証
