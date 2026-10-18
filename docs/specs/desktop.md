# Desktop Authoring

Status: Approved

Approval mode: Agent-autonomous
Basis: 2026-10-17 Humanの実装・判断委任。別passで製品像、旧資料との境界、途中状態・回復、観察可能なacceptanceをreviewし、Blockingなし。ライブラリ選定・新format・Build対象をAgent Decisionとして採用。Human gate: None within delegated scope。

Change provenance: [Tauri移行と開発版install](../spec-changes/0002-tauri-development-install.md)、 [編集対象・一時操作・表示の整理](../spec-changes/0001-editor-interaction-polish.md)。

## 根拠と範囲

Humanの製品像、豊かな標準UI・motion、Enum / Customの非modal編集、2026-10-17の完成までの実装委任に基づく。legacy2の型・YAML・キー・参照・Build資料はreferenceであり、自動継承したauthorityではない。以下はその目的から導いたAgent Decisionを含む。

## EDIT-001 — 編集session

開いたProjectのsourceを非同期に一度読み込み、通常操作はメモリ上のdraftへ直ちに反映する。folder開閉・選択・入力でfilesystem discovery、保存、全体の成功判定を実行しない。診断は編集を妨げず更新する。型変更は既存値を保持し、意味が合わない値を勝手に変換・破棄しない。

## EDIT-002 — 履歴・保存

操作単位のProject Undo / Redoを用意し、関連sourceを変更するrename等も一括で戻せる。Saveは履歴を消さず、保存済み状態は履歴と独立して追跡する。新規・削除・rename・移動もdraft操作とし、Save前にdiskへ反映しない。入力途中のローカル値はcommit前のEscで取消し、commit後はUndoする。日本語IMEの確定Enterを操作ショートカットへ流用しない。

Saveは意味エラーで拒否しない。保存前に変更対象の外部変更とpath衝突を確認し、黙って上書きしない。複数fileは全対象をpreflightしてbackupを残すtransactionで更新し、失敗時は復旧または復旧情報を保持する。close / Project切替は未保存変更を保存・破棄・cancelから選べる。破棄を自動選択しない。

有効sourceをGUIで編集して保存するときは、空でないmap / sequenceを2スペースのblock形式へ統一する。コメント・引用符・整数精度・改行形式・値を保持する。未変更fileやsyntax不正なraw入力を整形のためだけに書き換えない。

## EDIT-003 — 直接編集

表の値、列名・型・modifier、Enum / Flags member、Custom field、Value Objectの定義を同じsessionで編集する。追加はその場に初期値を持つ行・項目を作り、そのまま入力できる。member / field追加ごとのmodalは作らない。配列・nullableの状態を識別でき、入力部品はtop-level / nestedで共有する。

数値の整数は64bitの端まで精度を失わない。boolは共通switch、nullableは型共通の「値なし」と復元操作。Enum、型、タグ、参照候補は入力で検索する。FlagsのNoneとbit選択は相互制御する。新規recordはdefaultを再帰生成し、unique key候補を全draftから割り当てる。構成できなくても既存入力は破棄しない。

## EDIT-004 — navigationと効率

Explorerはsource / folderの作成・rename・移動・削除、検索、種類別icon、認識済みYAMLの拡張子省略を備える。drag moveは可逆なdraftとして確認modalなしで実行する。選択とfocusを区別し、別paneの操作で編集対象を消さない。

表は固定見出し、縦横scroll、行・列の選択、複製・削除・並替え、コピー＆ペースト、検索、キーボード操作を備える。補助panelの手前を含む実際のscroll領域でdrag auto-scrollする。末尾操作を追加ボタンで覆わない。配列dragは元の項目かpreviewを常に見せ、nested breadcrumbは途中を省かず末端を視認可能にする。

nestedの全祖先と一つ上へ戻る操作を表示し、配列要素から配列一覧へ戻れる。対象pathの遷移では入力の初期表示を行い、別対象の値差を操作アニメーションにしない。問題箇所へのjumpは一度で完結し、その後の検索・選択・scrollを上書きしない。横scroll後の値変更でscrollや列の見た目をリセットしない。

Explorerのfile context menuはそのfileの操作、folder context menuは作成とそのfolderの操作を表示する。folder本体のsingle clickでも開閉する。rename入力や右クリックでは開閉しない。headerの新規作成入口は維持する。

## DATA-001 — 表・型・関係

