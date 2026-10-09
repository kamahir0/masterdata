# ローカルGUI workflow

現在のcheckoutからproduction Desktopをビルドし、ユーザー領域へインストールして起動するdeveloper用の入口。正式release distributionではない。

macOSでは[`app.command`](../../scripts/local-app/app.command)、Windowsでは[`app.bat`](../../scripts/local-app/app.bat)をダブルクリックする。Terminal / cmdからも実行でき、作業directoryに依存しない。

必要環境はNode.js 22以上とnpm、repository指定のRust toolchain、.NET SDK 8以上。macOSにはXcode Command Line Tools、WindowsにはVisual Studio C++ Build ToolsとWebView2が必要。

共通処理は`npm ci`、frontend build / type check、Desktop applicationのfocused Rust tests、Tauri production build、stagingでの2秒生存確認、per-user install、launchの順。`desktop-evidence` featureは有効化しない。Tauri CLIはnpm lockfileから導入するためglobal installは不要。出力先はrepositoryの`target/`に固定する。

インストール先:

- macOS: `~/Applications/masterdata-local.app`
- Windows: `%LOCALAPPDATA%\Programs\masterdata-local\masterdata-desktop.exe`（portable executable）

置換前のfailureでは既存版を保持する。置換失敗時は既存版を復元し、復元にも失敗した場合はbackupのpathを表示して保存する。起動中の既存GUIは終了しない。Windowsで置換できない場合はGUIを閉じて再実行する。Project sourceとuser preferencesは削除しない。

macOSのlaunchはbundle内executableを直接起動し、native .NET invocationで必要なshellの`PATH`を引き継ぐ。smokeが起動したscratch processだけを終了し、最後に起動したGUIを残す。smokeは即時crashの確認であり、画面操作やproduct conformanceの証明ではない。

```sh
sh scripts/local-app/app.command --no-launch
node scripts/local-app/app.mjs --help
node --test scripts/local-app/app.test.mjs
```

`--no-launch`はinstallまで行い、最後のGUI起動を省略する。Windowsでも`app.bat --no-launch`を使える。失敗時はnon-zeroで終了し、ダブルクリックで開いたwindowがすぐ閉じないように待つ。
