# 仕様変更: Desktop制作v1 — P3 Project入口・Build / Publish

Status: Proposed

## Package summary

**承認対象は[0016: 日常編集](0016-desktop-daily-editing.md)、[0017: Workspace・設定](0017-desktop-workspace-settings.md)、本0018の一式。**
一括Approval後に各ruleを指定canonical ownerへatomicに適用し、3 proposalをAppliedとする。部分採用なら依存関係を再reviewする。
RFC 0008のAcceptedはこのpackageのApprovalではない。現時点ではfeature実装を開始しない。

## 根拠と分類（Source Evidence and Classification）

- Decision: Humanは2026-09-16、Desktop制作v1のP1–P3を詳細化する方向を選択した。
- Requirement: 新規Projectから制作、保存、Profile選択、Build、Unity向けPublishまでGUIから行える。
- Constraint: shared native services、saved-source-only Build、receipt authority、all-target preflight、target-local failure、.NET委譲を維持。
- Proposal: GUI初期化の安全範囲、operation capture、preview freshness、result表示、concurrencyは以下の詳細案。規範語は採用後contractの文案であり未承認。

## Affected Specifications

| Owner | 現Status / ID | 適用先 |
| --- | --- | --- |
| [Project layout](../specs/project-layout.md) | Approved; `PROJECT-CONFIG-008`維持 | PROJECT-INIT family追加 |
| [App shell](../gui/app-shell.md) | Approved; `GUI-SHELL-PROJECT-001`, `GUI-SHELL-LAYOUT-002`, `GUI-SHELL-CAPABILITY-001` | GUI-PROJECT追加、新surfaceへの参照 |
| [Build pipeline](../specs/build-pipeline.md) | Approved; `ARTIFACT-SET-004/005`, `PUBLISH-EXEC-001`〜`PUBLISH-EXEC-005`維持 | BUILD-REQUEST / PUBLISH-PREVIEW family追加 |
| [Build Selection](../specs/build-selection.md) | Approved; `BUILD-SELECT-007/009`維持 | 選択済みprofile requestはexisting semanticの接続 |
| [CLI](../specs/cli.md) | Approved; `CLI-007`維持 | CLI grammar変更なし |
| 新GUI surface | 新規 | `docs/gui/build-publish/spec.md`へGUI-DELIVERY family |

## Confirmed Decisions

一つのDesktop制作workflowとしてP1–P3をまとめる。単独Save / Buildから外部Publishを暗黙実行せず、CLI / GUI / .NETの既存責務を維持する。

## New Requirements

### PROJECT-INIT-001

GUI向けCreate Project serviceは、既存のempty directoryか、実在parent直下の未存在new directoryを明示targetとして受け付けなければならない（MUST）。hidden entryを含むnon-empty directory、symlink/junction/reparse point経由のtarget、既存Projectとcollisionするtargetは書込み前にrejectする。空判定だけに依存せず生成fileはexclusive createとし、途中で現れたentryを上書きしてはならない（MUST NOT）。
project id/name等は既存shared init input / validatorを使用する。生成物はPROJECT-CONFIG-008のscaffoldだけとし、sample YAML、Unity directory、Git repositoryを暗黙作成してはならない（MUST NOT）。CLI initの既存directory対応をこのGUI向け入口で撤回するものではない。

### PROJECT-INIT-002

Create successはconfigと必須scaffoldの作成完了・shared Project再解決成功後だけ報告しなければならない（MUST）。failure時は既存workspaceを維持し、作成済みentryと残状態を判定可能な範囲で報告する。multi-file crash atomicityを保証してはならない（MUST NOT）。
部分作成物を無断cleanupしたりblind retryで上書きしてはならない（MUST NOT）。利用者はactual targetを確認し、Open Projectで開けるなら開くか、別のempty/new directoryで明示再試行する。不可逆な自動recoveryを本packageへ導入しない。

### GUI-PROJECT-001

