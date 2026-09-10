# RFC: 最初のGUIレコード編集・保存体験

Status: Draft

## 背景（Context）

YAMLを手書きせずにマスターデータを編集し、Gitで変更を確認できることは、
[Product vision](../product/vision.md)のauthoring systemとしての中心的な価値である。
Build / PublishやMigration内部処理を増やすだけでは、この体験は完成しない。

2026-09-10のHumanとの議論では、AddField source commit safetyのfinal verification後に、
「GUIでレコードを編集・保存し、差分と検証結果を確認できる体験」を次priorityとして具体化することが選択された。
続く比較に対してHumanは、初期編集範囲はRequired Primitiveの非key field、保存単位はVS Codeのようなfile単位、
validation結果は保存可否を妨げない方針を選択した。
さらにdirty中Build、external modification、dirty bufferを失うnavigation lifecycleについてもVS Code寄りの明示的なbehaviorを選択した。

これらはproduct choiceとして本RFCに記録する。canonical GUI / source-edit contractの詳細、Human Approval、
implementation authorityをこのRFC単体で与えるものではない。

## 根拠と分類（Source Evidence and Classification）

| 分類 | 内容 |
| --- | --- |
| Decision | 次priorityを既存recordのGUI編集・保存体験の具体化へ進める。 |
| Decision | 初期編集対象はRequired Primitiveの非key fieldとし、その他のfieldは初期版ではread-onlyとする。 |
| Decision | dirty / Saveの単位はsource data fileとし、VS Codeに近い明示Save体験を採る。 |
| Decision | validation resultは表示するが、validation errorの有無をSave可否のgateにしない。 |
| Decision | dirty fileがあってもBuildできるが、Buildは保存済みsourceだけを使い暗黙Saveしない。 |
| Decision | clean fileの外部変更は自動Reloadし、dirty fileの外部変更はconflictとしてCompare / Reload / Overwriteを明示選択する。 |
| Decision | file / record / Table navigationではdirty bufferを保持し、Project切替・Reload・window close等の破棄を伴う操作だけSave All / Don't Save / Cancelを提示する。 |
| Requirement | GUIから値を変更・保存し、差分と検証結果を確認できるようにする。exact behaviorはcanonical仕様化時に確定する。 |
| Constraint | YAML正本、shared semantics、Tauriからapplication serviceへの委譲は既存Approved authorityに従う。 |
| Open Question | source preservation、exact scalar transport、I/O failure等の詳細contract。 |

## 課題（Problem）

既存GUI shellはproject情報、Validate、Buildを提供するが、テーブル編集を所有するApproved GUI仕様はまだない。
[GUI app shell](../gui/app-shell.md)のproject picker、unsaved changes、file watcher、
[YAML subset](../specs/yaml-subset.md)のGUI save preservationは未決定である。

Migrationはschema変更の意味を所有する。通常のrecord値編集をAddFieldやMigration Commandへ押し込まない。
そのsnapshot / patch / commit処理を内部で再利用できるかは、record保存のcontractを決めた後に評価する。

## 目標（Goals）

最初の到達点は次の一連の操作を目指す。

1. Desktopで既存Projectを選んで開く。
2. logical Tableを選び、複数data fileに分かれた既存recordsを確認する。
3. recordを選び、対応するRequired Primitiveの非key fieldを変更する。
4. source差分とvalidation resultを確認する。
5. dirtyなsource data fileを明示Saveする。
6. 開き直して変更を確認でき、Git diffでも意図した変更を追える。

既存のValidate / Buildへ接続できることを保ち、Unityへの最終反映は後続の製品受け入れ候補とする。

## 非目標（Non-Goals）

以下は初期版から外す。将来要望の撤回ではない。

- recordの追加・削除、schema編集、RenameField / DropField、MasterReference設計。
- Enum / Value Object / Nullable / Array / Custom Typeの編集、Primary / Secondary Key構成fieldの編集。
- spreadsheet同等のcell range操作、一括paste、履歴を持つUndo/Redo。
- Build Profileの設定・選択、タグ編集、GUI Publish、Git commit/push操作。
- Standalone / Connected Web、Native Host、installer、全hostの同時完成。
- formatter、全ファイルの再serialize、generated C# / binaryの編集。

## 選択された方針（Selected Product Choices）

### A: 最初に編集できる範囲

Required Primitiveの非key fieldだけを編集可能とする。
Primitiveは[Primitive Types](../specs/type-system/primitives.md)の全8種類。
Primary / Secondary Keyの構成field、およびEnum / Value Object / Nullable / Array / Custom Typeは初期版ではread-onlyとする。
複合型を持つTable全体を隠さず、対応fieldだけを編集可能にする。

数値の値域は既存ownerに従う。特にlong / ulongをfrontendの数値変換で丸めることは許されず、
exact representationはshared boundaryで仕様化する。

