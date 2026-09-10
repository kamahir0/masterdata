# RFC: 最初のGUIレコード編集・保存体験

Status: Draft

## 背景（Context）

YAMLを手書きせずにマスターデータを編集し、Gitで変更を確認できることは、
[Product vision](../product/vision.md)のauthoring systemとしての中心的な価値である。
Build / PublishやMigration内部処理を増やすだけでは、この体験は完成しない。

2026-09-10のHumanとの議論では、AddField source commit safetyのfinal verification後に、
「GUIでレコードを編集・保存し、差分と検証結果を確認できる体験」を具体化する推薦に対し、
Humanが「進めて」と指示した。これを次priorityの選択として記録する。
以下の初期対応範囲、保存方式、validation policyまで承認されたとは扱わない。
本RFCは比較・相談用であり、implementation authorityではない。

## 根拠と分類（Source Evidence and Classification）

| 分類 | 内容 |
| --- | --- |
| Decision | 次priorityを既存recordのGUI編集・保存体験の具体化へ進める。 |
| Requirement | GUIから値を変更・保存し、差分と検証結果を確認できるようにしたい。exact behaviorと強度は仕様化時に確定する。 |
| Constraint | YAML正本、shared semantics、Tauriからapplication serviceへの委譲は既存Approved authorityに従う。 |
| Proposal | 初期版は既存recordの非key・Required Primitive fieldに絞る。未承認。 |
| Proposal | 明示Save、変更箇所中心のsource preservation、stale拒否を採る。未承認。 |
| Open Question | 対応型、保存単位、validation errorの保存可否、dirty stateの扱いなど下記の選択。 |

## 課題（Problem）

既存GUI shellはproject情報、Validate、Buildを提供するが、テーブル編集を所有するApproved GUI仕様はまだない。
[GUI app shell](../gui/app-shell.md)のproject picker、unsaved changes、file watcher、
[YAML subset](../specs/yaml-subset.md)のGUI save preservationは未決定である。

Migrationはschema変更の意味を所有する。通常のrecord値編集をAddFieldやMigration Commandへ押し込まない。
そのsnapshot / patch / commit処理を内部で再利用できるかは、record保存のcontractを決めた後に評価する。

## 目標（Goals）

最初の到達点の候補は次の一連の操作である。

1. Desktopで既存Projectを選んで開く。
2. logical Tableを選び、複数data fileに分かれた既存recordsを確認する。
3. recordを選び、対応するfieldの値を変更する。
4. 保存予定のsource差分とvalidation resultを確認し、明示Saveする。
5. 開き直して変更を確認でき、Git diffでも意図した変更を追える。

これはacceptance scenarioの案であり、全項目の詳細がApprovedという意味ではない。
既存のValidate / Buildへ接続できることを保ち、Unityへの最終反映は後続の製品受け入れ候補とする。

## 非目標（Non-Goals）

以下は初期版から外す提案であり、製品の将来要望の撤回ではない。

- recordの追加・削除、schema編集、RenameField / DropField、MasterReference設計。
- spreadsheet同等のcell range操作、一括paste、複数record同時編集、履歴を持つUndo/Redo。
- Build Profileの設定・選択、タグ編集、GUI Publish、Git commit/push操作。
- Standalone / Connected Web、Native Host、installer、全hostの同時完成。
- formatter、全ファイルの再serialize、generated C# / binaryの編集。

## 選択肢（Options）

### OQ-A: 最初に編集できる範囲

| 案 | 利点 | 費用・制約 |
| --- | --- | --- |
| A（推奨）: Required Primitiveの非key fieldだけ | 保存・競合・差分確認を含む一連の体験を先に閉じられる | Enum / Value Object / Nullable / Array / Custom Typeは当初read-only |
| B: Primitive + Enum / Value Object / Nullable | 普通のゲームマスターへの適用範囲が広い | 型別editor、null入力、表示・wire representationの検証が増える |
| C: 全型とkey変更を同時に扱う | 型の制約が少ない | identity変更、nested editor、複合validationまで同時に決める必要がある |

AでいうPrimitiveは[Primitive Types](../specs/type-system/primitives.md)の全8種類。
Primary / Secondary Keyの構成fieldは初期版ではread-onlyとする提案。
複合型を持つTable全体を隠すのではなく、非対応fieldをread-onlyで保持・表示する案とする。
数値の値域は既存ownerに従う。特にlong / ulongをfrontendの数値変換で丸めることは許されず、
exact representationは共有境界の設計課題として残す。