未選択時にOpen / Createを提示し、Createはdestinationとproject inputを確認してから実行しなければならない（MUST）。既存Projectが開いている場合は、作成開始前に全dirtyのSave All / Don't Save / Cancel guardを通す。Cancelでは作成しない。成功後は新Projectを開き、失敗時は旧Projectを保持する。
入口はfolder作成、型、Table、Data file、Add Rowへのguided actionを提供するが、それぞれを独立した明示operationにしなければならない（MUST）。schema creationでData fileを暗黙作成しない。empty stateもkeyboardで次操作へ進める。

### GUI-PROJECT-002

Explorerのsource treeを維持し、logical Table一覧からOverview / schema、Type一覧からType Editorへ到達できなければならない（MUST）。一覧はshared workspace情報を使い、pathからidentityを導出しない。どの入口も同一sourceのbufferを共有する。
Recent Projectsはuser-localに最大10件、successful open順で保持する（MUST）。同じhostで同じcanonical rootは一件として扱い、missing/permission failureで他Projectを破壊しない。removeはrecent entryだけを消しdiskを変更しない。自動openや自動Buildを行わず、path情報をProjectのGit管理configやremoteへ送らない（MUST NOT）。

### BUILD-REQUEST-001

GUIのBuild requestはProject bindingと、unfilteredまたは明示named Profileをshared applicationへ渡さなければならない（MUST）。named Profile解決、source parse、validation、selection、Buildは既存ownerを使用し、frontend独自selectorやCLI subprocessを使用してはならない（MUST NOT）。
開始時に取得した保存済みconfig/source snapshotからresolved BuildPlanを構成し、そこから同じartifact setを生成する。読込中の変更が検出された場合はsnapshotを確定できないfailureとしてartifact publication前に停止する（MUST）。Plan確定後のsource変更はそのBuild inputへ混ぜず、operation結果はcaptured inputの結果として扱う。全filesystemに対するglobal lockを約束する意味ではない。
dirty YAML/config、Tag draft、View filter、clipboard previewを入力へ暗黙に含めてはならない（MUST NOT）。captured profileとconfig/inputの識別情報をsession resultへ残すがreceipt v1へnew required fieldを追加しない。

### PUBLISH-PREVIEW-001

GUI向けread-only Publish previewはshared receipt validation / all-target preflightを使用し、artifact-set identity、config identity、configured target kind/path、resolved destination、preflight結果を返さなければならない（MUST）。成功時は既存manifestに基づくC# addition/update/removalとbinary replacementの予定を示す。frontendがmanifestやfilesystem ownershipを再解釈してはならない（MUST NOT）。
preflight failureで全部の詳細を計算できなければ全targetをnot_attemptedとし、検査できなかった項目を未確認と表示する。最初のerrorだけ取得できるserviceでも未取得項目を成功扱いしない（MUST NOT）。previewはtarget parent作成を含むmutationを行わない。

### PUBLISH-PREVIEW-002

GUIのConfirm Publishは表示したpreviewと同じartifact setとconfigを対象にし、実行直前にidentityを再確認しなければならない（MUST）。変更時は無mutationでstale previewとして再確認へ戻す。
destination / manifest / unmanaged contentは毎回既存path-safety / all-target preflightで再検査する。表示した管理file集合やreplacement/removal計画が変われば古いpreviewでmutationせず再確認する（MUST）。mtimeだけをauthorityにせず、affected content/ownershipのexact identityを用いる。
全target実行を開始した後のfailureとcontinuationは既存PUBLISH-EXEC contractに従い、preview機能がcross-target rollbackやglobal atomicityを追加してはならない（MUST NOT）。execution-time recheckも既存path-safetyに従う。

### GUI-DELIVERY-001

Build / Publish surfaceはProject、Profile、dirty source数/config有無、Build/native capabilityと利用不可理由を表示し、Build / Publish last successful artifacts / Build and Publishを別actionにしなければならない（MUST）。初期Profileはunfilteredで、Overviewと同じ明示Project-session selectionを共有する。missing profileは選択がmissingと分かる状態でBuildを拒否し、unfilteredへfallbackしない。
Profile選択が無効でも、current configがload可能ならPublish-onlyのreceipt authorityを否定しない（MUST）。Publish-onlyはProfile selectionではなくcanonical setを対象とする。