### B: file単位のdirty / Save

Save単位はrecordやTableではなく、source data fileとする。
GUI上ではVS Codeに近い形でfileごとにdirty stateを持ち、利用者が明示Saveする。
同じfileに属する複数record / fieldの変更は、そのfileの1回のSaveでまとめて永続化される。
別fileのdirty stateは独立して保持する。

source diffはSave可否のgateではなく、変更内容を確認するsurfaceとして提供する。
Gitの有無に依存せず、UI内で保存予定または未保存のsource差分を確認できることを目指す。

未変更fileはbyte-for-byte保持する。変更fileでもrecord順、member順、コメント、改行、indentation等の
無関係なtextをできる限り保持し、変更valueのquote/styleは意味を正しく表すために必要な範囲だけ変更する。
source locationを安全に特定できない場合のbehaviorはcanonical source-edit仕様で決める。

### C: validationはSaveを妨げない

validation resultは編集体験の重要なfeedbackとして表示するが、validation errorの有無をSave可否のgateにしない。
編集値がdomain validation上invalidでも、保存操作そのものを禁止しない。
既存project errorもSave禁止条件にはしない。

このdecisionは「writeが必ず成功する」という意味ではない。filesystem I/O failure、permission、path safety、
external modificationとの競合など、永続化を安全に完了できない物理的・整合性上のケースはsource-edit contractで扱う。
validationと保存成功／失敗を混同しない。

### D: dirty中のBuild

source data fileがdirtyでもBuildの開始を禁止しない。Buildはdisk上の保存済みsourceだけを入力とし、dirty bufferを暗黙に含めない。
Buildを理由にSaveまたはSave Allを暗黙実行せず、dirty fileがある場合は未保存変更がBuildへ含まれないことを利用者が認識できる表示を行う。

### E: external modification

cleanなfileが外部変更された場合はdisk上の最新内容へ自動Reloadする。dirtyなfileが外部変更された場合はlocal bufferを保持したままconflict状態にし、通常Saveで暗黙上書きしない。
conflict recoveryでは少なくともCompare / Reload / Overwriteを明示選択できるようにする。

### F: dirty bufferを失う操作

file / record / Table間のnavigationだけでは確認を出さず、複数fileのdirty bufferを独立して保持する。
Project切替、Project Reload、window close等、現在保持しているdirty bufferを失う操作では `Save All` / `Don't Save` / `Cancel` を提示する。
Save Allに失敗またはconflictがある場合は元の破棄操作を完了せず、dirty bufferを保持する。

## トレードオフ（Trade-offs）

Required Primitiveに範囲を絞ることで、型editorの拡張より先に「編集 → 差分確認 → file Save → reopen」という
利用者価値を閉じられる。一方で初期版の編集可能範囲は狭いので、非対応fieldをread-onlyとして明示する必要がある。

file単位SaveはYAMLというsource正本と外部editorのmental modelに一致しやすいが、record単位Saveよりdirty管理と
複数record変更の表示が重要になる。厳密なsource preservationはformatterより実装費用がかかるが、YAML + Gitと外部編集との共存に直結する。

validation non-blockingにより、GUIはvalidatorではなくeditorとしてsourceを保存できる。一方、invalid sourceを保存可能にするため、
validation feedbackを見失わないUIと、保存済みsource / 編集中bufferの状態区別が必要になる。

## 提案（Proposal）

選択済みproduct choicesを次のownerへ分けてDraft化する。

- `docs/gui/`のrecord editor仕様: navigation、file selection、dirty表示、Save、差分表示、validation表示、focus、keyboard、error state。
- `docs/specs/`のsource-edit仕様: record値変更、snapshotとsource provenance、file単位commit、保存結果、preservation、external change / I/O failure。
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

OQ-A〜Cおよびdirty lifecycle / external modification / Buildのproduct choiceはHuman decisionで解決済み。
以下はcanonical Draftで閉じる。既存Approved authorityからrecoverできる事項はHumanへ逐次質問せず整理する。

- project pickerの入力方法、開けないproject、empty table、schemaをresolveできない状態の操作。
- record表示順、重複PKを含むsource recordの識別・表示、非対応値・literal block stringの表示範囲。
- exact scalar transport、文字列・数値の入力中状態、変更を元の値へ戻した場合のfile dirty判定。
- file単位Saveのsource provenance、preservation、atomicity、通常I/O failureの結果。
- 差分の表示単位、保存結果が不明な場合の再試行、reload後の入力復元の範囲。
- loading/saving中の操作、keyboard/focus、accessibility、shortcut、product textの言語。

## 受け入れ候補（Acceptance and Implementation Impact）

仕様化後に次を実際のUI操作・shared service testsで確認する。source内の文字列検査だけで完了扱いにしない。

