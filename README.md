# MasterData

ゲームのマスターデータを、表と型を行き来しながら直接編集するデスクトップアプリ。React / Ant Designの共通部品とmotionを使い、未完成なデータも保持して編集できます。CLIとの共通化を編集モデルの前提にしていません。

## アプリを起動する

Node.js 22.12以降、npm、Build用に.NET 8 SDK以降を用意します。

```sh
npm ci
npm start
```

「サンプルで試す」で冒険の道具箱を作業用コピーとして開けます。元のsampleは変更されません。表・列・Enum・Custom Typeの追加、配列・nullable・参照、Undo / Redo、保存、Build / Publishを試せます。

- 通常操作はメモリ上のdraftを変更します。保存してからもUndoできます。
- 作業途中のエラーは保存・他の編集を止めません。Build時に必要な整合性を検証します。
- Buildは現在のdraft snapshotからC# / MasterMemory binaryを生成し、.NETで実際にreloadして確認します。編集元の暗黙保存は行いません。
- Publishは設定した出力先へ、成功した成果物を配置します。Unityの起動や動作確認は別です。
- 保存時の外部変更を検出した場合は上書きを中止し、入力を保持します。保存backupはProjectの`.masterdata/transactions/`に残ります。

## 開発・検証

```sh
npm run check
npm test
npm run build
npm run test:ui
npm run pack
```

`npm test`は実.NETでのcompile / reloadを含みます。`.NET`がない環境での限定検証は`SKIP_NATIVE_BUILD=1 npm test`で行えます。`npm run test:ui`は作業用コピーを使うElectronの回帰テストです。Linuxでは`xvfb-run -a npm run test:ui`を使います。`pack`は`release/`へローカルアプリを作成します。

開発中のrendererは`npm run dev`で起動し、別terminalで`MASTERDATA_DEV_URL=http://127.0.0.1:5173 npm exec electron .`を実行できます。

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

## 旧プロジェクトとの境界

旧リポジトリは [masterdata-legacy2](https://github.com/kamahir0/masterdata-legacy2) として独立して保持しています。引き継ぎの境界は[開発ワークフロー](docs/execution-workflow.md#legacy-boundary)が所有します。

## License

[MIT](LICENSE)
