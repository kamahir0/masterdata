# RFC: 日常の制作を完結するMasterdata authoring system v1

Status: Accepted

## この文書の読み方

根源的な要望、既に決まった制約、その延長上の具体的な仕様案を一緒に読める設計提案である。
2026-09-16のHumanによる「小刻みな進め方から、まとまった実装へ移るため、創造性を発揮して理想のシステムを文書化する」という依頼に基づく。

**提案する到達点は、Desktopで「定義する → 探す → まとめて編集する → 確認する → Unityへ渡す」が完結するv1である。**
計算列、Reference、Webまで方向を示すが、すべてを最初の出荷条件にしない。

2026-09-16、HumanはOption Bと直前の実行summaryに示した推薦方向を選択した。詳細契約は[0016](../spec-changes/0016-desktop-daily-editing.md)・[0017](../spec-changes/0017-desktop-workspace-settings.md)・[0018](../spec-changes/0018-desktop-build-delivery.md)の未承認proposalへ具体化した。

「既決」はリンク先のApproved / Implemented contractの要約、「提案」は設計時の候補を表す。
提案節のdefault、操作、失敗時の挙動、受け入れscenarioも未承認であり、現行仕様や実装済み機能と混同しない。
本RFCは実装authorityではない。採用時は責務ごとの仕様変更へ移し、canonical ownerに適用する。
画面名は説明用であり、新しいCLI grammar、config key、file formatを暗黙に決定しない。

## 根拠と分類（Source Evidence and Classification）

| Class | Evidence / 解釈 |
| --- | --- |
| Requirement | 2026-09-16の依頼: 原要望と現状を把握し、使いやすい理想仕様をまとまった範囲まで創造的に文書化する |
| Preference | 同依頼: 今後は小刻みな進行から一気に実装を進める方向 |
| Constraint | 上記Approved authority: YAML / Git、shared Rust、.NET、identity分離、source preservation、各operationの安全性 |
| Decision | 2026-09-16、Humanは直前summaryへの「進める」によりOption B / P1–P3、file単位編集、保存前Undo、scalar一括入力、計算列の後段化を選択 |
| Proposal | codec、operator、設定保存等の具体的contractは0016–0018でreview・Approval対象。後段expressionのexact semanticsは未承認 |
| Open Question | 性能条件、提案defaultの採否、後段exact contract。未提供の過去会話は根拠に追加しない |

## 背景（Context）

### 根源的な要望

[Product vision](../product/vision.md)と既存契約から、解決したい問題を次のように整理する。

| 要望の核 | 利用者が得たい結果 | 設計への含意 |
| --- | --- | --- |
| マスターデータを普通の開発資産として扱う | Git diffで意図を読み、review・自動化・再生成できる | YAMLを正本にし、編集対象外のsource textを保つ |
| 形式の細部と格闘せずデータを作りたい | 型を選び、表やフォームで入力できる | schema-aware GUIを通常経路にし、complex valueも同じ体系で扱う |
| 数件の入力から大量の調整まで進めたい | 検索、比較、まとめた修正、検証が途切れない | single-cell editorの次に作業単位としての表操作を整える |
| 安全にゲームへ渡したい | 何をBuildし、どこへPublishしたか説明できる | buffer、保存済みsnapshot、artifact、配置先を区別する |
| 人とAIで別のルールを持ちたくない | 同じsourceとdiagnosticから変更を作り、確認できる | GUI専用・AI専用のdomain ruleを増やさない |
| 環境に縛られたくない | Unityを開かず作業し、将来はWebでも扱える | local-first、shared Rust semantics、host capabilityを維持する |

「大量の調整まで進めたい」は既存Data Editorのspreadsheet方向と今回の依頼からの解釈を含む。
過去の全会話は今回の入力に含まれない。記録にない利用人数、件数、運用SLA、発売済みゲームとの互換性要求は推定しない。

### 現時点で決まっていること

この表は導線であり、ruleの第二のownerではない。Approvedは実装完了を意味しない。