### GUI-DELIVERY-002

Buildはdirty中も実行可能で、保存済みsource/configだけを使う表示と独立したSave All actionを提供しなければならない（MUST）。SaveをBuild actionの隠れた前処理にしない。
stateはidle、running、succeeded、failedを区別し、進行中operationのProject/captured Profileを表示する。結果にはstructured diagnostics、artifact root、receipt検証結果を保持する（MUST）。失敗時の既存artifact残存を今回Build成功と表示してはならない（MUST NOT）。errorから対応sourceへ移動する際は0017のsnapshot/provenance確認を通す。

### GUI-DELIVERY-003

Publish-onlyはread-only previewからConfirm / Cancelへ進み、success、preflight failure、execution failure / partial success、transport outcome unknownを区別しなければならない（MUST）。target別succeeded / failed / not_attemptedとdiagnosticを表示し、全体失敗をtoastだけで消費してはならない（MUST NOT）。
valid receiptで0 targetは既存contractどおりsuccessful no-opと表示し、配置成功と誤認させない。retryは全targetに対する新previewから開始し、失敗targetだけを勝手に再試行しない（MUST NOT）。結果不明ならautomatic retryをせずactual artifact/destination確認へ誘導する。
Publish-onlyはcurrent YAMLをparse/hash/validate/compareせず、source freshnessを常に「この操作では未確認」と表示する（MUST）。receiptからBuild時Profileを推測せず、過去sessionのProfileをcurrent artifactのprovenanceとして表示しない。Unity compile / 動作確認の成功を含意しない。

### GUI-DELIVERY-004

Build and Publishは最初に「保存済みinputでBuildし、成功artifactのPublish previewへ進む」操作であることを明示しなければならない（MUST）。Build成功後にPUBLISH-PREVIEWで対象と影響を確認してからPublishを開始する。二段階のGUI導線であり、単独Buildの自動Publishではない。
Build失敗ならPublish未実行。Build成功後にpreview/Publishが失敗またはCancelでもsuccessful canonical setをrollbackしない（MUST）。結果はBuild success / Publish not-run-or-failedを分ける。
Build開始からpreviewまでにsaved config/targetが変われば変更を表示し、新しいpreviewの確認を要求する（MUST）。同じapp内の別Buildでartifactが入れ替わった場合も古いBuild結果をそのままPublish対象にしない。

### GUI-DELIVERY-005

同一Projectの同一app sessionではBuild、Publish、Migration Apply、config Saveを同時実行せず、理由付きbusy stateで二重開始を防がなければならない（MUST）。通常source SaveはBuildPlan確定まで待機し、その後は可能とする。待機中actionを暗黙queueして後でmutationせず、利用者の再操作を要する。
buffer編集、read-only navigation、Diff、Problemsは継続可能とする。Project切替 / closeは実行中mutationが確定するまで通常操作として完了させない（MUST）。OS強制終了へのatomicityは保証しない。
Migration Recovery Requiredではconfig/source mutationとBuild / Build and Publishを停止する。Publish-onlyはsourceに依存しないため、configをloadできreceiptとpreflightがvalidなら既存契約に従って利用可能とする（MUST）。source recoveryが完了したという表示にはしない。

### GUI-DELIVERY-006

Problemsはcurrent buffer、saved validation、Build resultをsnapshotとProfile付きで区別しなければならない（MUST）。古いdiagnosticをcurrentへ置換しない。sourceにmapできない問題も保持する。
operation開始・完了、partial failure、not_attempted、dirty-input除外、disabled reasonは文字とassistive semanticsで提示し、keyboardでaction、preview、result、source navigationへ到達できなければならない（MUST）。Confirm / Cancel後のfocusは開始actionへ戻す。progressの細かな段階数やpercentを推測しない。

## Changed Requirements

