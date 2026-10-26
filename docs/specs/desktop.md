# Desktop Authoring

Status: Implemented

Approval mode: Agent-autonomous
Basis: 2026-10-17 Humanの実装・判断委任。別passで製品像、旧資料との境界、途中状態・回復、観察可能なacceptanceをreviewし、Blockingなし。ライブラリ選定・新format・Build対象をAgent Decisionとして採用。Human gate: None within delegated scope。

Change provenance: [表の操作位置とBrowserの識別表示](../spec-changes/0026-table-navigation-and-browser-identity.md)、[レビューで判明した編集対象と保存の保護](../spec-changes/0025-review-data-safety.md)、[Explorer新規作成のEscape取消](../spec-changes/0024-explorer-create-escape.md)、[業務ツールとしてのUI文言の統一と過剰なスローガンの排除](../spec-changes/0020-business-app-ui-copy.md)、[操作と選択のfeedback](../spec-changes/0019-workbench-action-and-selection-feedback.md)、[モダンworkbench](../spec-changes/0018-modern-workbench-surfaces.md)、[Explorerの移動拒否理由](../spec-changes/0014-explorer-drop-reasons.md)、[Rust CLIとネイティブバックエンドコアの採用](../spec-changes/0013-rust-cli-and-backend-core.md)、[表の表示対象・tab退出・標準menu](../spec-changes/0012-table-scope-tab-exit-context-menu.md)、[TOMLプロジェクトファイル](../spec-changes/0011-toml-project-file.md)、[項目編集と直接操作](../spec-changes/0010-object-lists-and-direct-manipulation.md)、[視認できる削除feedback](../spec-changes/0009-visible-deletion-feedback.md)、[編集基盤とExplorer](../spec-changes/0008-editing-foundation-and-explorer.md)、[結果feedbackとBuild操作](../spec-changes/0007-result-feedback-and-build-controls.md)、[compact workbench](../spec-changes/0006-compact-editor-workbench.md)、[保存・Explorer・構造編集feedback](../spec-changes/0005-editor-save-explorer-motion.md)、[値入力の共通枠](../spec-changes/0004-value-input-frame.md)、[開発版の起動入口](../spec-changes/0003-development-app-discovery.md)、[Tauri移行と開発版install](../spec-changes/0002-tauri-development-install.md)、 [編集対象・一時操作・表示の整理](../spec-changes/0001-editor-interaction-polish.md)。

## 根拠と範囲

Humanの製品像、豊かな標準UI・motion、Enum / Customの非modal編集、2026-10-17の完成までの実装委任に基づく。legacy2の型・YAML・キー・参照・Build資料はreferenceであり、自動継承したauthorityではない。以下はその目的から導いたAgent Decisionを含む。

## EDIT-001 — 編集session

開いたProjectのsourceを非同期に一度読み込み、通常操作はメモリ上のdraftへ直ちに反映する。folder開閉・選択・入力でfilesystem discovery、保存、全体の成功判定を実行しない。診断は編集を妨げず更新する。型変更は既存値を保持し、意味が合わない値を勝手に変換・破棄しない。

## EDIT-002 — 履歴・保存

操作単位のProject Undo / Redoを用意し、関連sourceを変更するrename等も一括で戻せる。Saveは履歴を消さず、保存済み状態は履歴と独立して追跡する。新規・削除・rename・移動もdraft操作とし、Save前にdiskへ反映しない。入力途中のローカル値はcommit前のEscで取消し、commit後はUndoする。日本語IMEの確定Enterを操作ショートカットへ流用しない。

「保存」とCmd/Ctrl+Sは現在file、「すべて保存」とCmd/Ctrl+Shift+Sは全変更を対象にする。現在fileの保存で他file・設定・無関係なfolder変更を暗黙保存しない。対象fileの移動・作成に必要な親folderは作る。波及した他fileの編集は未保存で保持し、全保存で確定する。対象fileなしの通常保存は無効。入力のblur確定で新規fileが開かれた場合は、そのfileを今回の保存対象にする。window close / Project切替前の「保存して続ける」は全保存する。保存状態は保存I/O中の追加入力を保持し、Explorer・tab・全体で直ちに一致する。

