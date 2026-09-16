# 仕様変更: Desktop制作v1 — P2 Workspace・Tag・設定

Status: Proposed

## 根拠と分類（Source Evidence and Classification）

- Decision: Humanは2026-09-16にRFC 0008 Option Bの詳細化へ進むことを選択した。横断viewは保存済みread-only、mutationはfile単位。
- Requirement: 分割TableとProfileごとの対象を把握し、Tag / Profile / 配置先をGUIで編集する。
- Constraint: occurrenceとPKの分離、shared selection、source/config preservation、lost-update防止、Build / Publish非連動を維持。
- Proposal: 以下のTag patch、config edit範囲、結果・recovery・navigationは今回の詳細案。規範文は採用後の契約案であり、現行Approved behaviorではない。

## Affected Specifications

| Owner | 現Status / affected IDs | 適用先 |
| --- | --- | --- |
| [Source Edit](../specs/source-edit.md) | Approved; `SOURCE-EDIT-002`は通常value editのまま | 新規`docs/specs/source-tag-edit.md`へSOURCE-TAG family |
| [Record Mutation](../specs/source-record-mutation.md) | Approved; `SOURCE-RECORD-007`, `SOURCE-RECORD-003` | Tag composition参照追加、metadataとfieldを分離 |
| [Data Editor](../gui/data-editor/spec.md)、[Record Mutation GUI](../gui/data-editor/record-mutation.md) | Approved; `$tags`非目標、dirty / Save | GUI-TAG family追加、general lifecycleは維持 |
| [Build Selection](../specs/build-selection.md) | Approved; `BUILD-SELECT-001`〜`BUILD-SELECT-017`維持 | 新規`docs/specs/authoring-query.md`へAUTHORING-OVERVIEW family |
| [Project layout](../specs/project-layout.md) | Approved; config shape / identity変更なし | 新規`docs/specs/project-config-edit.md`へCONFIG-EDIT family |
| [App shell](../gui/app-shell.md) | Approved; `GUI-SHELL-PROJECT-002`, `GUI-SHELL-CAPABILITY-001`, `GUI-SHELL-VALIDATE-001` | config dirty・recovery gate合成 |
| 新GUI surface | 新規 | `docs/gui/table-overview/spec.md`へGUI-OVERVIEW、`docs/gui/project-settings/spec.md`へGUI-SETTINGS |

上記新pathは承認後の配置計画。[0016](0016-desktop-daily-editing.md)と[0018](0018-desktop-build-delivery.md)が一括package。

## Confirmed Decisions

保存済みTable Overview、file単位authoring、Tag / Profileの既存semanticsを使う方向。new persistent record identityやtag registryを導入しない。

## New Requirements

### SOURCE-TAG-001

Tag editは通常record member editと別のsemantic requestで、base snapshot内のoccurrenceまたはAdded draftの`$tags`だけを対象にしなければならない（MUST）。既存domain fieldsのeditorへ`$tags`をschema fieldとして加えてはならない（MUST NOT）。Pending deleteにはTag edit不可。
requestはordered string entriesを保持し、Add / Remove / Replace entryを提供する。case修正、trim、deduplicate、sortを暗黙実行してはならない（MUST NOT）。invalid lexical tag / duplicateはshared diagnosticとし、安全なsource candidateならSaveを拒否しない。

### SOURCE-TAG-002

既存`$tags`がstring sequenceなら個別entryをsource-preservingにpatchし、変更対象外entryとcommentを保持しなければならない（MUST）。non-sequence、non-string element、location曖昧なsourceは理由付きread-onlyとし、推測変換やsubtree/full-file再serializeをしてはならない（MUST NOT）。通常value editや閲覧まで無効にしない。
未指定`$tags`へ最初のentryを追加する場合はrecord mapping末尾へmetadata entryを挿入する。全entryを削除した既存propertyは`$tags: []`のempty sequenceとして保持する（MUST）。未指定から追加して全て戻した場合は元の未指定へ戻し、他変更がなければbyte-identical / cleanとなる。unrelated commentは削除しない。
block / flow string sequenceとblock record mappingへの新規property追加を最低対応範囲とする。それ以外はsafe localization可否をshared layerが報告する。

### SOURCE-TAG-003

Tag、existing value edits、Added drafts、Pending deletesを同一fileの一candidateへcompositionしなければならない（MUST）。deletionが最終candidateで優先し、Undo Deleteで削除前Tagと値を復元する。Added draftのTagは初期未指定、入力した場合はdeclared fieldsの後にmetadataとして出力する。
Source Editのfile commit / result / lost-update / explicit Overwrite / Outcome Unknown / 非連動と0016のhistory contractを適用しなければならない（MUST）。Tag editはschema/key mutationを開始しない。