- project選択 → Table → record → 値変更 → source diff / validation確認 → file Save → reopenで変更を確認する。
- 同一file内の複数変更が一度のSaveで永続化され、別fileのdirty stateが独立する。
- split data files、同一PKの別source record、非対応field、64-bit整数の精度を確認する。
- domain validation上invalidな値でもSave操作が禁止されず、保存後もvalidation resultを確認できる。
- clean external editの自動Reloadと、dirty external editのconflict recoveryを確認する。
- dirty file間のnavigationではbufferを保持し、Project切替 / Reload / window closeではSave All / Don't Save / Cancelで保護する。
- dirty中Buildが保存済みsourceだけを使い、暗黙Saveしないことを確認する。
- I/O failure時にsourceと入力を承認済みcontractに従って扱う。
- 未変更text/fileを保持し、Saveだけでbuild/publishを開始しない。
- 既存CLIのvalidationとGUIの保存済みsource validationが同じdomain resultを返す。

影響候補は`masterdata-core`、`masterdata-app`、Tauri command、共有frontend、fixturesと操作test。
.NET bridgeやMasterMemory formatの変更は不要と見込む。library、CST、RPC schema、public commandはここで選択しない。

## Refinement report

### Affected Specifications

GUI app shell（Draft）、YAML subset（ApprovedのGUI save Open Question）、Table / KeyとRuntime hosts（Approved）。
既存normative requirementの変更は現時点で確定していない。

### Confirmed Decisions

- 初期編集対象はRequired Primitiveの非key field。
- Save / dirty管理はsource data file単位。
- validation errorはSave禁止条件にしない。
- dirty中でもBuild可能だが保存済みsourceだけを使い、暗黙Saveしない。
- clean external modificationは自動Reload、dirty external modificationはconflictとしてCompare / Reload / Overwriteを明示する。
- file / record / Table navigationではdirty bufferを保持し、破棄を伴うProject切替 / Reload / window closeでSave All / Don't Save / Cancelを提示する。

### New Requirements

canonical GUI / source-edit Draftでnormative requirementとIDを割り当てる。

### Changed Requirements

None identified。

### Open Questions

上記のsource preservation、I/O failure、exact representation等の詳細事項。

### Potential ADRs

既存shared boundaryで足りる見込み。source editing方式のarchitectural trade-offが必要なら別途検討する。

### Compatibility Impact

上記互換性節を参照。既存contract変更はcanonical reviewで確認する。

### Implementation Impact

上記受け入れ候補を参照。今回はruntime変更なし。

## レビュー（Review）

Human product decisionsは増えたが、canonical GUI / source-edit仕様はまだDraft化・review・Human Approvalされていない。
そのため本RFCだけを根拠にimplementation-readyとは判定しない。

### Blocking Issues

implementationに対しては、file単位Saveのsource preservation / atomicity、通常I/O failure、exact scalar transport等のcanonical仕様化とHuman Approvalが未完了。

### Non-blocking Issues

None identified。

### Questions

現在選択済みのproduct-level質問は解決済み。追加Human decisionが必要なobservable choiceを仕様refinementで発見した場合だけ提示する。

### Approved as Proposed

No。Human product decisionsは記録済みだが、RFCはimplementation authorityではない。

| 観点 | 評価 |
| --- | --- |
| Intent fidelity / Normative strength | Human decisionを記録し、未承認の詳細contractと区別している。 |
| Internal / Cross-spec consistency | shared semanticsとsource正本を参照し、Migrationを値編集のauthorityにしていない。 |
| Terminology / Backward compatibility | file単位SaveとVS Code寄りのdirty lifecycleをproduct choiceとして追加。既存formatやdomain identityは変更していない。 |
| Testability / Unresolved ambiguity | file Save、validation non-blocking、Build、external conflict、dirty buffer保護の受け入れ候補を追加し、詳細未決定をOpenとして保持。 |
| Implementation leakage / Unrequested behavior | library/APIを固定せず、未解決のsource preservation等を推測していない。 |

実装diffがないためimplementation rationale reviewは対象外。

## 決定（Decision）

2026-09-10にHumanが以下を選択した。

1. 初期編集対象はRequired Primitiveの非key field。
2. Save / dirty管理はVS Codeに近いsource data file単位。
3. validation resultはSave可否を妨げない。
4. dirty中でもBuild可能だが、保存済みsourceだけを使い暗黙Saveしない。
5. clean external modificationは自動Reloadし、dirty external modificationはconflictとしてCompare / Reload / Overwriteを明示選択する。
6. file / record / Table navigationではdirty bufferを保持し、Project切替 / Project Reload / window close等の破棄を伴う操作だけSave All / Don't Save / Cancelを提示する。

RFCはDraftのままとし、GUI / shared source-editのcanonical Draftへ具体化する。