Saveは意味エラーで拒否しない。保存前に変更対象の外部変更とpath衝突を確認し、黙って上書きしない。複数fileは全対象をpreflightしてbackupを残すtransactionで更新し、失敗時は復旧または復旧情報を保持する。folder削除・移動は未知のfile / directory / symlinkをpreflightで保護し、削除失敗を保存成功にしない。失敗時は今回のfile変更と削除済みfolderを復元するか復旧情報を保持し、baselineを進めない。window close / Project切替は未保存変更を保存・破棄・cancelから選べる。破棄を自動選択しない。

有効sourceをGUIで編集して保存するときは、空でないmap / sequenceを2スペースのblock形式へ統一する。コメント・引用符・整数精度・改行形式・値を保持する。未変更fileやsyntax不正なraw入力を整形のためだけに書き換えない。


file tabのclose iconとCmd/Ctrl+Wは指定 / 現在tabだけを閉じ、最後のtab後もwindowとprojectを保持する。dirtyならfile名と「保存」「保存しない」「キャンセル」を示す。保存・破棄はそのfileだけを対象とし、他file・設定・folder draftへ波及させない。破棄は保存済みsourceへ復元し、新規sourceは取り除き、Undoで回復できる。復元先pathのdraft衝突、保存失敗、保存I/O中の追加入力では入力とtabを保持する。OSのwindow close / 終了は全体の未保存保護を維持する。

## EDIT-003 — 直接編集

表の値、列名・型・modifier、Enum / Flags member、Custom field、Value Objectの定義を同じsessionで編集する。追加はその場に初期値を持つ行・項目を作り、そのまま入力できる。member / field追加ごとのmodalは作らない。配列・nullableの状態を識別でき、入力部品はtop-level / nestedで共有する。

値入力の共通枠が、残り幅を使う値領域とnullable時の固定幅のnull操作領域を管理する。null / 未入力の作成ボタンと各型の入力・詳細入口は値領域を満たし、同じ幅の枠ではnull切替で左右端を変えない。null状態も操作領域を空けて保持し、空の領域にはfocus可能な操作を置かない。boolのswitch自体は適切な大きさを保つ。型別部品や表・ネスト側で個別の幅補正をせず、新しい型も同じ枠を使う。非nullableな枠はnull操作領域を必要としない。

数値の整数は64bitの端まで精度を失わない。boolは共通switch、nullableは型共通の「値なし」と復元操作。Enum、型、タグ、参照候補は入力で検索する。FlagsのNoneとbit選択は相互制御する。新規recordはdefaultを再帰生成し、unique key候補を全draftから割り当てる。構成できなくても既存入力は破棄しない。

## EDIT-004 — navigationと効率

Explorerはsource / folderの作成・rename・移動・削除、検索、種類別icon、認識済みYAMLの拡張子省略を備える。drag moveは可逆なdraftとして確認modalなしで実行する。選択とfocusを区別し、別paneの操作で編集対象を消さない。

横方向にoverflowする表のscrollbarは、recordや検索結果の件数によらず表編集viewport下端（追加操作footerの上）に配置し、resize・Problems開閉・tab切替に追従する。footerや末尾行の操作、headerとbodyの横位置、固定列と仮想化を保持する。列menuの「左へ」「右へ」は左・右の矢印iconを使い、行menuの上下iconと移動・Undoは維持する。

表は固定見出し、縦横scroll、行・列の選択、複製・削除・並替え、コピー＆ペースト、検索、キーボード操作を備える。補助panelの手前を含む実際のscroll領域でdrag auto-scrollする。末尾操作を追加ボタンで覆わない。配列dragは元の項目かpreviewを常に見せ、nested breadcrumbは途中を省かず末端を視認可能にする。

nestedの全祖先と一つ上へ戻る操作を表示し、配列要素から配列一覧へ戻れる。対象pathの遷移では入力の初期表示を行い、別対象の値差を操作アニメーションにしない。問題箇所へのjumpは一度で完結し、その後の検索・選択・scrollを上書きしない。横scroll後の値変更でscrollや列の見た目をリセットしない。