- `GUI-SHELL-PROJECT-001`: Openに加えGUI-PROJECT-001のCreate入口を追加。Open failure時の既存workspace保持を維持。
- `GUI-SHELL-LAYOUT-002`: Overview / Settings / Deliveryへのcommand導線を追加。Explorerをdomain treeへ置換しない。
- `GUI-SHELL-CAPABILITY-001`: 0017のconfig recovery gate、GUI-DELIVERY-005のbusy / recovery compositionを参照。
- `GUI-SHELL-BUILD-001` / `GUI-DATA-BUILD-001/002`: saved-only契約を維持し、explicit Profileとdirty config表示をBUILD-REQUESTへ参照。
- Build pipelineへrequest capture / preview familyを追加するが、CLIの直接Publishにinteractive previewを要求しない。GUI previewはapplication serviceで実現する。

## Compatibility Impact

source format、Table/key identity、generated C# / binary / receipt形、CLI grammarを変更しない。
GUIだけのConfirmをCLI automationへ転用しない。Publishのall-target / partial-failure / source freshness非検証を維持。
GUI Createは安全なempty/new directoryに絞るがCLI initの既存supportを削除しない。

## Implementation Impact

- 現行`NativeApplicationService`にはinit、prepare_build_with_selection、receipt validation、preflight_publish、publish、build_and_publishがある。full BuildのProfile指定とcaptured snapshot / review identityは接続を確認・拡張する必要がある。
- [publish service](../../crates/masterdata-app/src/publish.rs)はstructured per-target reportを持つ。GUI用preview DTOは内部planをそのままpublic protocolに固定せず、必要なsafe informationを公開する。
- Tauri / Reactのsurfaceとstate管理を追加する。.NET invocationは`masterdata-dotnet`、generationは既存codegenのまま。
- feature完成時はfocused core/app/adapter/React tests、repository checks、Desktopの実機制作scenarioを検証する。implementationはこのproposalで開始しない。

### Acceptance

| Family | 必須evidence |
| --- | --- |
| PROJECT-INIT | empty/new success、non-empty/hidden/symlink拒否、raceで既存file不変、途中failureと作成済みentry報告 |
| GUI-PROJECT | dirty Cancelで作成なし、失敗で旧Project保持、成功後guided actions、recent removeでdisk不変 |
| BUILD-REQUEST | profile-separate同PK、unknown profile、dirty YAML/config除外、読込中source変更、captured後変更を混ぜない |
| PUBLISH-PREVIEW | artifact/config/manifest変更で無mutation、unmanaged collision、対象集合の変更、preview自体無mutation |
| GUI-DELIVERY | 0 target、全target success、preflight全not_attempted、target部分失敗、unknown result、Build成功後Cancel / Publish failure |
| operation合成 | repeated click、busy中config Save、Migration recoveryでBuild不可/Publish-only可、Problems source移動のsnapshot不一致 |

### P1–P3統合完了条件

新規Projectで型・Table・Dataを作成し、complex initializerとrecordを入力し、scalar paste→Undo→Redo→Save、
Tag / Profile / target設定、OverviewとProblemsによる確認、明示ProfileのBuild、Unity向けC# / binary Publishまで実行する。
raw YAML/TOML手編集やGUIからのCLI subprocessを通常経路の前提にしない。
error-path fixtureではexternal conflict、stale preview、Save All部分失敗、Recovery Required、Publish部分失敗で入力・unmanaged fileを保つ。

性能はP1–P3で**測定evidenceを残す**。10万record（分割file）、20列Table、1万cell pasteの固定生成inputをtemporary workspaceへ置き、
hardware / OS / build mode / dependency versionを記録してload、query、preview、validation時間とpeak memoryを測る。
productのms保証や製品上限は本packageで設けない。処理中にprogressを認識でき、古い応答が入力を消さず、keyboardでread-only navigationできることはGUI契約として検証する。
benchmarkを理由にfixture本体を書き換えない。細かな内部task分解やcomponent選定は実装側へ委ねる。

## Potential ADRs

None identified for P1–P3. shared Rust、host capability、.NET委譲を維持する。P4–P6のarchitecture選択は本承認へ含めない。

## Open Questions

