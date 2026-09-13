# ローカルGUI workflow

## 目的と前提

現在のcheckoutからDesktop GUIを検証、Tauri package生成、per-user install、production appのlaunch smokeまで行うdeveloper convenienceです。正式なrelease distribution contractではありません。Rust stable、Node.js 20以上、npm、`npm ci`済みの依存関係が必要です。macOSではXcode Command Line Tools、WindowsではWebView2とVisual Studio C++ Build Toolsが必要です。`cargo xtask doctor`で確認してください。

## command

```text
cargo xtask app dev       # fixtureを準備して既存Tauri開発モードを起動
cargo xtask app verify    # frontend、GUI Rust、Tauri compileのfocused preflight
cargo xtask app package   # production Tauri packageを生成しtarget/local-distへ集約
cargo xtask app install   # 集約済みpackageをper-user領域へ配置
cargo xtask app smoke     # install済みproduction executableを起動し2秒生存確認
cargo xtask app reinstall # verify -> package -> install -> smoke -> launch
```

macOSでは `scripts/local-app/app.command` をダブルクリックできます。Terminalからは `sh scripts/local-app/app.command` も使えます。必要なら一度 `chmod +x scripts/local-app/app.command` を実行してください。Windowsでは `scripts/local-app/app.bat` をダブルクリックするかcmdから実行します。どちらもscript自身の場所をrootとして扱い、failure時にwindowがすぐ閉じないようにします。

## package / install

`target/local-dist/` はpackage phaseで毎回消去・再生成されるdeveloper向け集約先です。macOSでは `.app` を `~/Applications/masterdata-local.app` へ置き換えます。WindowsではTauriが生成したrelease `.exe` を `target/local-dist/windows/` に集約し、`%LOCALAPPDATA%\Programs\masterdata-local\masterdata-gui.exe` へコピーします。管理者権限やsystem-wide `/Applications`は要求しません。Windowsのlocal installはportable executable deploymentです。installer policyや正式版identityをこのworkflowで固定しないためです。

## failure / troubleshooting

失敗時はphase名とstructured diagnostic codeを表示してnon-zeroで終了します。`doctor`、`npm ci`、OS依存（Xcode CLT、WebView2/C++ Build Tools）、Tauri build output、空き容量、per-user directory権限を確認してください。`smoke`はinstalled executableの存在、process起動、2秒間の即時crashを確認します。

## scope

Standalone Web、Connected Web、Native Host lifecycle本実装、正式release distribution、code signing、notarization、auto-update、store distributionは対象外です。正式distributionではidentity、installer policy、署名、更新チャネル、各OSの配布審査を別途決定する必要があります。
