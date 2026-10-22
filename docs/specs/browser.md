# Browser Authoring

Status: Proposed

Basis: 2026-10-22 Humanとの検討。Human Proposalは「静的配信」「ローカルのプロジェクトを選択」「編集中はインメモリ」「保存時だけI/O」。HumanがBrowser版ではBuild / Publishと関連UIを持たない方針を仕様化するよう依頼した。以下の対応ブラウザ、保存と復旧、UI分岐の具体化はAgent Decisionであり、Humanが個別に承認した実装詳細ではない。

関連仕様: [Desktop Authoring](desktop.md)、[CLI Authoring and Automation](cli.md)。本書は**ブラウザ実行環境の新規仕様**であり、既存のDesktop / CLIのBuild / Publishや保存規則を変更しない。実装済みを意味しない。

## 目的・対象作業・非対象

インストール不要の静的Webページから、利用者自身のローカルMasterDataプロジェクトを開き、既存Desktop版と同じ基本操作でデータ・型・構造を編集し、明示的に保存する。ゲーム向け成果物の生成・配置は既存CLI / Desktopに委ねる。

代表作業は、URLを開く → プロジェクトルートを選ぶ → 表・型・参照やExplorerの項目を編集する → Undo / Redoで修正する → ファイルまたは全体を保存する → CLI / DesktopでBuild / Publishする、という流れ。

初期対象は**デスクトップ版Chrome / Edgeなど、必要なローカルDirectory Pickerと読み書きAPIを提供するChromium系ブラウザ**。HTTPS等のsecure contextで静的配信する。Safari / Firefox、モバイル、ネットワーク共有・クラウド同期特有のfilesystem挙動、Web上のBuild / Publish、サーバー保存・アカウント・同期、ローカル補助サービス、PWAインストール必須化は初期対象外とする。ブラウザ内で新規Projectを作る機能とsample作成は初期スコープに含めず、既存Projectを開いて編集することを優先する（Agent Decision）。

## WEB-001 — 配布とローカルProject選択

- ブラウザ版は静的アセットだけで配信でき、編集・保存にアプリ専用サーバーやローカル常駐プロセス、Node.js / .NETのインストールを要求しない。ブラウザ側のBuild機能は提供しない。
- 「プロジェクトを開く」は利用者の明示操作でローカル**フォルダ**を選ぶ。選択フォルダをProject rootとし、直下の`masterdata.toml`と`sources/`を読み込む。ファイル単体選択をProject root選択の代わりにしない。Tauri版のProjectとのファイル互換性を維持する。
- ファイル読込、YAML / TOMLの解釈、source一覧構築をProjectオープン時に行い、完了後に編集sessionを開始する。開く対象が不適切（設定がない、構文不正、権限不足など）なら、対象と理由を示し、現在のProject / 未保存draftを保持する。
- `sources/`のディスク上の固定名は変更しない。Explorerでは従来どおりその子をルートとして表示し、source / folderの通常操作を提供する。ルート外、許可のない場所、symlinkなど経路の安全性を検証できない対象へ書き込まない。
- 必要APIを使えないブラウザでは編集可能なふりをせず、対応環境を案内する。取得したProjectデータをアプリのバックエンドへ送信しない。静的アセット配信以外のネットワークサービスを編集・保存経路へ組み込まない。

## WEB-002 — 編集sessionと操作体験

- 既存`src/core/`のsnapshot / Sessionを共有する。セル、record、field、type、nested値、nullable / array、参照、Explorer、検索、tab、診断、Undo / Redo、focus、motion等の編集操作はDesktop仕様の観察可能な挙動を基本として維持する。
- **Projectオープン時の初回読込の後、通常の入力・選択・folder開閉・並替え・作成・rename・move・delete・Undo / Redoではfilesystem I/Oを行わない。** メモリ上のdraftに反映し、必要な検証はメモリに対して行う。ディスク反映は明示Save時のみ。明示的なProject再オープン等は新たな読込操作として扱う。
- 編集中の追加・移動・rename・削除は保存まで仮想的なProjectパスの変更であり、実ディスクへ逐次反映しない。保存の成功通知まで編集履歴とdirtyを保持する。
- ブラウザの通常のキー操作と競合するショートカットは、入力中のIMEやフォーカスを妨げない範囲で扱う。表示・テーマ・Editor設定はProject dirtyから独立させる。

## WEB-003 — 明示Saveとファイル互換性