None identified for this proposed scope. 今回具体化したcodec、query、history、config保存、Create、preview/conflict/busy behaviorは0016–0018全体への明示Approval対象。
P4 key/source移動、P5 expression、P6 Reference/Webは別package。benchmarkは測定必須であり未決latency目標を実装gateへ残さない。

## Canonical適用手順

1. Humanが0016–0018を一括Approveしたことを記録する。
2. 新規domain/GUI ownerをrepository templateに従って作り、各proposalのrequirement本文を一つのownerへ移す。変更IDは既存ownerへdelta適用する。
3. Source Edit / Record Mutationのscope note、Data Editorのrange/history非目標、shellのdirty/recovery合成も同じ変更で更新し、矛盾を残さない。
4. spec/GUI indexを更新、proposalをApplied audit recordへ変更。RFCはrationaleとして残し、requirementの二重ownerにしない。
5. spec / state checkを通し、未解決Gapがなければimplementation-readyとする。本格実装は別authorization boundary。

## Review

0016–0018全体を`review-spec`のchecklistで別passとしてself-reviewした。独立agent reviewではない。
sourceは今回の設計依頼、方向選択の「進める」、freshなApproved specification / ADR / relevant service / tests。

### Blocking Issues

None identified.

refinementへ戻して以下を修正し、再reviewした。

- `CONFIG-EDIT-003`: rename/deleteがないのにgrammar-invalidな新Profile名を保存できるとGUIだけで修復できない。新identityのvalid grammar/collisionをcreation preconditionへ分離した。
- `AUTHORING-BATCH-004`: floatの整数相当値をcopyしてinteger tokenへ変えるとpaste後にcategoryが変わる。float categoryを保つ表現を明記した。
- `AUTHORING-QUERY-002`: Required nullのis-null/is-invalid一致、query制約のAND、validity未確定時の扱いを明記した。
- `CONFIG-EDIT-003`: invalid configを保存した後、通常Project解決の失敗で設定editorも閉じると修復経路がなくなる。exact root/config bindingによる安全な再編集経路を明記した。

### Non-blocking Issues

None identified. 性能は測定evidenceを要求するが、未測定のlatency保証をcompletion gateにしない。

### Questions

None identified within P1–P3. 詳細案を承認するかはHumanのApproval actionであり、仕様内の未回答alternativeではない。

### Approved as Proposed

**Yes.** P1–P3はobservable behavior、失敗時の境界、canonical適用先、受け入れscenarioが揃い、一括のHuman considerationへ進める。
これはreview recommendationだけであり、3 documentは`Status: Proposed`のまま。人間の明示Approvalとcanonical適用が必要。

| Review axis | Verdict |
| --- | --- |
| Intent fidelity | Human選択のP1–P3、file単位、保存前Undo、scalar bulkを反映。細部は依頼された創造的仕様化の未承認案として明示 |
| Internal consistency | bulkとSave、configとYAML、履歴とUndo Delete、BuildとPublishのstate合成を定義 |
| Cross-spec consistency | source-preserving/validation非blocking、Migration initializer gate、receiptの非freshness、all-target / partial failureを維持 |
| Terminology consistency | field key、PK、occurrence、snapshot、metadataを区別。新しい永続identityなし |
| Normative strength | 規範文は採用後contract案。今回の方向選択を詳細Approvalへ昇格していない |
| Testability | 44の新IDとchanged IDにcodec・invalid・stale・I/O・keyboard・統合scenarioを対応 |
| Backward compatibility | source/config shape・binary・receipt・CLI grammar維持。GUI追加の制約をCLIへ転用しない |
| Unresolved ambiguity | P1–P3内の未回答alternativeなし。後段Reference / expression / Webは明示scope外 |
| Implementation leakage | DTO/library/algorithmを固定せずshared Rustとnative .NET責務を維持 |
| Unrequested behavior | P4–P6、Git自動実行、raw editor、任意コード実行を混入しない |

implementation diffはないためlocal rationaleの変更reviewはNot applicable。既存code/testsは影響調査のevidenceであり未承認behaviorを追認する根拠にはしていない。

## 承認記録（Approval Record）

未承認。`Status: Proposed`。RFCの方向選択と本packageのApprovalを混同しない。