| 領域 | 既決の中心 | Authority |
| --- | --- | --- |
| 正本 | YAMLが正本。generated C# / binaryは派生成果物 | [ADR 0001](../adr/0001-yaml-is-source-of-truth.md) |
| 共通処理 | CLI / GUIはshared application/coreを使用。GUIからCLI subprocessでdomain処理しない | [ADR 0002](../adr/0002-rust-core-shared-by-cli-and-gui.md) |
| MasterMemory | Rustが意味を解決し、.NETへcompile / binary buildを委譲 | [ADR 0003](../adr/0003-dotnet-mastermemory-bridge.md) |
| identity | `table`はproject-local identity。path・C#名は別。MessagePack `key`はserialization metadata | [Table identity](../specs/compatibility/table-identity.md)、[Table / Keys](../specs/table-and-keys.md)、[ADR 0005](../adr/0005-messagepack-key-as-serialization-only.md) |
| 型 | Primitive、Value Object、Enum、Flags、Custom Type、Required / Nullable / Array | [Type System](../specs/type-system/README.md) |
| 入力 | complex valueもshared resolved modelで編集。64-bit整数をlosslessに保持 | [Source Record Edit](../specs/source-edit.md)、[Data Editor](../gui/data-editor/spec.md) |
| 保存 | file単位dirty / Save、source preservation、lost-update prevention。domain-invalidでも安全なら保存可能 | [Source Record Edit](../specs/source-edit.md) |
| 追加・削除 | Added draftと既存recordを区別。初回保存前のkey入力は既存key変更の許可ではない | [Record Mutation](../specs/source-record-mutation.md)、[GUI](../gui/data-editor/record-mutation.md) |
| 定義変更 | Table field / type変更はPlan / Diff、依存解決、stale検査、必要なdestructive authorizationを経る | [Schema Migration](../specs/schema-migration.md)、[Type Migration](../specs/type-migration.md) |
| 選別 | TagとProfileからselected datasetを構成してdataset constraintを検証 | [Build Selection](../specs/build-selection.md) |
| 成果物 | C#・binary・receiptをcoherent setとしてBuild。Publishは別操作、複数targetとownershipを扱う | [Build pipeline](../specs/build-pipeline.md) |
| Web | Standaloneはauthoring / validation、Connectedは認可されたNative capabilityを利用 | [Runtime hosts](../specs/runtime-hosts.md)、[ADR 0006](../adr/0006-host-capability-composition.md) |

### 現在の足場と残る距離

調査基点は`6a1ef28b5d0d5a25eddf5ad101046ad9ccf0dcb6`。直前ObjectiveのComplex Value Authoring v1はcandidate
`fee882c2ef3c3516ded184707c6cc2d96cd4d252`のverificationを経て完了している。これは調査時点の記録であり、進捗台帳ではない。

- [shared authoring service](../../crates/masterdata-app/src/authoring.rs)、[ValueEditor](../../apps/gui/src/ValueEditor.tsx)、[workflow tests](../../apps/gui/tests/authoring.test.tsx)にはcomplex入力、draft、dirty Build、保存結果を扱う実装とtestがある。
- [Table Editor tests](../../apps/gui/tests/table-editor.test.tsx)と[Type Editor tests](../../apps/gui/tests/type-editor.test.tsx)にはPlan失効、dirty fileとの合成、destructive確認のevidenceがある。
- [application service](../../crates/masterdata-app/src/lib.rs)と[publish service](../../crates/masterdata-app/src/publish.rs)にはBuild / Publish基盤がある。GUIの運用導線を揃えることとengineの新規実装は区別する。
- Data Editorは選択file単位。range paste、general Undo/Redo、既存key編集は現行contractの対象外。
- [Reference](../specs/index-and-reference.md)はDraft。pipelineにReferenceの位置があることを、具体的機能の承認・完成とは扱わない。
- Programmable Viewは[Data Editorの将来方向](../gui/data-editor/spec.md)にあるが、language・共有format・securityは未決定。WebもApproved architectureと完成した利用体験を区別する。

## 課題（Problem）

個々のeditorが増えても、途中でraw source、外部表計算、terminalへ何度も戻るなら日常の道具としては未完成になる。
一方、便利さのために一括保存、キー変更、Publishを暗黙実行すると既存の安全設計を失う。
次は機能数でなく、代表的な制作仕事を最後まで完遂できる単位で実装をまとめたい。

## 目標（Goals）

提案するv1の成功像は次のとおり。

1. ProjectにTable / Type / Dataを作り、complex valueを含むrecordを入力できる。
2. 多数のrecordから対象を絞り、表でまとめて変更し、保存前なら操作を戻せる。
3. 分割されたTableを見渡し、選択recordのsourceへ迷わず戻れる。
4. Profileで変わる収録内容と問題を把握し、保存済みsourceからBuildしてUnityへPublishできる。
5. 外部編集、古い応答、部分失敗、Recovery Requiredが起きても、何が保存されたかを誤認しない。