### OQ-B: 保存と差分

推奨案は「一度に1 recordを編集し、明示Save、保存前のsource差分表示」である。
1 recordに対する複数field変更はまとめて保存し、変更対象data fileだけを更新する。
未変更fileはbyte-for-byte保持。変更fileでもrecord順、member順、コメント、改行、indentation等の
無関係なtextを保持し、変更valueのquote/styleは意味を正しく表すために必要な範囲で変更を認める案。
source locationを安全に特定できない場合は全fileの再serializeへfallbackせず保存不可を示す案。

代替はautosave、またはTable全体の一括Save。これらは保存タイミング・複数file failure・dirty管理の範囲が広がる。
Gitそのものの起動やstage/commitは不要とし、UI内のsource diffはGitがないprojectでも表示する提案。
このpreservation policyはGUIに固有のdomain logicではなく、採用後にshared source-edit仕様へrouteする。

### OQ-C: validation・未保存変更・競合

推奨する最初のbehavior packageは以下。

- 編集中のbufferに対する検証と、保存済みsourceに対する検証結果を区別する。
- 変更値が型・YAML表現としてinvalidならSaveを止め、入力は保持する。
- unfiltered validationのproject errorを表示するが、保存対象の値編集と無関係な既存errorだけでSaveを止めない。
  保存可能条件はMigration仕様を流用せず、source-editのpreconditionとして別途定義する。
- planで読んだsource/configが保存直前に変わっていたら保存を止める。外部変更を上書き・自動mergeしない。
  未保存入力を保持し、reloadで破棄するか、画面に留まるかを選べる。
- dirty状態のrecord/Table/project切替、Reload、window closeでは「保存／破棄／キャンセル」を明示する。
  保存失敗なら切替・closeを完了しない。操作不能なSaveを選ばせない。
- Buildは保存済みsourceだけを使うため、初期版ではdirty中は開始できない。暗黙Saveはしない。
- 通常I/O failure時の保存結果と元データの保持をshared serviceで扱う。
  crash/power lossまでの保証を、この小さなGUI taskの中で追加しない。

代替は「project validation errorが1件でもあれば保存禁止」または「invalidな編集値も保存可能」。
前者は無関係な不備の修正作業まで妨げ、後者はinvalid sourceのauthoring契約を広げるため、初期版には推奨しない。

## トレードオフ（Trade-offs）

Aと明示Saveを採れば、利用者が操作できる最小の閉じた体験を先に作れる。
単なるread-only viewerだけでObjectiveを閉じず、同じimplementation work packageに保存・error path・操作testを含める。
一方、型対応の狭さは明示し、非対応値を空文字やdefaultへ変換して保存しない。

厳密なsource preservationはformatterより実装費用がかかるが、YAML + Gitと外部編集との共存に直結する。
見た目を先に整えるだけで保存のcontractを後付けする進め方は避ける。

## 提案（Proposal）

まずOQ-A〜Cのproduct choicesをHumanが選び、その結果を次のownerへ分けてDraft化する。

- `docs/gui/`の新しいrecord editor仕様: navigation、selection、dirty、差分表示、focus、keyboard、error state。
- `docs/specs/`の新しいsource-edit仕様: record値変更、snapshotとsource provenance、保存precondition、失敗結果、preservation。
- 既存Approved ownerの意味を変更する必要がある場合だけ`docs/spec-changes/`へdeltaを隔離する。

[Table / Key](../specs/table-and-keys.md)のschema field order、複数fileのlogical Table、
[Build Selection](../specs/build-selection.md)のselection前後の違いは既存authorityから参照する。
PKだけを編集対象の永続identityにすると、タグで排他的な同一PKのsource recordsを区別できない。
source snapshot内でrecordを一意に特定する手段は必要だが、path/record ordinalを新しいdomain identityにはしない。
[ADR 0002](../adr/0002-rust-core-shared-by-cli-and-gui.md)と[ADR 0006](../adr/0006-host-capability-composition.md)のshared boundaryを維持する。

## 互換性（Compatibility）

新しいauthoring入口の追加であり、既存YAML、type、key、binary、build/publish semanticsを変更する意図はない。
GUI上の並べ替えや選択をsource/domain意味へ持ち込まない。
GUI Saveの具体的contractは新規で、現行Migrationの仕様や成功条件から暗黙に導出しない。

