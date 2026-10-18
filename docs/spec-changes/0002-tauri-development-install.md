# Tauri移行と開発版の固定インストール

Status: Approved

Approval mode: Agent-autonomous
Review: 別passで未保存保護、固定identity、更新失敗、IPC権限、UI差分の検証を確認。Blocking / Questions: None。Approved as Proposed: Yes。Autonomous approval eligible: Yes（明示依頼と通常技術判断の委任内）。Human gate: None。

## 根拠・目的

2026-10-18 Human request: Tauriへ移行し、現状の外観・操作・motionを維持する。ローカルcheckoutからbuild / install / launchまで実行する.command / .batを用意し、開発版を識別でき、更新ごとに同名アプリが増えないようにする。

## 提案

- UI、編集session、YAML、保存・Build / Publishの振る舞いは維持し、ElectronをTauriへ置換する。描画エンジン差は実操作で検証・補正する。未保存内容を移行のために破棄しない。
- Agent Decision: 検証済みTSの保存transaction・検証・生成を、Node SEAの同梱専用hostとして再利用する。Rustはwindow・menu・dialogと限定IPCを所有する。編集modelはTSに一つだけ置き、Rustに複製しない。利用者にNode.jsを要求しない。Node同梱のサイズは最小Tauriより増えるが、保存安全性の不要な再実装を避ける。通信はstdioのみでnetwork listenerを設けない。
- Agent Decision: 開発用アプリのproduct nameは`MasterData [Dev]`、identifierは`dev.kamahir0.masterdata.dev`に固定する。通常版と設定・single-instanceを分離する。macOSは`~/Applications/MasterData [Dev].app`、WindowsはユーザーのLocalAppDataの固定directoryとStart menu shortcutへ上書き更新する。checkout / build番号を名前・identifierへ付加しない。
- rootの`app.command` / `app.bat`はcheckoutを変更・fetch / pullせず、依存install→check / focused tests→package→staging確認→固定先へrollback可能な置換→起動を行う。`--no-launch`を用意する。失敗時は既存installを維持または復旧し、エラーを表示する。
- 起動中の対象installを無断kill / 書換しない。更新を止め、未保存内容を保存して閉じてから再実行するよう案内する。通常起動はsingle instanceとし、二重起動は既存windowを前面にする。
- 任意の同名アプリや旧repositoryの成果物を名前だけで削除しない。build成果物は開発用installではなく、普段起動する固定先を案内する。以前の重複bundleの整理は今回所有するものを識別できた範囲だけで行う。

## Acceptance・検証

TSモデル・保存・.NET compile/reload/publish保護の回帰、実Rust bridgeのtest、Chromium / WebKitの既存UIシナリオ、macOSのTauri bundleでsample・編集・nested・scroll・motion・保存・Build・closeを確認。各OS CIでbuildと回帰を行う。macOSでscriptの固定installと再実行、失敗時保護・起動中拒否を検証し、Windows scriptもplatform分岐のtestとCIで検証する。取得していないOS実機操作は成功と扱わない。

公開release / 署名配布、UI再設計、CLI追加、実データ破棄は対象外。未決定事項なし。既存の自律判断委任内で承認可能。security capabilityは既存の限定native操作を超えて拡張しない。
