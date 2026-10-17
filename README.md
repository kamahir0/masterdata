# MasterData

MasterDataを編集するデスクトップGUIを、日常の作業体験から設計し直すプロジェクトです。現在は開発ワークスペースの立ち上げ段階で、アプリケーションの実装はまだありません。

## 開始する

- 開発者・AI agentの入口: [AGENTS.md](AGENTS.md)
- 製品の方向性: [Product Direction](docs/product/direction.md)
- 今回の目的と完了範囲: [Current Objective](docs/current-objective.md)
- 再開地点: [Development State](docs/execution-state.md)
- 開発の進め方: [Execution Workflow](docs/execution-workflow.md)
- 仕様の整理・レビュー・承認: [Specification Workflow](docs/contributing/specification-workflow.md)
- 現在の製品仕様: [Specifications](docs/specs/README.md)

ワークスペースの整合性を確認します。

```sh
python3 tools/check_workspace.py
git diff --check
```

製品の実装・テスト・起動コマンドは、技術選定と実装に合わせて追加します。

## 旧プロジェクトとの境界

旧リポジトリは [masterdata-legacy2](https://github.com/kamahir0/masterdata-legacy2) として独立して保持しています。引き継ぎの境界は[開発ワークフロー](docs/execution-workflow.md#legacy-boundary)が所有します。

## License

[MIT](LICENSE)
