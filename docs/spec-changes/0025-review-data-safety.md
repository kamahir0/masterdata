# レビューで判明した編集対象と保存の保護

Status: Applied

## Basis / scope

Human Requirement: 2026-10-23、全体レビューで見つかった7件をすべて対応する。
Canonical owner: Desktop EDIT-002 / EDIT-005 / DATA-001、およびBrowserのproject handle永続化。
既存の対象同一性、現在file保存、未知項目保護、path containment、局所診断はbug fix。本提案は期待結果が未具体化だった失敗時の扱いを補う。保存形式や権限の拡張、実データの破棄、公開releaseは対象外。

## Behavior / Agent Decision

- nested editorは行・祖先配列要素のsession IDを追跡し、並替え・挿入・Undo / Redo後も同じ対象を編集する。対象が削除された場合は閉じ、同じindexの別要素へ移らない。
- 現在file保存は入力のblur確定後の選択を対象にする。新規名の確定で開いたfileも同じSaveで保存できる。
- sourceの新規作成・rename・移動先には、読込から除外されるdotで始まるsegmentを認めず、理由を示してdraftを保持する。保存側でも拒否する。既存の隠し項目は読込対象外のまま変更しない。これは保存後に消えたように見える状態を防ぐ判断で、Browserの既存制限に揃える。
- project相対pathはすべてのsegmentでWindows drive prefixとbackslashを拒否する。絶対path・上位参照・symlinkを拒否する境界を維持する。
- folder削除・移動の保存は、削除予定のdirectory内に保存対象外のfile / directory / symlinkがあれば書込前に拒否する。削除途中の失敗も保存成功にせず、今回変更したfileと削除済みfolderを復元するか復旧情報を保持し、baselineとdraftを保存済みにしない。削除は非再帰で行う。
- 最近のfolderのIndexedDB処理は補助機能とする。open / transactionのerror・abortを処理し、永続化失敗でも開いたprojectを使える。connectionを終了する。最近のfolderを読めなければ未登録として扱う。
- null・scalar・配列等の不正なEnum / Flags member、reference、keyをsourceとelement位置に結び付けて診断し、他の診断を維持する。意味不正でも保存を妨げず、rawで回復できる。

## Acceptance / validation

nestedを開いて行と祖先配列を並替え・挿入し元対象を編集、削除時の閉鎖、Undo / Redoを検証する。新規名入力中のSaveで新fileがdiskへ保存され他draftが残ることをGUIで確認する。dot名・埋込drive prefix・backslashはsession / native保存で拒否し元fileを保持する。未知file付きfolder削除は何も書換えず失敗し、未知fileを外せば再保存できる。IndexedDBの非同期error・abortと同期throwを注入してopen / saveを継続する。不正member / reference / key混在の局所診断と他診断の維持を確認する。Chromium / WebKit、Rust native tests、通常tests、全OS CIで検証する。

## Unresolved / safety

None。上記は依頼内の編集・データ保護の具体化で、新しい権限・アクセス先を追加しない。dot source名は保存成功後の再読込不一致を修正する制約。既存隠しfileは削除しない。

## Review / approval

別passで、対象削除後に別要素へ誤書込しないこと、blur後の保存対象、未知fileのpreflightと削除途中のrollback、hidden既存fileの非変更、IndexedDB abortの完結、診断の位置と継続性を確認した。Blocking Issues / Non-blocking Issues / Questions: None。Approved as Proposed: Yes。Autonomous approval eligible: Yes（選択済み7件の修正と既存境界の保護、権限拡張なし）。Human gate: None。

Approval mode: Agent-autonomous
Objective / request basis: レビュー全件対応のHuman委任。review後にorchestrationが承認。