## 未解決事項（Open Questions）

最初にHumanが選ぶのはOQ-A（型と操作範囲）、OQ-B（保存・preservation）、OQ-C（errorとdirty policy）。
その後、採用範囲に応じて下記をcanonical Draftで閉じる。以下は未承認のため実装時に推測しない。

- project pickerの入力方法、開けないproject、empty table、schemaをresolveできない状態の操作。
- record表示順、重複PKを含むsource recordの識別・表示、非対応値・literal block stringの表示と編集範囲。
- exact scalar transport、文字列・数値の入力中状態、変更を元の値へ戻した場合のdirty判定。
- Save preconditionとunrelated errorの分類、検証snapshot、file identity/path safety、通常I/O failureの結果。
- 差分の表示単位、保存結果が不明な場合の再試行、reload後の入力復元の範囲。
- loading/saving中の操作、keyboard/focus、accessibility、shortcut、product textの言語。

この一覧を全てHumanへの逐次質問にはしない。既存Approved authorityからrecoverできる事項は整理し、
新しいobservable choiceだけを比較可能な案として提示する。

## 受け入れ候補（Acceptance and Implementation Impact）

仕様化後に次を実際のUI操作・shared service testsで確認する案。source内の文字列検査だけで完了扱いにしない。

- project選択 → Table → record → 値変更 → source diff → Save → reopenで変更を確認する。
- split data files、同一PKの別source record、非対応field、64-bit整数の精度を確認する。
- 値invalid、external edit、I/O failure、dirty中のnavigationでsourceと入力を保護する。
- 未変更text/fileを保持し、Saveだけでbuild/publishを開始しない。
- 既存CLIのvalidationとGUIの保存済みsource validationが同じdomain resultを返す。

影響候補は`masterdata-core`、`masterdata-app`、Tauri command、共有frontend、fixturesと操作test。
.NET bridgeやMasterMemory formatの変更は不要と見込む。library、CST、RPC schema、public commandはここで選択しない。

## Refinement report

### Affected Specifications

GUI app shell（Draft）、YAML subset（ApprovedのGUI save Open Question）、Table / KeyとRuntime hosts（Approved）。
既存normative requirementの変更はない。

### Confirmed Decisions

GUI編集・保存体験を次priorityとして具体化すること。OQ-A〜Cの具体案は未決定。

### New Requirements

None identified。新規normative IDは選択後のcanonical Draftで割り当てる。

### Changed Requirements

None identified。

### Open Questions

上記OQ-A〜Cと採用後の詳細事項。

### Potential ADRs

既存shared boundaryで足りる見込み。source editing方式のarchitectural trade-offが必要なら別途検討する。

### Compatibility Impact

上記互換性節を参照。既存contract変更なし。

### Implementation Impact

上記受け入れ候補を参照。今回はruntime変更なし。

## レビュー（Review）

`review-spec`でintentと既存authorityを別passで照合した。

### Blocking Issues

OQ-A〜Cおよび保存の詳細contractが未決定。実装仕様としての承認・implementation-ready判定は不可。
比較用Draftとしては未確定を明示しており、既存Approved semanticsへの混入はない。

### Non-blocking Issues

None identified。

### Questions

OQ-A〜Cについて推奨案を採るか。選択後の詳細仕様も別途reviewとHuman Approvalが必要。

### Approved as Proposed

No。比較用Draftであり、未決定事項を保持している。Human Approvalや実装開始を代替しない。

| 観点 | 評価 |
| --- | --- |
| Intent fidelity / Normative strength | priority決定と未承認の提案を区別。新規MUST等は追加していない。 |
| Internal / Cross-spec consistency | shared semanticsとsource正本を参照し、Migrationを値編集のauthorityにしていない。 |
| Terminology / Backward compatibility | 既存用語を使用。新しいrecord identityやformatを定義していない。 |
| Testability / Unresolved ambiguity | 操作による受け入れ候補を列挙。詳細の未決定をOpenとして保持。 |
| Implementation leakage / Unrequested behavior | library/APIを固定せず、追加案を採用済みと扱っていない。 |

実装diffがないためimplementation rationale reviewは対象外。

## 決定（Decision）

比較案の採用は未決定。RFCはDraftのままとする。