Explorerは重複するproject名表示を除き、約28pxの行で三角・icon・名前を隣接させ、子folderの三角と親folder iconの中心を同じ縦軸に揃える。幅不足の名前はellipsisとfull pathのtooltipで補い、未保存markを保持する。Escapeはrename中なら取消、それ以外はExplorer選択だけを解除してdocumentを保持する。

Explorerのfile context menuはそのfileの操作、folder context menuは作成とそのfolderの操作を表示する。folder本体のsingle clickでも開閉する。rename入力や右クリックでは開閉しない。headerの新規作成入口は維持する。


Explorer fileのsingle clickは最大一つのitalicなpreview tabを開き、次の未open fileでcleanなpreviewだけを置換する。定着済みtabは保持する。Explorer file / tabのdouble clickで定着し、編集されたpreview・元からdirtyなfile・新規fileは自動定着する。遷移前に入力途中の値をcommitする。Explorerは常時到達可能なtoggleと表示menu（Cmd/Ctrl+Shift+E）から開閉し、検索・展開・選択・編集中fileを保持する。非表示のExplorerはkeyboard操作対象にしない。

source iconは表（grid）、分離data（record行）、Enum（単一候補）、Flags（複数選択）、Custom（複数field構造）、Value Object（一つの値の包み）を一式の線画で区別する。Git branch / tag / 組織図を流用せず、file・tab・新規menuで統一し、種類名をtooltip / accessible nameでも示す。

## EDIT-005 — 編集対象と操作結果

行・列・Custom field・Enum / Flags member・配列要素は、値・PK・現在indexから独立したsession内IDで識別する。同じ値の配列要素も区別し、値変更・並替え・Undo / Redoで入力対象を保つ。IDをYAMLへ追加しない。nested editorも祖先の行・配列要素の同一性を追跡し、並替え後も同じ対象を編集する。対象削除時は閉じ、同じindexの別要素へ移らない。編集操作の結果から、作成対象の表示・選択・入力開始とfeedbackを共通して扱う。後続の操作・navigationは古いfocus要求を取り消し、固定frame数や表示indexの推測に依存しない。

Explorerで新規作成するときは検索を解除し、全祖先と作成先folderを展開して新規nodeを表示・選択しrenameへfocusする。menu / rename操作でfolderを開閉しない。作成 / rename入力は展開等の表示更新でfocusを失わず、外側clickによるblurまたはEnterで有効な名前を確定し、Escapeで新規作成自体を取り消し、既存項目では名前編集だけを取り消して元のpathを保つ。新規nodeは名前確定まで仮表示としsession・file tabへ追加せず、取消時は以前のExplorer選択・編集中file・既存draft・dirty・Undo / Redo履歴を保つ。確定時に一つのUndo単位で作成し、fileなら開く。Enter / Escape後のblurで二重確定せず、IME変換中のEnter / Escapeは確定 / 取消に流用しない。確定後は次操作先のfocusを取り戻さない。sourceの新規作成・rename・移動先にはdotで始まるsegmentを認めず、保存側でも拒否する。既存の隠し項目は変更しない。project相対pathは各segmentのWindows drive prefix・backslashも拒否し、絶対path・上位参照・symlink拒否を維持する。無効名は理由を通知し、既存項目は元のpathを保持し、新規作成は仮nodeと入力を残して再入力またはEscapeで回復できる。folder本体へのdropはそのfolder内へ移動し、folder全体をhighlightする。node間のgapは親folderへのdropとして線で示し、表示と移動先を一致させる。自己・子孫・path衝突への移動を実行前に防ぐ。

表headerにはPK、SK n（0始まりsecondary group）、Key n（MessagePack field key）を表示する。複合keyの順序、複数所属、secondaryの一意 / 重複可を確認できる。参照選択は実値を先、その後に弱い色 / 小さい文字の補助名を表示し、両方で検索できる。値の保存と参照先への移動は従来通りとする。

## DATA-001 — 表・型・関係