### GUI-TAG-001

Data Editorは選択recordのTag editorをdomain列と別に提供し、add/remove/replaceをkeyboardで操作できなければならない（MUST）。tag確定を一history操作とし、入力中textと確定entryを区別する。空textも勝手に破棄せず、確定すればinvalid tagとして診断する。
候補は読込済みsource tagsと保存済みprofile tagsのunionをshared layerから受け取り、一覧が部分的ならその旨を表示する（MUST）。候補選択は任意で新tag入力を許可する。Add Rowと既存rowの両方に提供し、filterやProfile選択を理由にsource tagsを変更しない。

### AUTHORING-OVERVIEW-001

shared applicationは保存済みconfig、source membership、対象Tableと解決に必要なschema/type/data bytesを識別するsnapshotを取得しなければならない（MUST）。read-only queryはこのsnapshotとresult identityを返し、frontendはfilesystem discovery / YAML parseを行わない。
読込中の変更を検出した場合は結果をcurrent completeとして返さず、再読込要求として扱う（MUST）。filesystem全体のglobal atomic snapshotを保証する意味ではない。
各rowはTable identityとsource provenanceを持ち、同じPKでもdeduplicateしない。default orderはproject-relative source pathのdeterministic順、そのfile内source順。PKによる並べ替えは0016の明示queryだけで行う。

### AUTHORING-OVERVIEW-002

Overviewはdomain-invalid recordを黙って捨ててはならない（MUST NOT）。安全にproject/targetをresolveできるrangeを表示し、parse不能・membership不明・type unresolved等で欠落があればincompleteと識別する。完全な対象件数やselection結果を推測してはならない（MUST NOT）。0件とload failureを区別する。
render可能なrowにはSOURCE-EDIT-015のlossless projection / read-only reasonを使用する。Profile-independent検証によりselectionを安全に実行できない場合、raw閲覧とdiagnosticsは提供してもselected件数と理由はUnavailableとする。

### AUTHORING-OVERVIEW-003

Profile previewは同snapshotの保存済みprofileを解決し、Build Selection ownerを使用しなければならない（MUST）。初期はunfiltered、選択はこのProject sessionの明示state。消えたprofileはmissing stateになり、unfilteredへfallbackしない（MUST NOT）。
結果はTable別total/selected count、row別selected/excluded、matched include/exclude tags、include-emptyかinclude-unmatchedかを構造化して返す（MUST）。exclude優先を既存formulaから説明し、GUIで別selection判定を行わない。
defaultは全rowにmembership表示。selected-onlyは明示view filterとする。countはsearch/filter前datasetを表し、現在表示件数と区別する。Profile変更は通常File Viewのqueryやrowを変更しない。
dataset diagnosticsはshared validatorから取得し、unsupported/Draft Reference semanticsをfrontendへ実装してはならない（MUST NOT）。

### GUI-OVERVIEW-001

OverviewにはTable、source file/occurrence、保存済みsnapshot、Profile、dirty file数、dirty config有無を表示し、未保存変更は未反映と識別できなければならない（MUST）。cellはread-only。refresh、sourceへ移動、search/filter/sort、Profile選択をkeyboardで実行できる。empty / loading / partial / unavailable / staleを区別する。
source/config変更を検出したら表示をstaleにし、read-only refreshでdirty bufferを保存・破棄してはならない（MUST NOT）。古いresponseは新query/profile/resultへ適用しない。

### GUI-OVERVIEW-002

rowからsourceへ移動するときは、対象file snapshot identityとeditor base identityの一致をshared layerで確認しなければならない（MUST）。一致するdirty editorでは同じoccurrenceを選択し、未保存値は保持する。Pending deleteならそのUndo導線へ移動する。
不一致ならfileを開いてsnapshot差を示してよい（MAY）が、rowを推測選択せずOverview refreshを案内する。PK/nameで再接続してはならない（MUST NOT）。queryで非表示なら一時的にqueryをclearしたことを伝え、source rowにfocusする。

### CONFIG-EDIT-001

設定editorはexisting `masterdata.toml`のexact base bytes、project binding、config buffer revisionを使用し、次のtyped operationだけをv1で提供しなければならない（MUST）。