## 非目標（Non-Goals）

v1の出荷条件には任意JavaScript実行、real-time共同編集、cloud database、AI自動修正・自動Publish、
Excel workbookとの完全往復、general SQL、任意型変換、released-version binary compatibility、
file/folder一括移動・削除、Web Native Host installer完成を含めない。後段の候補は後述する。

## 選択肢（Options）

| 選択肢 | 到達点 | 利点 | 代償 |
| --- | --- | --- | --- |
| A: 従来どおり単機能を追加 | key編集、file操作等を個別に完了 | 一回の変更が小さい | 制作全体の断点が残り、今回求める加速に結び付きにくい |
| B: Desktop制作v1をまとめる（推薦） | 表編集からBuild / Publishまでの日常workflow | 既存資産を使って価値のある完成形へ届く | 複数surfaceの整合をまとめて検証する必要がある |
| C: Web・Reference・programmable runtimeも同時完成 | 長期像まで一度に提供 | 最大の機能範囲 | protocol、security、言語、互換性が相互依存し、完成判定が遠い |

## トレードオフ（Trade-offs）

Bは大胆なまとまりと、確かに完成する境界を両立する案である。
全Table表示は初回read-only、bulk mutationは一つのfile内、Undoは未保存buffer内から始める。
UIの一貫性に一部制約を置く代わりに、cross-file transaction、永続record identity、Git履歴操作を同時発明せず、
日常作業を改善する。複数fileの保存は既存Save Allを使い、その非atomic性を隠さない。

## 提案（Proposal）

以下はすべて提案behaviorであり、承認後にowner specificationへ移す候補である。

### 1. 制作の入口と画面構成

Desktopを最初の完成hostとする。Open ProjectとCreate Projectを入口にし、最近のProjectはuser-localな便利機能とする。
Create Projectは既存initialization contractをGUIへ接続し、サンプルデータや隣接Unity directoryを勝手に作らない。
作成後は「型を作る」「Tableを作る」「Data fileを作る」「recordを追加」の次操作を示す。
Table schema作成とData file作成は別の明示操作として維持する。

Explorer、中央editor、Problemsを維持し、file/folder導線にlogical Table / Typeへの直接導線を加える。
同じsourceに複数の入口から到達しても同じdirty bufferを使用する。
中央はData / Table / Type / Diffに加え、Table OverviewとBuild / Publishを提案する。
選択file・logical Table・dirty状態を識別でき、disabled commandにはpermission、競合、未対応、Recovery Required等の理由を示す。
keyboardで主要操作とProblemsからの移動を完結でき、色だけでdirty・error・収録対象を区別しない。

### 2. 探す: File ViewとTable Overview

File Viewは既存Data Editorを継承し、初期表示はsource record orderとschema field orderとする。
検索、column filter、表示sortはview operationであり、YAMLの順序を書き換えない。
検索対象はtyped scalarの表示値から始める。文字列はcase-sensitiveな部分一致を推薦し、complex全体検索は後段とする。
数値比較はshared typed valueを使い、64-bit値をJS numberへ変換しない。exact operator一覧は後続proposalで閉じる。

Table Overviewはlogical Tableの全Data documentを横断するread-only viewとする。
各rowにsource fileとoccurrenceへの導線を持ち、同じPKのrowも個別表示する。
開く操作はFile Viewの正しいoccurrenceを選択する。source変更で再特定できなければ、同じPKの別recordへ推測で移動しない。

Overviewは保存済みsourceのsnapshotを使う案を推薦する。dirty fileがあれば件数と「未保存変更は未反映」を示す。
全bufferを重ねたproject-wide viewはv1から外し、File Viewのcurrent-buffer validationと区別する。
Profile選択はOverview内の明示操作であり、通常File Viewの行を勝手に隠さない。

### 3. まとめて編集する: Range操作

一つのFile View内の矩形range選択、copy、paste、同じ値のfillをv1へ提案する。
最初はRequired / Nullableのscalar leafとして編集できる列を対象とし、complex構造pasteは対象外とする。
既存key列、computed列、read-only cellを含むmutationは、対象を黙ってskipせず全体を適用前に停止する。

pasteはactive cellを左上としてclipboardの矩形を当てる。選択rangeとのshape差を暗黙repeatで埋めず、rowも自動追加しない。
filter中はvisible rowの順に対応付け、hidden rowは変更しない。preview後にsort / filter / snapshotが変わればpreviewを失効させる。
画面row番号を永続targetにせず、shared source provenanceへ解決する。

