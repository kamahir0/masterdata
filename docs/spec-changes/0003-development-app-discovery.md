# 開発版の起動入口を一つに保つ

Status: Approved

Approval mode: Agent-autonomous

## Basis / scope

2026-10-18 Human request: 既存インストール済みの整理と開発アプリbuild workflowの修正。画像ではインストール版とbuild成果物の計7入口が同じアプリ一覧に現れていた。

Agent Decision: 固定install先だけでなく、一括処理が生成・検証する中間.appとLaunch Services登録も管理する。既存の旧bundle整理はこのPCでの一回限りの退役作業とし、通常のinstallerに他のcheckoutや旧製品を走査・削除させない。

## Behavior

- macOSの一括build / installは、正常終了・通常の失敗とも自分が生成した中間.appを退役し、source / smoke用stagingの登録を解除する。固定先の最新版を登録して起動入口にする。Rust cache等は保持し、強制中断で残った自分の中間bundleは次回package前に片付ける。
- install失敗・起動中の拒否は既存版を保持する。後片付け失敗を成功扱いにせず対象pathを示す。無関係なbundleや同名の別製品を名前だけで削除しない。
- 今回の旧版はpath / bundle identity / source checkoutを確認し、終了後に復元可能な形で退役する。未保存入力はユーザーの選択なしに保存・破棄しない。projectと設定は消さない。
- macOS全体のLaunch Services / Spotlight DBの初期化や他のアプリの再登録は行わない。Windowsの固定install / shortcutは維持する。

## Acceptance

成功・smoke失敗・package失敗で中間bundleとstagingが残らず、以前のinstallは失敗時に保持されることをfocused testで確認。一括処理を実機で反復し、正常終了後のアプリ一覧は固定先のMasterData [Dev]一つになることを確認する。Windowsの既存install test / CIも確認する。

## Review

Blocking Issues / Non-blocking Issues / Questions: None. Approved as Proposed: Yes. Autonomous approval eligible: Yes（選択済みObjective内、対象を自分の中間bundleと識別済み旧版へ限定し、失敗時の既存版とユーザー入力を保持する）。Human gate: None for workflow / recoverable retirement。未保存2件はHumanが破棄を選択。通常の終了dialogで保存しないを実行。
