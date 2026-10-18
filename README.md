# MasterData

ゲームのマスターデータを、表と型を行き来しながら直接編集するデスクトップアプリ。React / Ant Designの共通部品とmotionを使い、未完成なデータも保持して編集できます。CLIとの共通化を編集モデルの前提にしていません。

## アプリを起動する

開発・ローカルbuildにはNode.js 22.12以降、npm、Rust toolchain、OSの[Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)、.NET 8 SDK以降を用意します。完成したアプリの利用にはNode.js / Rustは不要です。Build機能には.NET SDKが必要です。

macOSは[scripts/local-app/app.command](scripts/local-app/app.command)、Windowsは[scripts/local-app/app.bat](scripts/local-app/app.bat)を実行すると、現在のcheckoutの依存install・検証・build・開発版install・起動まで行います。Gitのpullはしません。

- アプリ名: **MasterData [Dev]**。通常版とidentifier・設定を分離します。
- macOS: `~/Applications/MasterData [Dev].app`
- Windows: `%LOCALAPPDATA%\Programs\MasterData-Dev`。スタートメニューの`MasterData [Dev]`から起動できます。
- 再実行は固定先を更新します。起動中なら更新を止めるので、変更を保存して終了してから再実行してください。
- `./scripts/local-app/app.command --no-launch` / `scripts\local-app\app.bat --no-launch`でinstall後の起動を省略できます。初回buildはRust依存のcompileに時間がかかります。

rendererのhot reloadを使う開発起動は次の通りです。

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
npm run test:native
npm run test:ui
npm run pack -- --bundles app # macOS
# Windows / Linux: npm run pack -- --no-bundle
```

`npm test`は保存保護と実.NETのcompile / reloadを含みます。`.NET`なしの限定検証は`SKIP_NATIVE_BUILD=1 npm test`で行えます。`npm run test:native`はRust bridgeから同梱hostへの通信・64bit整数・保存競合・Build / Publishを検証します（先に`npm run build`）。`npm run test:ui`は作業用コピーを使い、Windowsは実際のWebView2に対応するChromium、macOS / LinuxはChromium / WebKitでUIを回帰確認します。初回はWindowsで`npx playwright install chromium`、他OSで`npx playwright install --with-deps chromium webkit`が必要です。これらのbrowser回帰はOS固有動作の証明ではないため、Tauri実アプリでも代表操作を確認します。

Tauriの成果物は`src-tauri/target/release/`以下です。通常利用にはbuild directoryのコピーを増やさず、上記scriptの固定install先を使います。単一起動なので再起動操作は既存windowを前面にします。install失敗時は既存版を保持・復旧します。強制中断後に`.install-lock`が残った場合は、別installが動いていないことを確認してそのlock directoryだけを取り除きます。

React側が編集sessionを所有し、Tauriがwindow / menu / dialogを担当します。保存transaction・検証・.NET生成は専用Node SEA hostを同梱し、stdioだけで接続します。Electronは使いません。Node同梱により最小Tauriよりサイズは増えますが、利用者に別runtimeのinstallは要求しません。設計理由は[ADR](docs/adr/0001-desktop-session.md)に記録しています。

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