複数cellへの変更は、適用前に件数・file・列・before / after・入力上の問題をpreviewする。
適用はlocal bufferへの一操作でありdisk Saveではない。domain-invalidな入力は問題表示付きで保持できる。
losslessな表現不能、貼付形状不正、read-only対象等のoperation failureは部分適用しない。
数字らしいstringを勝手に数値化せず、空文字とnullを区別する。null化は専用の明示操作を推薦する。

外部表計算とのclipboardはTSVを入口とする案。quote・改行・tab・empty cellのexact codecはfixture付きで先に仕様化する。
入力の切り捨てや、complex値をJSONらしさで推測するfallbackは採用しない。
連番生成、数式のsource書戻し、multi-file paste、bulk delete / duplicateは後段とする。

### 4. 戻せる作業: 未保存Undo/Redo

履歴はfileごとの未保存編集を単位とする。scalar入力確定、complex操作、range適用をそれぞれ一操作として戻せる案を採る。
別fileへ移動しても履歴を保持し、active fileだけがUndo対象になる。Undo後の新規編集はそのfileのRedoを破棄する。
base snapshotと同じ内容へ戻れば既存仕様どおりcleanになる。

v1はSave成功でそのfileの履歴をclearする。Save失敗ではbufferと履歴を保持する。
保存済みdisk、Migration、Publish、Git commitをUndoする意味は持たせない。
Conflict中もlocal bufferの操作を戻せるが、external sourceの更新は既存Compare / Reload / Overwrite契約に従う。
Reloadにはbufferと履歴の破棄が含まれると示す。終了を越えたdraft復旧は別仕様とする。

### 5. 型とTableを育てる

既存Table / Type EditorのPlan / Diffを継承し、initializerにData Editorと同じschema-aware value editorを接続する。
通常経路ではcomplex initializerのJSON / YAML手入力を避ける。値の意味、validation、依存解決はfrontendへ移さない。

Primary / Secondary Key定義編集はv1本体の次に着手できる独立packageとする。
通常cell editへ混ぜず、ordered field sequence、変更後query APIへの影響、dataset diagnosticsをPlanへ示す案を推薦する。
Secondary Keyのpersistent ID / nameは導入しない。field順序変更とserialization key変更を混同しない。
新Migration operationとpostconditionは現行Migration v1への別承認deltaとなる。

既存recordのkey値変更はさらに別operationとして、occurrenceを明示したPlan / Diffを推薦する。
同値の他recordの書換えやReference cascadeを推測しない。型変更、自動変換、Table rename、released binary互換は含めない。

### 6. ProfileとTagで収録内容を把握する

Tagはdomain fieldと別のcontrolに表示する。既知tagを候補に出すがregistry登録は要求しない。
初版Tag編集はfile単位bufferへ入り、通常値編集と同じSave / Diff / Conflict lifecycleを使う提案とする。
現行source-editの`$tags`非scopeを拡張するため、新しい承認対象である。

Profile一覧・作成・include/exclude編集はProject settingsへ置く。
config保存はexact baseとcandidate差分を確認し、外部変更時に停止する。
YAML source-editをTOMLへ適用済みとみなさず、comment、対象section外bytes、unknown section保持を新契約で定める。
Profileは追加と既存selection編集へ絞り、rename / deleteはv1必須にしない。
Publish target設定は新規Projectからの出口に必要なため、既存の`csharp` / `binary` targetの追加・path編集をv1へ含める。
target設定の保存は配布を開始しない。設定から外す操作を用意する場合も、旧destinationのfile削除やownership放棄処理を暗黙実行しない。
TOML内のtarget arrayとProfile sectionの双方について、無関係な設定とcommentを保持するcontractを用意する。

selection previewは保存済みsource / configからshared engineで算出し、Tableごとの全件数・selected件数と、rowの選択／除外理由を示す。
GUIがinclude/exclude式を再実装しない。profile未選択はunfilteredと表示し、消えたnamed profileから黙ってfallbackしない。
production用・debug用で同じPKのrecordも別occurrenceとして表示し、それぞれのselectionで検証する。

### 7. Problemsを直すための場所にする

一覧に対象snapshotを表示し、current buffer、保存済みProject、選択Profileの検証を区別する。
field / record / fileへのnavigationとnested control focusは既存structured diagnosticから構成する。
非表示cellへ移動する場合は、対象を見せるため一時的にfilterを解除したことを示す案とする。
古いsnapshotの応答で新しい問題や値を上書きしない。