primitiveはbool / int / uint / long / ulong / float / double / string。Enum / Flags、Value Object、Custom、配列とnullableを扱う。Flagsの予約Noneは名前と値0・存在を固定し、他memberはその後ろで並替える。raw sourceの自由編集と診断は維持する。型循環や未知型、値shape、名前重複、key、参照を診断する。不正なmember / reference / key要素もsourceと要素位置を示して診断し、他の診断を維持する。表のprimary / secondary（unique / nonunique、複合）、referenceをGUIで設定できる。

同梱sampleの非空collectionも保存時と同じ2スペースのblock YAMLにする。

YAML sourceはlegacy2のschema / data / typeの形を採用し、schema内recordsと分離dataを同じ編集操作で扱う。名前を変えるときは関連draftも更新できる。壊れた値はGUIで修復・初期化でき、syntaxが壊れたsourceにはtext編集から修正できる道を残す。

分離data fileのUI名称はTable Data（テーブルデータファイル）。schema内への追加先は「テーブル定義ファイル内」、空の分離file作成は「テーブルデータファイルを作成」とし、既存recordを移動する説明を付けない。

## DATA-002 — Project・出力選択

Projectは固定名`masterdata.toml`に名前・profiles・publish先を保存し、そのfileがあるfolderをrootとして`sources/`以下のYAMLを編集する。内部作業領域は`.masterdata/`。旧YAML設定と旧CLI設定構造へのfallback / 自動importは行わない。nameは空でない文字列を必須とし、省略したprofiles / publishはdefaultを補う。syntaxや既知項目のshape不正はfileと理由を示して読込を中止し、現在のdraftを保持する。未変更設定textを保存の副作用で書き直さない。設定変更の保存では未知項目の値を保持し、アプリ管理のblock TOMLへ整形する（手編集comment / layoutのroundtripは対象外）。設定も外部変更検出と保存transactionで保護し、Publish先として上書きできない。tagsのinclude / excludeによるprofilesをGUIで編集・選択し、今回含まれるrecordを確認できる。Build profile初期値は未選択とし、「プロファイルを選択」を表示する。clearや選択profileの削除で未選択へ戻る。未選択でもBuildでき、タグで絞り込まない既存規則を保持し、結果panelでも未選択 / 絞り込みなしとして示す。

## BUILD-001 — BuildとPublish

明示Buildは選択profileのcurrent draft snapshotを検証する。未保存draftを対象にしたことを表示し、暗黙Saveをしない。不整合があれば該当箇所へ戻れる診断を返し、前回の成功成果物を保持する。Build中も編集可能で、完了時は対象revisionを識別する。

C#定義とMasterMemory binaryは.NET / MasterMemoryで生成し、生成binaryを実際にreloadしてから成功とする。Rust / JSでbinary formatを実装しない。Publishは最新成功成果物を、画面で設定した外部targetへ明示操作で配置する。管理下成果物だけを更新・退役し、隣接するユーザーfile / Unity `.meta`を変更しない。成功・失敗・古い成果物の区別を表示する。toolbarはprofileとBuild split buttonにまとめ、主buttonでBuild、隣のmenuからPublishと実行結果panelの再表示へ進める。表示menuにも実行結果の入口を残し、表示だけでは処理しない。Publishはpanelで対象・配置先を確認してから明示実行する。busy中の重複Build / Publishを防ぐ。成功・失敗・Publish結果をproject内で保持する。公開releaseやUnity起動はこの操作に含めない。

## UI-001 — 共通UI・motion

React / Ant DesignのTable、Tree、Input、Select、Switch、Tabs、Dropdown、Drawer、Modal、Notification等を広く使う。通常操作に共通tokenとmotionを伴わせ、入力をアニメーション完了待ちにしない。追加・削除・並替え・panel切替にも動きを付ける。表の行・列、Enum / Flags member、Custom field、配列、Explorer新規項目には約280msの出現、並替えには約260msの緩やかなease-outを使う。Build / Publish結果にも対象を絞った出現feedbackを付ける。dropdown・folder・panel開閉やdrag previewは約100〜160msの即応を維持する。配列を含む並替えdragはpointer追従と隣接要素のpreviewを示し、drop時に一つのUndo単位で確定する。Esc / pointer取消ではdraftを変えない。補間の完了前も入力・追加・Undo・close・dragができ、連続操作では前のmotionを安全に打ち切り / 接続する。virtual mountを実追加と区別し、新規行 / 列を表示してfeedbackを届ける。record追加は新規行へ移ってすぐ入力でき、検索中の明示追加は検索を解除する。遅延focusで既に移動したユーザーを引き戻さない。単なる入力・保存ack・scroll・folder開閉で既存項目の出現を再生しない。reduced motionでは補間を抑える。全体明滅、診断増減による表位置の移動、更新でのfocus消失を避ける。テーマ、reduced motion、文字とariaによる状態、キーボード操作を備える。

