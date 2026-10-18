# Current Objective

## Objective

現状のUI・操作・motionを維持してElectronからTauriへ移行し、ローカルcheckoutのbuildから識別可能な開発版の固定先install・起動までを.command / .batで一括実行できるようにする。

選択根拠: 2026-10-18 Human request。通常の技術判断は既存の自律実装委任内で行う。

## Completion boundary

[Desktop Authoring](specs/desktop.md)と[Tauri移行提案](spec-changes/0002-tauri-development-install.md)の関連要件、focused回帰、macOS実アプリとinstallの確認、通常commit / push、各OS CI。

## Non-scope

公開release、UI再設計、CLI、実データ破棄、識別できない既存アプリの削除。