validationは入力とSaveの許可判定にしない。一方Buildの検証失敗はBuild失敗として扱う。
値の不正、保存不能、Build不能、Publish部分失敗を同じerror表示だけで混同しない。
v1で自動修正は行わず、候補がある場合も通常編集かreview可能なPlanに導く。

### 8. Build / Publishを制作の出口にする

専用surfaceに保存済み入力を使うこと、明示Profile、native capability / toolchainの利用可否を表示する。
Build、Publish last successful artifact、Build and Publishを区別して提示する案とする。
複合操作はBuild成功後だけ既存Publishへ進む。単独BuildやSaveへPublishを混ぜない。
dirty中もBuild可能とし、未保存変更を含まない表示とSave Allへの別導線を提供する。

Build結果に成功／失敗、diagnostics、canonical artifactの場所とreceipt検証結果を表示する。
Publishはcurrent YAMLの再Buildではなく検証済みartifact setの配布と表示する。
receipt validはsourceが最新である証拠ではない。freshness未取得なら「未確認」とし、最新表示を推測しない。
Profileごとのartifact archiveは新設せず、既存last successful canonical setを使う。

Publish前に全configured targetのpreflightとpathを確認できる。現行all-target contractに従い、
未承認のtarget subset publishや失敗targetだけのretryを追加しない。
target別結果を残し、unmanaged collisionを自動overwriteせず、Unity `.meta`を所有物と推測しない。
Publish成功はUnity側compileやゲーム内動作確認の成功を意味しない。

### 9. 人とAIが同じ仕事を扱う

組込みAI chatはv1必須にしない。YAML、仕様、structured diagnostics、Plan / Diffを共通の接点とする。
CLIの既存操作はshared engineを使用し、GUI機能ごとに新CLI grammarを自動追加しない。
後段のmachine-readable bulk / migration interfaceでもsnapshot、Plan、authorization、resultを分離する。
AI-generated変更も通常の差分確認と安全性検査を通す方向とする。
Gitは明示workflowのままとし、productのSave / Build / Publishに自動commitを付けない。

## v1以降の理想像

### 計算列と作業用View

Source、Computed / View、Annotation columnを分ける既存directionを継承する。
最初はread-onlyの決定的expressionを推薦する。任意JavaScript、network、filesystem、process、時刻、randomへアクセスしない。
算術、比較、条件分岐、同じTableのgroup aggregateから始め、runtime dataへ値を書き戻さない。

代表例はdrop tableの`groupId`別`weight`合計から各rowの確率を表示するView。
filterで母集団が勝手に変わらないよう、計算対象を保存済みTable全体か明示Profileのdatasetとして表示する案とする。
分母0、欠損、型不一致、循環、評価budget超過は計算問題として示し、0に丸めずsource validationとも分離する。
整数のexact性と除算表現はlanguage仕様で決め、表示の丸めをsourceに反映しない。

同じexpressionから色・強調を導出できる方向とし、装飾だけで意味を伝えない。
共有ViewはGit管理のauthoring metadata、列幅等はuser-local stateとする案を推薦する。
format / version / discoveryは別仕様とし、既存schemaへ未知keyを勝手に追加しない。
schema renameで壊れたexpressionを黙って文字列置換せず、問題として示すか承認されたmigrationで更新する。
Annotationはsource occurrenceを越える永続的な紐付けが未解決なため、そのidentity設計まで延期する。

### Referenceとキー変更

Referenceは明示宣言された関係だけを扱い、`itemId`という名前や同じValue Object typeから推論しない方向を推薦する。
最初はunique targetへのscalar参照と候補選択・navigationから始める案を比較対象にする。
composite key、non-unique target、Nullable、配列参照、generated helperはDraft ownerで整合を閉じるまで実装しない。
profileで対象外になった参照先を、source全体にあるからvalidとはしない。
Reference導入後のkey変更は依存Planを更新し、明示scopeだけを変える。自動cascadeは別承認とする。

### Sourceの整理

file rename / moveはlogical identityを変えないorganization operationとする。
source root内の明示destination、collision停止、external change検査、dirty対象の扱いを先に定める。
単一fileから始め、dirtyなら保存・破棄・中止を選んでから操作する案を推薦する。folder一括移動やdeleteは同時に入れない。
semantic内容が同じでもpath/bytes入力のhashは変わり得るため、receipt / cacheの同一性まで保証しない。

### Webへの展開