保存状態はiconとtooltip / accessible nameで示し、通常画面で「未保存の変更」の文やfile数・メモリ編集中の説明を重ねない。設定表示は「モーション: ON / OFF」、FlagsのNone表示は「None」に統一する。folderとProblemsの開閉は約100〜120msのOutExpo相当ease-outを使う。


editor設定とproject設定を別の入口・画面にする。editor設定はPC内で保存し、project dirty / Undoから独立させる。themeはLight / Dark / System（初期System）、SystemではOSの変更に追従する。file tabsは一行scroll（初期）または自動折返しを選べる。折返しが多数になればtab帯内を縦scrollして編集領域を保つ。Explorer visibilityとmotionもeditor preferenceで、変更時にsessionをreloadしない。darkの基調は黒に近い無彩色、lightは白・灰色とし、緑accentでactive / focusを示す。error / warningの意味色を保持する。titlebar・操作帯・tabs・modeをcompactにし、大きい見出しとpathの重複を圧縮する。定義名の直接編集、型表示、日常操作と設定への入口を保ち、最小window幅で重ねない。

rendererの共通入口でWebView標準context menuを抑え、未保存draftをReloadで失う入口を作らない。custom file / folder / tab menuへのevent伝播、textのkeyboard shortcutとnative編集menuを維持する。

## DESKTOP-001 — Desktop基盤と開発版

Tauriで既存React / Ant Design UIと編集sessionを提供する。専用の同梱hostを含め、利用者側にNode.jsを要求しない。window・menu・native file / folder dialog、限定されたnative操作、未保存・処理中のclose保護を維持する。二重起動は既存windowを前面にする。「開く」はproject file用pickerから`masterdata.toml`を選び、cancelなら現在projectを保持する。新規作成は保存先folderを選び、新root内に`masterdata.toml`と`sources/`を作る。sampleとinternal startupにも同じproject fileを使う。表示名は設定内nameであり、root名との強制同期はしない。

ローカルcheckoutから`scripts/local-app/app.command` / `scripts/local-app/app.bat`で依存install、検証、package、開発版のinstall、起動まで行える。開発版は`MasterData [Dev]`、identifier `dev.kamahir0.masterdata.dev`、macOS `~/Applications/MasterData [Dev].app` / Windows LocalAppDataの固定先を使い、更新で別コピーを増やさない。通常版と設定・identityを分離する。`--no-launch`で起動を省略できる。checkoutのfetch / pullや実データ変更はしない。

macOSの一括処理は自分の中間.appとsmoke用stagingのLaunch Services登録を解除して片付け、固定install先だけを登録する。正常終了と通常の失敗で中間.appを残さず、強制中断の残存分は次回package前に回収する。後片付け失敗はpathとともに報告する。通常のinstallerは他のcheckoutや旧製品を走査・削除しない。

対象installが起動中なら強制終了・置換せず再実行を案内する。package / staging / install失敗時は既存版を保護・復旧する。名前だけで他の同名アプリを削除しない。

## Acceptance

- Project作成→表・列・Enum・Custom追加（modalなし）→recordとnested値入力→検索→Undo / Redo→Save→再openで復元。
- bool→intの変更後も元値が残り、別操作が可能。Undoで復元。nullable / array / Flagsがtop-levelとnestedで共通挙動。
- source / folder move・deleteをUndoし、Save後のUndoもdirtyに戻る。外部変更と保存衝突でデータを失わない。
- 多数sourceのfolderを繰り返し開閉して追加I/Oなし。表末尾、Problems開時、nested arrayでdrag / scroll操作が成立。
- profileを切替え、referenceを含むsampleをBuild。実.NETでcompile・binary reloadを確認し、Publishでunmanaged fileと`.meta`を保持。
- focused unit / integration、Chromium / WebKitのUI回帰、Tauri実アプリの操作、macOSのローカルbundle、CI。対象外環境の未取得evidenceを成功と扱わない。

