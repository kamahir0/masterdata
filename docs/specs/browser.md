# Browser Authoring

Status: Implemented

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

- workbenchの面・背景階層・compact spacing・接続tab・補助panelはDesktop UI-003を共有する。Browser固有の機能境界を保ち、狭いwindowでも操作入口に到達できる。根拠と承認: [モダンworkbench](../spec-changes/0018-modern-workbench-surfaces.md)。

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
- どの対象が保存されたか検証できない場合、完了通知・dirty解除・Project切替を成功扱いにしない。下記の保存・回復プロトコルを実ブラウザで故障注入して確認する。

## WEB-005 — 権限、タブ終了、Project再開

- フォルダへのアクセスはユーザーが選択・許可した範囲に限定する。権限要求と再要求はブラウザのユーザー操作要件に従う。許可を失っても編集中のdraftを黙って破棄しない。保存方法menuと復旧dialogの「フォルダのアクセスを再許可」でpickerを開き、元と同じfolderかhandle identityを検査してhandleだけを更新する。取消・別folder・拒否は入力 / Undo / 保存基準を保持する。再許可でload / 保存を暗黙実行せず、次の明示保存・復旧は通常の競合検査を通す。根拠と承認: [フォルダ再許可](../spec-changes/0017-browser-reauthorization.md)。
- フォルダハンドルを保存して次回の再選択を軽減してよいが、**ハンドルの保持と権限の永続性を同一視しない**。再開時はアクセスを検査し、必要ならユーザーに許可・選択を求める。保存できたハンドルを根拠に自動書込みしない。初期版では直近Project内容そのものをIndexedDB等に永続化しない。
- 未保存変更があるとき、アプリ内のProject切替・file tabを閉じる操作はProject切替では「すべて保存 / 保存せず続ける / キャンセル」、file tabでは対象fileの「保存 / 保存しない / キャンセル」を示す。ブラウザタブのreload / 閉じる / navigationでは利用可能な離脱警告を使うが、ブラウザやOSの強制終了で必ず確認できるとは表現しない。
- 警告を通さないクラッシュで未保存のメモリdraftが消える制約を明示する。自動保存、復旧用の編集中draft永続化、背景file監視は初期版に含めない。

## WEB-006 — Browser固有UIと機能境界

- BrowserではBuild / Publishの実行コマンド、toolbar button、split menu、profile選択、進捗・結果panel、成果物の表示・Reveal、Publish先編集欄など、**Build / Publish専用UIを表示しない**。無効状態のbuttonを置いて代用しない。ショートカットやメニュー、起動パラメータからも実行できない。
- `masterdata.toml`のBuild / Publish関連値は保持するが、Browserでは編集入口を持たせない。recordの`$tags`等、元データそのものの編集は維持する。Project名とBrowserで変更可能な一般設定は編集できる。
- ネイティブのウィンドウ装飾、OSアプリメニュー、ファイルReveal、**絶対pathのコピー**等、ブラウザの権限で正しく成立しないUIは表示しないか、明示的なProject相対path表現へ置き換える。絶対pathを推測・捏造しない。
- ビルド成果物は既存の`masterdata build` / `masterdata publish` CLI、またはDesktopから生成・配置する。Browser版のSaveがBuildやPublishを暗黙実行することはない。Build / Publishのための別サービス・WASM・補助プロセスは導入しない。
- Desktop版は現在のBuild / Publish UIと機能を保持する。共通UIを利用する場合も、ランタイム能力に基づく分岐がDesktop側の機能消失を起こさない。

## Acceptance / validation

1. HTTPSまたはlocalhost等のsecure contextで配信した静的アプリを対応ブラウザで開き、ユーザー操作で選んだ既存Projectを読み込める。非対応ブラウザ、picker取消、設定欠落、不正TOML、アクセス拒否では既存draftを失わず理由が分かる。
2. ネストしたfolderと複数sourceのProjectで、Excel風のセル操作、型・参照・raw修復、source / folderのcreate・rename・move・delete、Undo / RedoがSaveまで実ディスクに触れず動き、Desktopと同じ操作結果・focus・dirty表示になる。
3. 現在file保存と全保存が正しい対象だけを書込み、同じProjectをCLI / Desktopで開ける。コメント、quotes、CRLF、64bit unsigned上限、空collection、未知TOMLキー、未変更fileを含む回帰fixtureで往復一致・非対象不変を確認する。
4. 外部編集、別tab競合、同名path、書込み途中での失敗・権限取消し・再読込を再現し、黙った上書きや誤った保存成功がなく、部分反映後の復旧情報・手順が残ることを確認する。**保存の保護・回復試験が成立するまで書込み可能なBrowser版を完成扱いにしない。**
5. 保存中の追加入力が失われず、dirtyとUndoが一致する。未保存状態でのアプリ内切替は選択で保護され、ブラウザ標準の離脱警告が利用可能な場面で動作する。
6. Browser画面とmenu / keyboard操作にBuild / Publish関連入口や設定欄がなく、設定ファイル上のprofiles / publishは保存前後で変わらない。Tauri版とRust CLIでは既存Build / Publishが引き続き動作する。
7. ChromiumのUI回帰、File System Accessのmockによるfocused testsだけでなく、初期検証対象Chromeの実ローカルフォルダで権限・保存・中断復旧の代表作業を手動検証する。Edge実機の証拠は別途取得するまで未検証と明記する。未検証環境を対応済みと報告しない。