Standalone Webでpermissionを持つworkspaceのFile View・編集・検証を再利用する。
hostが必要なwrite / identity検査を提供できなければ理由付きで操作不可にし、保存成功を近似しない。
Connected Webはnative Build / Publishを追加し、handshake・authorization・workspace scope・capabilityを維持する。
接続断やpermission失効で未保存入力を失わず、結果不明writeを自動retryしない。
pairing、transport、installer、browser、workspace handoffは独立package。通常利用にserver/accountを必須化しない。

## 一気に実装するためのpackage境界

「まとめて進める」は、毎回product priorityを決め直さず、合意した完成条件まで内部taskを自律分解する意味で提案する。
未承認semanticsの自動承認やrepository lifecycle変更を意味しない。

| Package | まとまり | Depends on | 完了を観測する仕事 |
| --- | --- | --- | --- |
| P1: 日常編集 | range / clipboard、未保存Undo、検索・filter、complex initializer | 現在の基盤、clipboard / history仕様 | 一つのfileで多数cellを変更し、戻し、差分確認して保存できる |
| P2: 見渡す・選別する | Overview、Tag、Profile設定・preview、Problemsのsnapshot表示 | P1、config保存契約 | 分割TableとProfile差を確認し、対象sourceへ戻って修正できる |
| P3: ゲームへ渡す | Project入口、Build / Publish、capability・結果表示 | P2、既存Build / Publish engine | 新規Projectから保存・Build・Unity向け配置までGUIで完結 |
| P4: 定義変更の拡張 | key定義・既存key値編集、source単一file移動 | 個別migration / source管理契約 | 影響を確認して変更し、stale / failureで安全に停止 |
| P5: 制作支援View | expression、aggregate、共有View、装飾 | P2、language / metadata契約 | drop率をsource無変更で計算・共有 |
| P6: Reference・Web | 別々の独立packageとして詳細化 | 各ownerの未決事項解消 | 明示関係の検証、hostを越えた同じauthoring |

**推薦する最初の一括実装契約はP1–P3。** P4–P6は方向を共有するがv1を止める条件にはしない。
P1–P3のspec deltaを一つのreview packageとして揃え、共通状態を先に定義してsurface別に実装する。
承認後のcomponent、algorithm、private API、内部task分解は実装側で決め、observableな決定だけHumanへ戻す。

## 受け入れシナリオ

canonicalへの移行時に各ownerのstable Requirement IDとtestsへ分解する。scenario名はRequirement IDやDiagnostic Codeではない。

| Scenario | 成功の観測 | 失敗・境界の観測 |
| --- | --- | --- |
| 新規制作 | Project、型、Table、Dataを明示作成しnested値を保存 | collisionで既存file不変。別Dataを暗黙作成しない |
| 配置先設定 | Unity向けC# / binary pathを設定し、保存後のpreflightで確認 | 設定保存だけではdestinationを変更しない。unsafe pathはPublish前に停止 |
| 表計算から調整 | string、数値、64-bit境界値をpaste、preview・Undo・Redo・Save | read-only混入は部分適用なし。shape不正とdomain-invalidを区別 |
| filter中fill | visible rowだけ変更しhidden rowとsource順は不変 | preview後sort変更で古いtargetへ適用しない |
| complex値 | Custom / Arrayを編集し同じeditorでinitializer作成 | sibling・comment・quote保持。lossy number変換なし |
| 分割Table | 同PKの別occurrenceを表示し正しいsourceへ移動 | stale occurrenceを別rowへ推測で再接続しない |
| Profile | selected row / 件数がshared selectionと一致 | missing profileでfallbackせずdirtyを保存済みpreviewへ混入しない |
| config競合 | ProfileのDiffが対象sectionに限られ再読込できる | external change / unsupported syntaxでbroad rewriteしない |
| 保存とBuild | invalid値を安全にSaveでき、Build失敗を別表示 | dirty Buildはdiskだけ使用。Save失敗で履歴を消さない |
| 外部編集 | Conflictでlocal bufferを残しCompareへ進む | Outcome Unknownは自動retryせずactual sourceを確認 |
| Publish | 同じreceipt-valid setを全targetへ配布し結果を表示 | preflight失敗は全target不変。一部失敗を全成功扱いしない |
| Recovery | 原因とaffected filesを確認しread-only navigation継続 | Recovery Required中はSave / Migration / Buildを開始しない |
| 入力と遅延 | keyboardでrange・nested editor・Problemsを操作 | 古い応答で新しい入力・diagnosticsを上書きしない |