## UI-003 — 作業領域と種類の表現

表footerの「レコードを追加」「テーブルデータファイルを作成」は常時見える輪郭と共通の高さを持つbuttonとし、icon・文字で意味を示す。hover / focus / disabledを区別し、追加先と無効理由、作成・入力開始・Undoの規則を保持する。Explorerのiconと名前には選択背景内の左余白を取り、選択 / 非選択で位置を変えず、開閉矢印・階層線も揃える。行高・深い階層の中心軸・ellipsis・dirty・drop表示を保持する。

file tab切替時は選択先tabの緑の上辺に約360msの伸長とfadeを付け、前半から終盤まで変化を視認できる補間にする。追加 / closeのmotionは変更しない。本文は直ちに切り替え、同じtabの編集・dirty更新・再clickでは再生しない。連続切替は最新の選択を表示し、wrap / scroll・preview・close・keyboard・既存の退出motionを保持する。editor / OSのreduced motionでは選択補間を抑える。

Desktop / Browserのworkbenchは、無彩色のwindow背景、補助面のExplorer、主要面のeditorを小さい共通gap・角丸・控えめな輪郭で分離する。titlebarと全体toolbarは背景側、file tabs・file内mode・表内操作は内容側に階層化する。表の行・セルはカード化せず既存密度を保持する。Light / Darkで共通のsemantic tokenをAnt Designと共有し、緑accent、error / warning、種類色の意味を保つ。狭幅では外周のgapを縮め、主要操作を重ねず到達可能にする。

選択file tabは上側の角丸と緑の上辺、文字色で識別し、下に線を引かずeditorと同じ面へつなげる。非選択hoverとkeyboard focusを区別し、focus ringは内側に見せる。preview italic・dirty dot・close・scroll / wrap・既存motionを維持する。wrapは帯の高さを制限し選択を同色と目印で識別する。Problemsはeditorの下にgapで分離した補助面として配置し、内容を覆わず開閉・jump・表のviewport再計測を保つ。nested / 設定 / 結果Drawerは共通の面・角丸を使い、mask・閉じる操作・入力モデルを維持する。resizeの溝には控えめなhandleを示し、hover / focus / dragを緑で強調する。Explorer非表示時は空の溝を残さない。これらの表示変更でsession・draft・選択・focus・scroll・Undo・未保存状態をresetしない。

Explorer境界をdragで幅変更でき、矢印 / Home / End、double clickでdefault復帰もできる。PC内設定に保存し、折畳みとproject dirtyから独立する。default範囲は180〜480pxで、狭いwindowの編集領域も確保する。

種類iconは形を維持し、Table=blue、Table Data=cyan、Enum=amber、Flags=orange、Custom=violet、Value Object=roseの種類色を両themeで用いる。green accentは操作状態に使う。headerは種類badge、icon、名前の順とし、TABLE / TABLE DATA / ENUM / FLAGS / CUSTOM TYPE / VALUE OBJECTを明示する。種類名と形も保持し、色だけに頼らない。

削除は即時にdraft / Undoへ反映し、可視対象に約300msの非対話な退出snapshotを示す。退出領域の縮小と、残った項目・移動する末尾操作が空間を詰める動きを同じ補間で結び、対象の退出を視認できるようにする。列削除は横方向へ同様に詰める。画面外のsnapshotを作らず、ghostは入力・selection・accessibility対象にせず完了 / 取消時に回収する。tabの追加 / closeは当該tabの幅とopacityで約200〜280msの出入りを示し、既存tab全体を上下移動させない。右端・最後の一つでも対象の姿を残して退出を見せ、後続tabの移動だけに依存しない。退出tabは即時に非対話・非accessibleとし、連続操作とscroll / wrapを維持する。active tabへのscrollはtab帯内に限定する。reduced motionでは補間を抑える。

