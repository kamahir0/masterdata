# MasterData

ゲームのマスターデータを、表と型を行き来しながら直接編集するDesktop / Browserアプリ。React / Ant Designの共通部品とmotionを使い、未完成なデータも保持して編集できます。CLIとの共通化を編集モデルの前提にしていません。

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

「開く」ではプロジェクトルートの`masterdata.toml`を選択します。プロジェクト名・出力プロファイル・Publish先はこのファイル、データ本体は`sources/`内のYAMLに保存します。設定はGUIから変更でき、設定変更の保存時はTOMLを整形し直します（手書きコメント・書式の保持には未対応）。

「サンプルで試す」で冒険の道具箱を作業用コピーとして開けます。元のsampleは変更されません。表・列・Enum・Custom Typeの追加、配列・nullable・参照、Undo / Redo、保存、Build / Publishを試せます。

- 通常操作はメモリ上のdraftを変更します。保存してからもUndoできます。
- 作業途中のエラーは保存・他の編集を止めません。Build時に必要な整合性を検証します。
- Buildは現在のdraft snapshotからC# / MasterMemory binaryを生成し、.NETで実際にreloadして確認します。編集元の暗黙保存は行いません。
- Publishは設定した出力先へ、成功した成果物を配置します。Unityの起動や動作確認は別です。
- 保存時の外部変更を検出した場合は上書きを中止し、入力を保持します。保存backupはProjectの`.masterdata/transactions/`に残ります。

## ブラウザで利用する

静的配信したアプリをデスクトップ版Chromeで開き、「プロジェクトを開く」で`masterdata.toml`と`sources/`のある**フォルダ**を選びます。利用者側にNode.js / Rust / .NETや補助サービスは不要です。ファイルをサーバーへ送信せず、初回読込後の通常編集はメモリ上で行います。表・型・参照・Explorer・Undo / RedoはDesktopと同じ編集UIです。BrowserではBuild / Publish・出力設定UIを提供せず、既存CLI / Tauri版を使います。設定ファイル内のprofiles / publishは保持します。

配布する人は次の操作で静的アセットだけを生成できます（Node.jsとnpmが必要、Rust / .NETは不要）。

```sh
npm ci
npm run build:web
# dist/ 全体をHTTPSの静的ホストへ配置
# ローカル確認: npm exec vite preview -- --host 127.0.0.1 --port 5173
```

`dist/index.html`のfile://直開きは対象外です。HTTPS、またはlocalhost / 127.0.0.1で配信してください。初期対象はFile System AccessとWeb Locksを備えたデスクトップChromiumです。実機検証はmacOSのChromeで実施し、Edgeの実機・Safari / Firefox・モバイルは検証済みとしていません。必要APIがなければ編集できない理由を表示します。

- 保存 / Cmd・Ctrl+Sは現在file、すべて保存 / Cmd・Ctrl+Shift+Sは全変更です。作成・rename・移動・削除も保存までディスクへ反映されません。保存後もUndoできます。
- 外部変更や同名衝突があると保存を止め、入力を保持します。外部エディターや別originからの同時書込みをOSレベルで排他できないため、保存処理中は外部の変更を避けてください。
- 複数fileの保存は一括atomicではありません。適用前にProject内の`.masterdata/browser-transactions/`へ原文・予定文を退避し、検証後に更新します。途中失敗では復旧を試み、未解決なら通常保存を止めて復旧画面を表示します。journalには編集データが含まれるため、外部へ不用意に共有しないでください。
- 復旧画面でpathごとの状態を確認し、必要なら復旧JSONをダウンロードして「保存前へ復旧」します。外部の第三の内容は上書きせず、競合解消後に再試行できます。同じProjectの入力とUndoは保持し、復旧後に再保存できます。メモリsessionがなく全予定内容を検証できる場合は「保存済み内容を採用」も選べます。破損journalは自動実行せず、表示された場所とJSONを手動確認してください。
- 未保存の入力はメモリのみです。通常の離脱警告を提供しますが、クラッシュ・強制終了では未保存入力を復元できません。明示Saveの復旧journalと、編集中入力の自動保存は別です。フォルダハンドルも永続化せず、再開時は選び直します。

保存・回復の契約は[Browser Authoring](docs/specs/browser.md)が所有します。公開サイトへのdeploymentはこの実装に含めません。

## CLIで操作する

AIエージェントや自動化スクリプトからは、`npm run cli -- <command>`（または `npx tsx cli/main.ts <command>`）で同じプロジェクト設定（`masterdata.toml`）・YAMLデータ構造・診断・Build / Publishを扱えます。仕様は[CLI Authoring and Automation](docs/specs/cli.md)を参照してください。

```sh
npm run cli -- inspect --project samples/Adventure/masterdata.toml
npm run cli -- validate --project samples/Adventure/masterdata.toml
```

## 開発・検証

```sh
npm run check
npm test
npm run build
npm run build:web # 静的bundleのみ
npm run test:native
npm run test:ui
npm run pack -- --bundles app # macOS
# Windows / Linux: npm run pack -- --no-bundle
```

`npm test`は保存保護と実.NETのcompile / reloadを含みます。`.NET`なしの限定検証は`SKIP_NATIVE_BUILD=1 npm test`で行えます。`npm run test:native`はRust bridgeから同梱hostへの通信・64bit整数・保存競合・Build / Publishを検証します（先に`npm run build`）。`npm run test:ui`は作業用コピーを使い、Windowsは実際のWebView2に対応するChromium、macOS / LinuxはChromium / WebKitでUIを回帰確認します。初回はWindowsで`npx playwright install chromium`、他OSで`npx playwright install --with-deps chromium webkit`が必要です。これらのbrowser回帰はOS固有動作の証明ではないため、Tauri実アプリでも代表操作を確認します。Browser固有の自動GUI試験は実ChromiumのOPFS handle / streamを使い、pickerのみを差し替えます。実ローカルフォルダの故障注入は`npm run dev`で`/tests/browser-manual.html`を開き、専用の作業用コピーで確認できます。この検証画面は配布bundleに含まれません。

Tauriの成果物は`src-tauri/target/release/`以下です。通常利用にはbuild directoryのコピーを増やさず、上記scriptの固定install先を使います。単一起動なので再起動操作は既存windowを前面にします。install失敗時は既存版を保持・復旧します。macOSの一括処理では、中間の`.app`とsmoke用の一時登録を片付け、固定先の開発版だけを登録します。Rustのbuild cacheは保持します。`npm run pack`で手動packageだけを行った成果物は別途残ります。強制中断後に`.install-lock`が残った場合は、別installが動いていないことを確認してそのlock directoryだけを取り除きます。

React側が編集sessionを所有し、Tauriがwindow / menu / dialogを担当します。Desktop / CLIの保存transaction・検証・.NET生成はRustコア、BrowserのI/Oは選択したフォルダhandle内に限定したTypeScript adapterを使います。編集Session、YAML・TOML処理、UIを共有し、BrowserへネイティブのBuild / Publishを持ち込みません。設計理由は[ADR](docs/adr/0001-desktop-session.md)に記録しています。

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