性能は未計測の「大規模でも快適」という保証にしない。合計10万record、一つの表示Table 20列、1万cell pasteを
暫定benchmark入力に提案する。製品上限ではない。reference machineとlatency目標は測定前に固定し、navigation、preview、
validation、memoryを別に測る。進捗表示と入力保全を先に確認し、未測定のms保証を承認済みにしない。

## Affected Specifications

| Owner | Current status / affected IDs | 提案delta |
| --- | --- | --- |
| [Data Editor](../gui/data-editor/spec.md) | Approved; `GUI-DATA-EDIT-001`, `GUI-DATA-LAYOUT-003`, `GUI-DATA-STATE-001` | range / Undo追加。Overviewは別surface。P1–P3では既存key制限維持 |
| [Source Record Edit](../specs/source-edit.md) | Approved; `SOURCE-EDIT-002`, `SOURCE-EDIT-003`, `SOURCE-EDIT-007`, `SOURCE-EDIT-014` | bulk合成とTag authoringを別ownerへ。安全性・lossless性は維持 |
| [Build Selection](../specs/build-selection.md) | Approved; `BUILD-SELECT-007`, `BUILD-SELECT-008`, `BUILD-SELECT-009` | semantics維持。GUI editor / previewを追加 |
| [Project layout](../specs/project-layout.md) | Approved | config保存は新owner候補。Project identity維持 |
| [App shell](../gui/app-shell.md) | Approved; `GUI-SHELL-PROJECT-001`, `GUI-SHELL-CAPABILITY-001`, `GUI-SHELL-VALIDATE-001` | creation入口、navigation、snapshot表示 |
| [Table Editor](../gui/table-editor/spec.md)、[Type Editor](../gui/type-editor/spec.md) | Approved | initializer typed UI。P4 key編集は別delta |
| [Build pipeline](../specs/build-pipeline.md) | Approved; `PUBLISH-003`, `PUBLISH-EXEC-001`, `PUBLISH-EXEC-002` | domain変更なし。GUI契約から参照 |
| [Schema Migration](../specs/schema-migration.md) | Approved; `MIGRATION-002`, `MIGRATION-010` | P4は別承認。Saveにmulti-file保証を転用しない |
| [Reference](../specs/index-and-reference.md) | Draft; `REF-001`, `REF-002`, `REF-003` | P6で詳細化、P1–P3へ先取りしない |
| [Runtime hosts](../specs/runtime-hosts.md) | Approved | 既存composition維持。P6 protocolは別途 |

## Confirmed Decisions

既決表のApproved / Implemented authorityとAccepted ADRの境界。
今回のHuman決定はOption Bの方向選択と詳細化のscopeである。specificationのApprovalではない。

## New Requirements

None identified as approved or newly normative requirements. 新behaviorは比較可能な設計提案として保持する。
採用後にowner別spec-changeでstable IDと規範強度を確定し、アイデアを既決のMUSTにしない。

## Changed Requirements

None identified as applied changes. 現行canonical specificationは変更しない。
予定deltaはAffected Specificationsのとおり。P1–P3と後段を別lifecycleで扱う。

## Compatibility Impact

- P1–P3は既存型・Table identity・MessagePack key・generated API・binary formatを変えない。
- Tag / Profileは既存shapeを使用するが、保存契約を追加する。全面formattingはしない。
- bulkもfile単位保存。Save All部分成功をglobal atomic successに言い換えない。
- sort / filter / Overviewはcanonical orderingと独立。occurrenceを永続record IDにしない。
- P4のkey変更はquery APIとdataset validityへ影響し得る。P5のmetadata formatは独立承認。
- released-version互換性は未保証。異なるschemaのC# / binary混在を許可しない。

## Potential ADRs

- P5: expression評価をshared Rustで行うか別runtimeを使うか。推薦はshared Rustだが未決定。
- P5 / Annotation: metadata ownershipとrecord紐付け寿命。
- P6: Native Host transport / pairing / workspace handoff。ADR 0006を再定義せず具体化する。
- P1–P3は既存architecture内を想定。componentやprivate API選択だけのADRは要求しない。

## Implementation Impact