- 「保存」/ Cmd・Ctrl+Sは現在file、「すべて保存」/ Cmd・Ctrl+Shift+Sは全変更を対象にする。対象外のsource、folder、設定を暗黙に保存しない。保存中の追加入力は保存開始時点のsnapshotと区別し、成功後も新しいdirtyを残す。
- Save時だけ変更対象のディスク内容とpathを確認し、必要なfolder / fileの作成、書込み、移動に相当する新pathの作成と旧pathの削除を行う。実ファイルシステムの任意パスrename機能を前提にしない。保存前のExplorer操作ではディスクに変更を加えない。
- `masterdata.toml`、`sources/`のYAML、相対path、拡張子、型・値・tags・profiles・publish設定はDesktop / CLIと互換にする。BrowserでBuild / Publish UIを隠しても、`config.profiles`と`config.publish`の値を欠落・初期化・勝手に書換えない。
- 編集したYAMLはDesktopの仕様と同等に、コメント、quote、順序、改行、64bit整数、raw syntax errorの扱いを維持する。未変更ファイルや未変更設定テキストは無用に再書込みしない。TOMLで名前等を変更した場合、未知キーを維持する既存規則に従う。
- データの意味的な不整合はSaveを妨げず、Build / Publishは別工程として扱う。保存成功時は保存済み状態を更新し、Undo履歴を消さない。

## WEB-004 — 競合・複数ファイルSave・中断回復

- Save対象の全pathについて**書き込み前の事前検査**を行う。オープン時または直近成功Save時の基準と現在のディスク内容を比較し、外部変更、削除、同名衝突、権限喪失を検出したら、黙って上書き・削除せず、理由と再オープン等の回復手段を示す。保存直前の再確認も行う。別tabで同じProjectを編集した場合も競合対象とする。
- BrowserのファイルAPIに**複数ファイル一括の原子的commitはない**ことを前提とする。Save Allを「一つのatomic write」と説明しない。適用前に変更内容・原文を復旧可能な形で確保し、複数対象の途中失敗時には安全なrollbackを試みる。復旧不能なら変更済み / 未変更 / 要回復のpathと退避データを保持・提示する。復旧情報を消す前に、実ファイル状態を検証する。
- 書込み・削除の部分成功、ページ終了、ブラウザ強制終了、ディスク容量不足、権限取消しのあとでも、古い基準で次のSaveを安全とみなさない。未解決の保存トランザクションがある場合は通常保存を進めず、状態照合・復旧手順に導く。
- 単一fileの書込みストリームがcloseまで反映されない性質と、複数fileにまたがる原子性の欠如を区別する。Native側と同等の**データ保護の目的**を維持するが、Browser上で同等のOSレベル原子性が存在するとは主張しない。
- どの対象が保存されたか検証できない場合、完了通知・dirty解除・Project切替を成功扱いにしない。詳細な保存プロトコル、復旧データの形式・置場、既存`.masterdata/`との共存は実装設計時に確定し、実ブラウザで故障注入して確認する。

## WEB-005 — 権限、タブ終了、Project再開

- フォルダへのアクセスはユーザーが選択・許可した範囲に限定する。権限要求と再要求はブラウザのユーザー操作要件に従う。許可を失っても編集中のdraftを黙って破棄しない。
- フォルダハンドルを保存して次回の再選択を軽減してよいが、**ハンドルの保持と権限の永続性を同一視しない**。再開時はアクセスを検査し、必要ならユーザーに許可・選択を求める。保存できたハンドルを根拠に自動書込みしない。初期版では直近Project内容そのものをIndexedDB等に永続化しない。
- 未保存変更があるとき、アプリ内のProject切替・tabを閉じる操作は「すべて保存 / 保存せず終了 / キャンセル」を示す。ブラウザタブのreload / 閉じる / navigationでは利用可能な離脱警告を使うが、ブラウザやOSの強制終了で必ず確認できるとは表現しない。
- 警告を通さないクラッシュで未保存のメモリdraftが消える制約を明示する。自動保存、復旧用の編集中draft永続化、背景file監視は初期版に含めない。

## WEB-006 — Browser固有UIと機能境界