## 保存と回復の具体化（Agent Decision）

- Browser I/Oは選択したDirectoryHandleの子のみを扱い、`sources/`と`masterdata.toml`、専用`.masterdata/browser-transactions/`以外を書き換えない。handleは初期版では永続化しない。再開はフォルダの再選択と明示操作から行う。必要API・secure context・Web Locksをfeature detectionする。
- 各保存は同一originのWeb Lockで直列化する。全変更pathの内容とfolder状態をpreflightし、保存直前にも再確認する。他originや外部エディターをOSレベルで排他できないため、再確認後の同時書込まで完全防止するとは主張しない。保存中は外部変更を避ける案内を利用文書に置く。
- 適用前に`.masterdata/browser-transactions/<id>.json`へversion・変更path・全原文・保存予定文・folder状態を記録し、close後の再読込で一致を検証する。退避作成に失敗したらsourceを変更しない。既存Nativeの`transactions/`とは分離する。これは明示Saveに伴う復旧情報であり、編集中draftの自動永続化ではない。
- 親folderを作り、新path / 内容を書込み後に旧pathを削除する。folder削除は空に限り、未知fileを再帰削除しない。削除対象folderに保存対象外のfileがあれば事前に止め、対象pathと理由を示す。対象fileはUTF-8の復号失敗を拒否し、lossyな上書きを行わない。
- 全対象の予定内容とfolder状態を再読込で確認してからjournalを削除し、保存成功・基準更新を通知する。失敗時は原文または今回の予定文と一致するpathだけを巻き戻し、全対象が元に戻ったことを検証してからjournalを削除する。外部の第三の内容は上書きしない。
- journalが残った場合は通常保存を止め、pathごとの現在状態（変更前 / 保存予定 / 外部変更 / 読込不能）を表示し、原文・予定文入りの復旧JSONダウンロードと「保存前へ復旧」を提供する。全対象が保存予定と一致し、そのProjectのメモリsessionがない場合のみ「保存済み内容を採用」も提供する。復旧は明示操作から権限を再要求し、外部変更を上書きしない。復旧失敗でもjournalを保持し、再試行可能にする。破損journalは自動実行せず場所を示して通常保存を停止する。
- 同じProjectのメモリsessionがあれば保存前への復旧後もdraft / Undoと元の保存基準を維持し、再保存できるようにする。次回保存でも全対象のdisk比較を省略せず、基準不一致は通常の外部変更として停止する。復旧dialogは閉じて入力へ戻れ、未解決の警告と再表示入口を残す。journalは消さず、影響するProjectへの通常保存を拒否する。通常のProject切替は未保存保護と保存中追加入力の確認を通す。変更根拠と承認: [live recovery](../spec-changes/0016-browser-live-recovery.md)。
- BrowserではCmd/Ctrl+S / Shift+S、O、Z / Shift+Z、Fを提供し、IMEとinputの通常Undo / 検索を妨げない。Cmd/Ctrl+Wはブラウザタブ操作のままとし、file tabはclose iconから閉じる。保存・復旧処理中またはdirty時にはbeforeunloadを登録する。
- 初期版はChromeで実機検証する。Edgeは同等APIのある対象候補だが、実機証拠を得るまで検証済みとは表現しない。HTTPSとlocalhost等の信頼できるsecure contextで動作する静的bundleを用意し、実際の検証URL・エンジン・権限と中断回復の証拠を残す。

## Review

2026-10-22、確定したProposedに対する別passのchallenge review。Human intent、Desktop / CLI非変更、現在file保存の対象、保存中追加入力、永続journalの検証前にsourceへ触れないこと、第三の外部変更をrollbackでも保護すること、未知file / 破損journal、権限再要求と再読込の境界を確認。APIによる外部writer排他とクラッシュ後draft復元の限界を明示。Blocking Issues: None。Non-blocking Issues: Chrome以外の実機証拠は対応表で区別。Questions: None。Approved as Proposed: Yes。Autonomous approval eligible: Yes（選択済みObjective内、追加の外部送信 / 権限拡張 / 公開なし、利用者が選択したfolderへの明示Saveは今回依頼に含まれる）。Human gate: None。

Approval mode: Agent-autonomous
Objective / request basis: 2026-10-22のBrowser実装・検証・通常Git deliveryのHuman委任。PR #1の提案を上記の具体化とともに採用。review passの判定後、orchestrationが承認。


## APIの前提

[File System Access仕様](https://wicg.github.io/file-system-access/)、[File System Standard](https://fs.spec.whatwg.org/)、[ChromeのFile System Access解説](https://developer.chrome.com/docs/capabilities/web-apis/file-system-access)を参照。実機結果と対象Candidateは[Development State](../execution-state.md)に記録する。