| Boundary | 想定作業 |
| --- | --- |
| `masterdata-core` | bulk candidate、typed filter、Tag patch、config保存契約。domain rule集約 |
| `masterdata-app` | buffer合成・Overview・selection preview・config edit・Build / Publish orchestration |
| React / Tauri | range / history / navigation / initializer / settings / build、snapshot-aware DTOと薄いadapter |
| CLI | 既存操作のregression確認。新grammarが必要なら別delta |
| codegen / .NET | P1–P3でformat再設計なし。build / reload evidence維持 |
| Tests / fixtures | clipboard、64-bit、source preservation、競合、stale応答、Publish部分失敗。fixtureはtemporary copy |
| Verification | focused core/application/adapter/GUI tests、制作scenario、`cargo xtask check-all`、Desktop keyboard / focus |

## Open Questions

### 方向選択時の比較

| Choice | 推薦 | 代替 / trade-off |
| --- | --- | --- |
| 完成範囲 | B: P1–P3 Desktop制作v1 | Aは小さいが体験が途切れ、Cは未決事項が多い |
| 横断編集 | Overviewは保存済みread-only、mutationはfile単位 | 全buffer横断編集は便利だがtransaction / snapshot合成が増える |
| Undo | Save成功で未保存履歴clear | Save越え履歴には外部変更とのrebase / revert契約が必要 |
| 一括入力 | scalar range、preview、operation単位のbuffer適用 | complex / multi-file pasteは表現とrecoveryが広がる |
| 計算列 | P5へ分離し最初はbounded expression | 任意codeは自由だがsecurity・determinism・host互換の負担が増える |

Option B、file単位、保存前Undo、scalar一括入力、計算列後段化はHuman選択済み。詳細behaviorのApprovalは0016–0018で別途行う。

### P1–P3の詳細化先

- TSV quoting / newline / empty-cell codec、nullable入力、copy format。
- filter / sort operator、null / invalid値、bulk target固定のstate transition。
- range・nested editor・通常text inputのshortcut precedenceとfocus復帰。
- Profile / publish targetのTOML保存のpreservation、unsupported syntax、commit result。source-editとの共通化範囲。
- Build / Publishの既存service情報と、新しいread-only preview APIが必要な情報の区別。
- benchmarkのreference machine / latency。実利用規模が得られれば暫定入力を更新する。

上記P1–P3のdetailは0016–0018へ具体化した。benchmarkは測定evidenceをcompletionに要求し、未測定latencyをproduct保証にしない。
Reference syntax、expression grammar / numeric model、metadata format、Annotation identity、Web protocolは後段packageの開始条件であり、P1–P3に混ぜない。

## 初回設計時のレビュー（Review）

以下は方向選択前のRFC review記録。現在の詳細package reviewは[0018](../spec-changes/0018-desktop-build-delivery.md#review)が所有する。

`review-spec`の観点で別の読み直しpassによるself-reviewを実施した。独立agent reviewは実施していない。
新規ProjectからPublishするscenarioに対しtarget設定のGUI導線が不足していたため、v1へ追加した。

### Blocking Issues

canonical implementation contractの一括承認にはP1–P3のdetailとowner別spec deltaが不足する。
設計方向を比較・選択するRFCでは未確定境界を明示している。

### Non-blocking Issues

性能入力は暫定で実dataset / hardwareの裏付けが必要。現行code調査はservice / testの確認であり全機能の実機検証ではない。

### Questions

product choiceの推薦一式を採るか。採用後はdetailを具体化し、P1–P3のspec-change一式をreview対象とする。

### Approved as Proposed

No — 実装仕様のApproval recommendationではない。方向選択は可能だが未解決detailを持つRFCを実装authorityへ昇格させない。

| 観点 | Verdict |
| --- | --- |
| Intent fidelity / Unrequested behavior | 創造的な設計依頼に応える提案。原要望と解釈を分離 |
| Internal consistency / Normative strength | 提案はnon-normative。file単位境界維持、新delta明示 |
| Cross-spec consistency / Terminology | identity、Save、Migration、Publish、hostをownerへroute。Referenceを先取りしない |
| Testability / Unresolved ambiguity | scenario提示。codec / config保存等を実装承認前の残件として明記 |
| Backward compatibility | P1–P3でdomain format変更なし。後段API / metadata impact分離 |
| Implementation leakage | component / private APIを固定せずshared Rust / .NET境界維持 |

## 決定（Decision）

2026-09-16、Humanは直前の自己完結した「進める」の意味に対して「進める」と回答し、Option B / P1–P3を選択した。
file単位編集、保存前Undo、scalar一括入力を基本に詳細化し、計算列は後段とする。
本RFCをAcceptedとするが、0016–0018のspecification Approvalではない。採用方向の詳細proposalをreviewし、
明示Approvalとcanonical適用後に実装へ進む。