- BrowserではBuild / Publishの実行コマンド、toolbar button、split menu、profile選択、進捗・結果panel、成果物の表示・Reveal、Publish先編集欄など、**Build / Publish専用UIを表示しない**。無効状態のbuttonを置いて代用しない。ショートカットやメニュー、起動パラメータからも実行できない。
- `masterdata.toml`のBuild / Publish関連値は保持するが、Browserでは編集入口を持たせない。recordの`$tags`等、元データそのものの編集は維持する。Project名とBrowserで変更可能な一般設定は編集できる。
- ネイティブのウィンドウ装飾、OSアプリメニュー、ファイルReveal、**絶対pathのコピー**等、ブラウザの権限で正しく成立しないUIは表示しないか、明示的なProject相対path表現へ置き換える。絶対pathを推測・捏造しない。
- ビルド成果物は既存の`masterdata build` / `masterdata publish` CLI、またはDesktopから生成・配置する。Browser版のSaveがBuildやPublishを暗黙実行することはない。Build / Publishのための別サービス・WASM・補助プロセスは導入しない。
- Desktop版は現在のBuild / Publish UIと機能を保持する。共通UIを利用する場合も、ランタイム能力に基づく分岐がDesktop側の機能消失を起こさない。

## Acceptance / validation

1. HTTPSで配信した静的アプリを対応ブラウザで開き、ユーザー操作で選んだ既存Projectを読み込める。非対応ブラウザ、picker取消、設定欠落、不正TOML、アクセス拒否では既存draftを失わず理由が分かる。
2. ネストしたfolderと複数sourceのProjectで、Excel風のセル操作、型・参照・raw修復、source / folderのcreate・rename・move・delete、Undo / RedoがSaveまで実ディスクに触れず動き、Desktopと同じ操作結果・focus・dirty表示になる。
3. 現在file保存と全保存が正しい対象だけを書込み、同じProjectをCLI / Desktopで開ける。コメント、quotes、CRLF、64bit unsigned上限、空collection、未知TOMLキー、未変更fileを含む回帰fixtureで往復一致・非対象不変を確認する。
4. 外部編集、別tab競合、同名path、書込み途中での失敗・権限取消し・再読込を再現し、黙った上書きや誤った保存成功がなく、部分反映後の復旧情報・手順が残ることを確認する。**保存の保護・回復試験が成立するまで書込み可能なBrowser版を完成扱いにしない。**
5. 保存中の追加入力が失われず、dirtyとUndoが一致する。未保存状態でのアプリ内切替は選択で保護され、ブラウザ標準の離脱警告が利用可能な場面で動作する。
6. Browser画面とmenu / keyboard操作にBuild / Publish関連入口や設定欄がなく、設定ファイル上のprofiles / publishは保存前後で変わらない。Tauri版とRust CLIでは既存Build / Publishが引き続き動作する。
7. ChromiumのUI回帰、File System Accessのmockによるfocused testsだけでなく、Chrome / Edgeの実ローカルフォルダで権限・保存・中断復旧の代表作業を手動検証する。未検証環境を対応済みと報告しない。

## 実装上の判断と未解決

- Agent Decision: Browser専用のファイルI/O adapterを設け、既存のReact編集sessionとできる限りのUIを共有する。Tauri IPCやRust native coreをブラウザへ直接持ち込まず、現行のDesktop / CLI資産を変更しないことを優先する。
- Agent Decision: 初期対象を対応デスクトップChromium系に限定し、`showDirectoryPicker` / `FileSystemDirectoryHandle` / `createWritable`相当の機能をfeature detectionで確認する。利用者による明示的な権限付与を前提とする。
- 実装開始前に詰めるべき技術検証: Rust側と同等のデータroundtripをJS側で成立させる方法、Browser用transaction / recoveryの具体的な設計と耐障害試験、複数tab・外部writerに対する競合限界、permission再要求の実機UX。安全性を実現できない場合は保存機能のscopeを縮小して勝手に完成扱いにせず、仕様判断へ戻す。
- 本書は現行Desktop仕様を暗黙に改定しない。Browserに固有の例外はここでのみ定義する。

参考（APIの前提）: [File System API](https://developer.mozilla.org/en-US/docs/Web/API/File_System_API)、[showDirectoryPicker](https://developer.mozilla.org/en-US/docs/Web/API/Window/showDirectoryPicker)、[createWritable](https://developer.mozilla.org/en-US/docs/Web/API/FileSystemFileHandle/createWritable)、[FileSystemHandle permissions](https://developer.mozilla.org/en-US/docs/Web/API/FileSystemHandle/queryPermission)。

## Review

Refinement後のself-challenge（2026-10-22）。意図との整合、Desktop / CLIとの境界、Save時のみI/O、機能の非表示と設定値保持、離脱・競合・部分失敗を確認。Blocking Issues: None for the **Proposed仕様**。Non-blocking / implementation risk: Browser APIの非atomic複数file保存と実ファイルroundtrip、安全な復旧は実機検証が必要。Human gate: 本仕様の提案作成にはNone。実装承認・完成判定とは別。Approval: 未実施。