app iconはgreen accent系のvector原稿から生成し、macOSは16〜1024pxのretina slot、Windowsは16〜256pxを含める。小サイズの線も読みやすくし、画面内brandとnative bundleで意匠を揃える。

UI文言は業務ツールとして平易・簡潔・正確な機能説明を用い、ポエミーなスローガンや過剰なキャッチコピー（「世界をつくる」「YOUR DATA, YOUR WORLD」「思いついたその場から」「つながりまで、見える。」等）を配置しない。未選択時はツール名（MasterData）のみを表示し、空状態・プレビュー・ヘルプは機能と対象（表・型・外部キー参照・整合性検証等）に即した表現を用いる。

## UI-004 — 対象の境界と直接操作

MasterRef・出力profile・副キーは共通の項目枠で編集する。headerに名前と右端の項目全体の削除操作、bodyに内部設定を置き、境界・余白・背景で一項目の範囲を示す。安定identityと追加 / 削除feedback、Undoを用い、他項目の入力を取り違えない。

検索できる単一選択（Enum / MasterRef / 型 / profile）は、開いた空の入力部分の再click、Esc、選択で閉じ、入力で絞り込める。検索文字の編集や複数選択のtag操作を妨げない。

file tabとExplorerのfileは共通の名前変更・削除・パスをコピーを持つ。folderだけ作成を加える。renameはExplorerを表示し祖先を開いてinline入力へ移る。copyは現在のdraft pathに対応する絶対path（project root / sources以下）とする。保存・破棄を暗黙に行わず、変更は既存の可逆draft操作にする。

Explorerのdrag先判定と表示を一致させる。folder中央はそのfolderをhighlightし、file / node間は親folderへの明瞭な挿入線、空白・見出しはsources rootへのExplorer全体のhighlightを示す。祖先への移動を許可し、自己・子孫・衝突は拒否する。拒否時はdrag中にExplorer内で理由を文字表示し、同名衝突なら名前・移動先と名前変更 / 別folder選択による対処を示す。判定は検索対象だけでなく全draftに行う。拒否のdrop eventが届いた場合も同じ理由を通知する。上書き・mergeは行わず、draft / Undo / diskを変更しない。feedbackはpointer / focusや行位置を変えず、許可先への切替・領域外・drag終了 / 取消で回収する。同じ場所は変更なし。hover展開・取消・Undoを保持する。展開folderの下端から子孫へ細い階層線を描き、末端はnode側へ直角に折れてicon手前で止まる。triangle hover背景はtriangle付近に収め、resize境界はsticky headerより前に出す。

仮想表では削除によるscroll終端の自動補正でも可視行 / 列の退出と後続配置を連動させ、sticky列 / headerをsnapshotで覆わず、画面外の全件をmountしない。行操作・配布タグは左の固定領域に置く。固定領域のcontrol以外のclickで行全体を選択し、Ctrl / Cmdで追加解除、Shiftで範囲選択できる。indexは選択可能textにしない。入力・タグ・menu・drag・コピー・Deleteを保持する。

レコード追加先は現在の表示対象fileから一意に決め、「追加先ファイル」UIは置かない。複数sourceの「すべてのデータ」では追加buttonと複製をdisabledにし、表示選択で一つのfileを選ぶ案内をtooltip / accessible descriptionに示す。既存行数を超えるpasteも一括拒否し、理由とfile選択による回復を示す。既存recordの編集・範囲内paste・削除・移動は維持する。単一source表では全表示でも追加できる。検索結果のfile数で規則を変えない。単一file表示では追加・複製・paste拡張をそのfileだけに行う。空のテーブルデータfile作成後はそのfileを開いて追加・保存でき、選択fileがUndo等で消えたら全表示へ戻る。

nestedの前進 / 後退は方向の分かる短いslideとfadeで表す。退出paneは非対話・非accessible、現在paneは完了を待たずに操作できる。途中入力の確定 / Esc、全breadcrumb、reduced motionを保持する。theme選択は文字に太陽・月・PC iconを添える。
