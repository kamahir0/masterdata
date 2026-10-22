# ADR-0001: GUIが編集sessionを所有し、永続化・Build・CLIはRustコアで共有する

Status: Accepted

採用根拠: Desktop Authoringの別pass review後、2026-10-17のHuman委任および2026-10-21のHumanによるRust採用決定に基づく。日常のGUI編集レスポンス（0ms同期更新・未完成入力保持）と、ネイティブ永続化・CLIの単一バイナリ実行を両立する。

## 問題

旧プロジェクトでは一回の厳密なdomain commandとGUIの一操作を近づけたため、未完成の定義を作る途中にmodalや全体検証を要求し、エラー修正まで拒否しやすかった。frontendの見た目だけを変えても解消しない。一方、AIエージェントやCIが呼び出すCLIやファイル永続化・Build基盤にNode SEAサイドカーを要求すると、起動オーバーヘッドとバイナリ肥大化が生じる。

## Decision

2026-10-18 Human requestによりdesktop shellをTauriへ移行し、2026-10-21 Human decisionによりバックエンドコアとCLIにRustを採用する。TypeScript + React / Ant Designは維持し、rendererのimmutable Project draftがGUIの操作と履歴を所有する。意味検証はpureなmoduleとして分離し、GUIではbackgroundで即座に実行する。UIは同じ値入力componentを表・nestedで使い、TauriのRustバックエンドはopen / create / sample / save / build / publish / revealをインプロセスで提供する。

保存済みbaseと編集historyを分ける。GUIのdataはメモリ内の値として変更し、YAMLへのpatch / serializationはSaveまたは明示text表示時に行う。これにより入力やfolder開閉でdisk / 全量parseやIPC往復を待たない。snapshotは変更していないsource / recordを共有し、履歴は有限にする。

型変更時に入力を保持するため、draftを「その型で常にvalidな値」だけに制限しない。shapeと意味の診断は編集の受付から分ける。保存時の競合検出と安全性、Build時の厳密検証はそれぞれの境界で行う。

YAMLとMasterMemoryの利用目的は維持する。CLI（`masterdata`）とTauriネイティブバックエンドは `src-tauri` のRustコア（`masterdata.toml` 読書き、コメント・クオート・CRLF・64bit整数・配列順を保持するYAML AST処理、スキーマ・型・レコード操作、保存トランザクション、.NET / MasterMemoryによるC#・バイナリ生成とreload検証、Publish）を共有し、Node.jsランタイムやNode SEAサイドカーに依存しない。

## Browserへの拡張（2026-10-22）

[Browser Authoring](../specs/browser.md)の承認に基づき、React Session / YAML serializer / TOML contract / UIを共有し、能力に応じて入口を分岐する。Native I/O・Build / PublishのRustコアは維持する。Browser用にRust / WASMを導入してNativeのtransactionを模倣するより、選択したDirectoryHandleに限定したI/O adapterでWeb APIの非atomicな複数file保存と回復を明示する方が、静的配信と既存Desktop / CLIの品質を両立できる。

原文と予定文を含むjournalをsource更新前にclose・再読込し、全pathの事前検査と直前比較、失敗時の条件付きrollback、再open時の復旧を行う。Web Locksは同一originの競合を直列化するが、外部writerのOS排他を保証しない。未保存draftの自動永続化は行わない。これらの制約をNativeと同等のatomic性であるかのように隠さない。

## 比較した選択

- GUIのセル入力ごとにRustへIPC往復する案: キー入力・ドラッグ並替え・UndoのたびにシリアライズとIPC待機が発生し、未完成入力の保持も複雑になるため採用しない。
- Node SEAサイドカーの継続: CLIの単一バイナリ高速起動とDesktopバンドルの簡素化においてRust直接実装が優れるため退役させる。
- 独自HTML UI: 統一された入力・keyboard・テーマ・motionを再実装するため採用しない。

## 根拠

[Desktop Authoring](../specs/desktop.md)、[CLI Authoring and Automation](../specs/cli.md)、2026-10-17〜2026-10-21のHuman指示。Tauriの[command境界](https://v2.tauri.app/develop/calling-rust/)、[WebView](https://v2.tauri.app/reference/webview-versions/)、Ant Designの[Table](https://ant.design/components/table/)と[theme / motion](https://ant.design/docs/react/customize-theme/)を確認した。
