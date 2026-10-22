# Current Objective

## Objective

リリースビルド（CI/CD）および関連ワークフローを最適化し、ビルド所要時間を大幅に短縮する。Rust/Cargo ビルドキャッシュの導入、ステップ重複（`npm run build` の二重実行）の排除、および CPU コア並行度の見直しを行い、成果物の完全性と安全性を担保したままビルドパイプラインを高速化する。

## Completion boundary

仕様変更提案（0022-release-build-optimization）の作成・別pass review・自律承認、`docs/specs/distribution.md` の更新、`.github/workflows/release.yml`（および `workspace.yml`）の最適化実装、ローカル整合性検証（`python3 tools/check_workspace.py`, `git diff --check`, `npm run check` 等）、self-review、通常commit / push。

## Non-scope

配布成果物の構成（Desktop バンドル・CLI アーカイブ・SHA256 チェックサム）の変更、署名要件の変更、GUI / CLI コア機能の振る舞い変更。