primitiveはbool / int / uint / long / ulong / float / double / string。Enum / Flags、Value Object、Custom、配列とnullableを扱う。型循環や未知型、値shape、名前重複、key、参照を診断する。表のprimary / secondary（unique / nonunique、複合）、referenceをGUIで設定できる。

YAML sourceはlegacy2のschema / data / typeの形を採用し、schema内recordsと分離dataを同じ編集操作で扱う。名前を変えるときは関連draftも更新できる。壊れた値はGUIで修復・初期化でき、syntaxが壊れたsourceにはtext編集から修正できる道を残す。

## DATA-002 — Project・出力選択

Projectは`masterdata.yaml`に名前・profiles・publish先を保存し、`sources/`以下のYAMLを編集する。既存legacy2の`masterdata.toml`も読込み時に解釈できるが、そのCLI lifecycleを採用しない。tagsのinclude / excludeによるprofilesをGUIで編集・選択し、今回含まれるrecordを確認できる。

## BUILD-001 — BuildとPublish

明示Buildは選択profileのcurrent draft snapshotを検証する。未保存draftを対象にしたことを表示し、暗黙Saveをしない。不整合があれば該当箇所へ戻れる診断を返し、前回の成功成果物を保持する。Build中も編集可能で、完了時は対象revisionを識別する。

C#定義とMasterMemory binaryは.NET / MasterMemoryで生成し、生成binaryを実際にreloadしてから成功とする。Rust / JSでbinary formatを実装しない。Publishは最新成功成果物を、画面で設定した外部targetへ明示操作で配置する。管理下成果物だけを更新・退役し、隣接するユーザーfile / Unity `.meta`を変更しない。成功・失敗・古い成果物の区別を表示する。公開releaseやUnity起動はこの操作に含めない。

## UI-001 — 共通UI・motion

React / Ant DesignのTable、Tree、Input、Select、Switch、Tabs、Dropdown、Drawer、Modal、Notification等を広く使う。通常操作に共通tokenとmotionを伴わせ、入力をアニメーション完了待ちにしない。追加・削除・並替え・panel切替にも動きを付ける。全体明滅、診断増減による表位置の移動、更新でのfocus消失を避ける。テーマ、reduced motion、文字とariaによる状態、キーボード操作を備える。

保存状態はiconとtooltip / accessible nameで示し、通常画面で「未保存の変更」の文やfile数・メモリ編集中の説明を重ねない。設定表示は「モーション: ON / OFF」、FlagsのNone表示は「None」に統一する。folderとProblemsの開閉は約100〜120msのOutExpo相当ease-outを使う。

## DESKTOP-001 — Desktop基盤と開発版

Tauriで既存React / Ant Design UIと編集sessionを提供する。専用の同梱hostを含め、利用者側にNode.jsを要求しない。window・menu・folder dialog、限定されたnative操作、未保存・処理中のclose保護を維持する。二重起動は既存windowを前面にする。

ローカルcheckoutから`app.command` / `app.bat`で依存install、検証、package、開発版のinstall、起動まで行える。開発版は`MasterData [Dev]`、identifier `dev.kamahir0.masterdata.dev`、macOS `~/Applications/MasterData [Dev].app` / Windows LocalAppDataの固定先を使い、更新で別コピーを増やさない。通常版と設定・identityを分離する。`--no-launch`で起動を省略できる。checkoutのfetch / pullや実データ変更はしない。

対象installが起動中なら強制終了・置換せず再実行を案内する。package / staging / install失敗時は既存版を保護・復旧する。名前だけで他の同名アプリを削除しない。

## Acceptance

- Project作成→表・列・Enum・Custom追加（modalなし）→recordとnested値入力→検索→Undo / Redo→Save→再openで復元。
- bool→intの変更後も元値が残り、別操作が可能。Undoで復元。nullable / array / Flagsがtop-levelとnestedで共通挙動。
- source / folder move・deleteをUndoし、Save後のUndoもdirtyに戻る。外部変更と保存衝突でデータを失わない。
- 多数sourceのfolderを繰り返し開閉して追加I/Oなし。表末尾、Problems開時、nested arrayでdrag / scroll操作が成立。
- profileを切替え、referenceを含むsampleをBuild。実.NETでcompile・binary reloadを確認し、Publishでunmanaged fileと`.meta`を保持。
- focused unit / integration、Chromium / WebKitのUI回帰、Tauri実アプリの操作、macOSのローカルbundle、CI。対象外環境の未取得evidenceを成功と扱わない。