- named Profile追加、existing Profileのinclude_tags / exclude_tags更新。
- Publish target追加、existing targetのpath更新。existing targetはconfig snapshot内のarray occurrenceで指定する。

Profile rename/delete、target remove/reorder/kind変更、project id / roots / artifact path等のgeneral config編集はv1外。target追加時のkindは`csharp`または`binary`を明示入力する。source rootやdirectoryからkindを推測してはならない（MUST NOT）。

### CONFIG-EDIT-002

shared layerはTOML syntaxと編集targetをlosslessに定位し、対象propertyと必要syntaxだけをpatchしなければならない（MUST）。unrelated section、unknown section、comment、quote、newline、array要素順をpreserveする。config全体の通常serializer出力へfallbackしてはならない（MUST NOT）。
最低対応は`[build.profiles.<name>]` table内のstring array、`[[publish.targets]]`内のstring path、両方の追加とする。quoted key、multiline array、inline commentを含むvalidな最低形を扱う。inline table等の別TOML表現を安全に定位できなければ理由付きunsupportedとし、黙ってnormalizationしない。
既存array更新はentry単位でpreserveする。新entryは末尾、削除entryは必要separatorとともに除去し、外側standalone commentを所有推測で削除しない。明示empty listは`[]`、既存未指定から変更後元に戻した場合は未指定の元bytesへ戻す。新Profile/targetはconfig末尾へ必要な区切り付きで追加し、無関係な既存bytesを変えない。

### CONFIG-EDIT-003

candidateはvalid TOMLと一意なtargetを維持し、変えた入力をlosslessに表現できなければならない（MUST）。malformed TOML、duplicate table/property、target ambiguousはSave candidate failure。範囲を安全に特定できないconfig全体はtyped編集不可とする。
一方tag entryのgrammar、duplicate/overlap tag、既存unknown profile key、target path等のdomain diagnosticだけでSaveを禁止してはならない（MUST NOT）。この区別はsource-invalid状態を保持して直せるための提案であり、Project validatorのacceptanceを緩和しない。
新Profile名はBuild Selectionのgrammarを満たし既存名とcollisionしないことをcreate operationのpreconditionとする（MUST）。入力は修正できるformに保持し、silent repairしない。Profile rename/deleteを持たないv1で、修正不能な新しいidentityを生成しないためである。外部source由来のinvalid name / unknown propertyは保持し、本editorで修復対象外ならlocationと理由を示す。
保存済みconfigのdomain-invalidによりProject serviceが利用不能なら、そのstateとconfig editorの修正導線を保持し、古い正常configでBuild等を継続してはならない（MUST NOT）。config editorは既に確立したrootとexact config identityから再取得し、domain-valid Projectの再解決を編集開始の必須条件にしない。権限・root binding・構造の安全性を再確認できなければ停止する。

### CONFIG-EDIT-004

config Saveは一fileの明示commitとし、mutation直前にexact content identityを検査しなければならない（MUST）。mismatchはConflictで無変更、safe I/Oを満たす成功だけSuccessとする。FailureとOutcome Unknownを区別し、未保存入力を保持する。Outcome Unknownはactual bytes取得前にretryしてはならない（MUST NOT）。partial/truncated contentを成功扱いしない。crash/global transaction保証は追加しない。
v1 config ConflictにはCompare / Reload / Cancelを提供し、Overwrite / automatic mergeは提供しない。Reloadによる入力破棄は明示確認する（MUST）。新しいbase上で利用者が再編集してSaveする。
config保存はsource YAML、artifact、target filesystem、Gitを変更してはならない（MUST NOT）。path保存はそのdestinationへのpublish実行認可ではない。

### GUI-SETTINGS-001

Project SettingsはProfileとPublish targetsをtyped formで編集し、config file単位のdirty / Diff / Saveを持たなければならない（MUST）。section移動や他editorへのnavigationでbufferを保存・破棄しない。Cmd/Ctrl+Sはactive settingsのconfigだけを保存する。
form editのactive textを確定してからSaveし、表現不能なら入力を保持して停止する。domain-invalidならdiagnostics付きで確定できる。Save中は当該config編集を停止する。
settingsのgeneral Undo/Redoはv1必須ではない。text input内Undoは維持し、config全体の明示Discardは確認を経る。YAML file履歴と混同してはならない（MUST NOT）。

### GUI-SETTINGS-002

Project切替 / Reload / closeは、YAMLとconfigの全dirtyを同じSave All / Don't Save / Cancel guardへ含めなければならない（MUST）。Save Allはconfigを先に保存・project binding再解決し、config失敗/無効でYAML保存の安全なauthorityを取得できなければ以降を開始しない。config成功後にYAMLで失敗した場合は成功fileをrollbackせず、元の上位操作を止めて残bufferを保持する。all-or-none transactionではない。
通常config Save後にsource roots/identityが外部変更等で変わった場合は旧workspaceへ推測で接続しない（MUST NOT）。再open / reloadとdirty guardを要求する。
Migration Recovery Required中はconfig保存も停止する。read-only Diff / Compare / 診断は継続可能とする（MUST）。config dirtyだけで保存済みinputのBuild / Publishを禁止しないが、未保存設定は不使用と表示する。

### GUI-SETTINGS-003

設定form、Tag、Overviewのcontrolはlabelとstateをassistive technologyへ伝え、keyboard-onlyで操作・error確認・Diff・復帰ができなければならない（MUST）。unknown/unsupported設定を隠して消す代わりにreadonly reasonとlocationを提供する。
成功したconfig Saveはsaved profile/target一覧とread-only previewをinvalidateし、新snapshot取得後に更新する（MUST）。named profileのselectionを暗黙変更しない。

## Changed Requirements

- `SOURCE-RECORD-003`: schema field集合は現行どおり。optional Tag metadataはfieldとは別にSOURCE-TAGへ委譲する参照を追加。
- `SOURCE-RECORD-007`: record mutation setへoptional Tag editsをcompositionする。Pending delete優先、Undo復元は維持。
- Source Record MutationとGUIの`$tags`非目標は本Tag契約への参照へ変更。`SOURCE-EDIT-002`の通常value requestにTagを混入しない制限は維持。
- `GUI-SHELL-PROJECT-002`と`GUI-DATA-SAVE-003`の上位dirty guardはconfigを含むGUI-SETTINGS-002へ参照。file間navigationで確認を出さない契約は維持。
- `GUI-SHELL-CAPABILITY-001`: Migration Recovery Requiredでconfig Saveも止めるdeltaを追加。通常Publish-onlyの扱いは0018に明記。
- `GUI-SHELL-VALIDATE-001`: manual saved validationは明示Profileかunfilteredを対象とし、buffer検証、saved検証、selection previewをsnapshot付きで区別する。CLI validateのdefault変更を意味しない。

## Compatibility Impact

existing YAML / TOML canonical shape、tag selection formula、project identity、binaryは変えない。config-invalid保存が可能になるがvalidatorの許可範囲は変えない。
Profile/target occurrenceはsnapshot-local locatorであり永続IDを追加しない。new config editorのno-overwriteはYAMLのexplicit Overwrite許可を撤回しない。

## Implementation Impact

coreはTag source patch、config candidateとdiagnostic、query / selection結果。appはexact snapshot、保存、recovery、Overview / configのservice。
GUIは結果を表示しYAML/TOML parse・selectionを複製しない。既存publish engineのdestination検査をsettingsに再実装せず、0018のpreflightへ接続する。

### Acceptance

| Family | 必須evidence |
| --- | --- |
| SOURCE-TAG / GUI-TAG | absent→add→removeのbyte同一、既存[]、flow/block/comment、invalid/duplicate、Added/Deleted/Undo、同PK別rowの保全 |
| AUTHORING-OVERVIEW | split fileと同PK、partial parse、source途中変更、profile missing、exclude優先、selected-onlyとtotal countの区別 |
| GUI-OVERVIEW | dirty base一致/不一致、Pending delete、stale response、query clearとfocus、0件とUnavailableの区別 |
| CONFIG-EDIT | quoted table key、multiline array、comment/CRLF、unknown section、existing/new profile、target occurrence、unsupported inline table、domain-invalid保存、外部変更 |
| GUI-SETTINGS | active Saveはconfigだけ、Save Allでconfig成功→YAML失敗、config-invalid後の修正導線、Outcome Unknown、Recovery Required、navigation保持 |

fixtureはtemporary copy、patch boundaryはbyte比較、host failureはinjection、GUIはworkflow testsで確認する。

## Potential ADRs

None identified. configもshared candidate / host I/O分離の既存原則を使用する。

## Open Questions

None identified for this proposed scope. general TOML editor、profile rename/delete、target remove/reorder、metadata formatはscope外。
新しい詳細案は未承認であり、0016–0018全体へのApprovalを要する。

## レビュー（Review）

一括reviewは[0018](0018-desktop-build-delivery.md#review)に集約。

## 承認記録（Approval Record）

未承認。`Status: Proposed`。
