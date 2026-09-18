# Desktop制作v1 Candidate verification

確認日: 2026-09-18（JST）

## Scope

対象Candidate: `43c99fc5884f908d0061c2b615d0f0f968c82aca`。Base: `48d139e`。開始時remote HEAD: `f262471d58b9e38429e637ba23502241bc83fdb7`。

[Current Objective](../current-objective.md)のP1–P3 completion boundaryと[Approved canonical inputs](../specs/desktop-authoring-v1/README.md)に対し、core/application/GUI/Tauriを別passでレビューした。過去の設計意図やimplementation commitの自己申告を合格根拠にしていない。今回はreviewのみで、以下の実装修正は行っていない。

## Specification Conformance

Fail。shared Rust applicationを経由する基本構成は維持されているが、保存・Publishのdata safety、設定buffer、clipboard、query、typed initializer、operation lifecycleでApproved contractとの不一致がある。

## Findings

以下の番号はこのreview内のfinding参照であり、Requirement IDではない。P1はdata loss/誤mutation等の優先修正、P2はその他の必須仕様違反を表す。いずれもBlockingで、Approved範囲のcorrectionで解消できる。

### F01 — P1: Publishの配布先変更で未確認の削除を実行する

場所: `crates/masterdata-app/src/delivery.rs:107–111`。

previewが保持するidentityはartifact/configだけ。Confirm時の再preflight結果を以前の管理file集合・replacement/removal計画と比較せず実行する。PUBLISH-PREVIEW-002違反。

実行再現: delivery directoryがない時点でpreview（Item.g.cs追加、削除なし）を取得。その後delivery/External.g.csと、それを管理対象にするvalid manifestを外部作成。同じartifact/config tokenでConfirmするとSuccessとなり、previewに存在しなかったExternal.g.csを削除した。staleで無mutationを期待するassertionは失敗。

修正条件: destination ownership/contentと表示planをexact identityでbindingし、変更時は新previewを要求する。manifest変更・binary replacement変更の無mutation regressionを追加する。

### F02 — P1: TOML内の文字列をProfileとして誤編集する

場所: `crates/masterdata-core/src/config_edit.rs:186–205,287–292`。

`sections()`はmultiline stringの状態を追わず、文字列内のheader風の行を本物のtableと認識する。CONFIG-EDIT-002/003違反。次のvalid TOMLにprodのinclude_tagsをupdatedへ変更すると、unknown.text内のdecoyだけがupdatedになり、本物のrealは変わらないことを実行確認した。

```toml
[unknown]
text = """
[build.profiles.prod]
include_tags = ["decoy"]
"""
[build.profiles.prod]
include_tags = ["real"]
```

修正条件: parsed TOML structureとexact source spanを一意に結び付ける。安全に定位できない場合は無変更で停止する。

### F03 — P1: Profile配列編集でcommentを失う

場所: `crates/masterdata-core/src/config_edit.rs:443–486,545–556`。

`include_tags = [\n  "a", # keep a\n  "b", # keep b\n]\n`を同じa,bへ更新してもchanged=trueとなり、両commentが消えることを実行確認した。CONFIG-EDIT-002違反。`["a" # keep a\n]`への要素追加ではseparator配置のためparse failureになる例もある。

修正条件: element後commentとtriviaを保持し、semantic no-opは元bytesを返す。multiline/inlineのcomment regressionを追加する。

### F04 — P2: 禁止されたconfig OverwriteがAPIに残る

場所: `crates/masterdata-app/src/config.rs:175–179`、`apps/gui/src-tauri/src/lib.rs:356,372`。

old baseでconfigを開き、外部からcommentを追記後、現在identityを`overwrite_expected_identity`として渡すとSuccessになり外部commentが消えることを実行確認した。GUIは現状nullを渡すが、公開Tauri/application経路は実際に上書きを受理する。CONFIG-EDIT-004はv1のOverwriteを禁止している。

修正条件: source Saveから流用したOverwrite経路をconfig contractから取り除き、base conflictを常にConflictとして返す。

### F05 — P1: 不正configのtarget一覧で編集対象がずれる

場所: `crates/masterdata-app/src/config.rs:335–352`、`apps/gui/src/ProjectSurfaces.tsx:462,493`。

raw settingsはunknown kindのtargetをfilter_mapで落とし、GUIは圧縮後indexをsource occurrenceとして送信する。targets[0]=unsupported/first、targets[1]=csharp/secondの場合、表示されたsecondを編集するとfirstのpathが変わりsecondは不変となることを実行確認した。CONFIG-EDIT-001/003、GUI-SETTINGS-003違反。

修正条件: 元のoccurrence identityをDTOへ保持し、unsupported entryもlocation/reason付きreadonlyで表示する。

### F06 — P1: Settingsの連続編集・Reloadで入力を失う

場所: `apps/gui/src/ProjectSurfaces.tsx:381–415,438–443,471`。

Profile編集をpreview後、targetを編集・preview・Saveすると、各previewは保存時baseから一つのrequestだけを適用するためProfile変更が消える。Save後はconfig全体をclean扱いする。Profile選択変更は入力を上書きし、Reload Settingsもdirty確認なしにbufferを破棄する。GUI-SETTINGS-001/002違反（静的制御フロー確認）。

修正条件: file単位bufferへ変更を合成し、selection/navigationで保持する。Discard/Reloadは明示guardを通す。

### F07 — P1: SettingsでCmd/Ctrl+Sが別のYAMLを保存する

場所: `apps/gui/src/App.tsx:1188–1193`、`apps/gui/src/ProjectSurfaces.tsx:426–427`。

Settings表示中もglobal shortcutはactivePathのYAML Saveを呼ぶ。config formを編集しただけの状態ではSave AllもpendingRequest/preview不在でfalseを返し、active textを確定しない。GUI-SETTINGS-001違反。

修正条件: active surfaceにSaveをdispatchし、configのform確定→preview→Saveを同じ経路にする。別YAMLが不意に保存されない回帰テストを追加する。

### F08 — P2: Config保存後のbindingと一覧を更新しない

場所: `apps/gui/src/ProjectSurfaces.tsx:438–443`、`apps/gui/src/App.tsx:1133–1145,1553–1560`。

Save成功は子componentのsnapshotとdirtyだけを更新し、親workspace/profile一覧/Overview previewをinvalidateしない。新ProfileはDeliveryの選択肢に現れず、Save Allもconfig保存後のbinding再解決をせずYAML保存へ進む。GUI-SETTINGS-002/003違反。

修正条件: config保存後の再解決とsnapshot invalidationを実施し、identity/roots変化はdirty guardへ戻す。invalid configでYAMLを続行しない。

### F09 — P1: Pasteがclipboardの行列を別形状へ書き込む

場所: `apps/gui/src/App.tsx:2171–2183`、`crates/masterdata-app/src/batch.rs:261–275`。

Pasteにも現在のselectedTargetsを渡し、shared側はcell総数だけ比較してflattenする。単一active cellから2×2貼り付けは失敗し、1×4選択へ2×2貼り付けは横4cellへ誤配置される。GUI-GRID-001違反。

修正条件: active-cell top-leftとclipboard寸法から対象を決め、行列形状・範囲外・readonly cellをshared layerで検査する。Fillとの意味を区別する。

### F10 — P2: Typed Migration Initializerが未実装

場所: `apps/gui/src/TableEditor.tsx:79–83`、`apps/gui/src/TypeEditor.tsx:116–120`。

AddField/AddCustomFieldは今もInitializer (JSON value) textareaを要求する。型/modifier変更でも旧initializerをunsetへ戻さない。GUI-TABLE-INT-008 / GUI-TYPE-INT-010違反。今回Candidateで両ファイルの変更がなく、既存raw入力が残っていることを確認した。

修正条件: shared resolved modelからtyped controlを生成し、unset/explicit null、nested exact 64-bit、型変更の失効を検証する。

### F11 — P2: is-invalidがvalid complex値を誤判定する

場所: `crates/masterdata-core/src/authoring_query.rs:359–366`。

valid Flags値[Read]、Customのnullable int memberがnullの値にis-invalid queryを実行すると、いずれも一致することを実行確認した。Flagsが無条件Invalid、Customはnested NullをValidとして許容しない。AUTHORING-QUERY-002のshared validationとの一致に違反する。

修正条件: query用の簡略validity実装をdomain validatorと整合させ、Flags/nullable nested/array等のvalid-invalidを回帰検証する。

### F12 — P2: Nullable scalarのsortを拒否する

場所: `crates/masterdata-core/src/authoring_query.rs:417–419`。

nullable intへのsortがE-AUTHORING-QUERY-SORT-UNSUPPORTEDになることを実行確認。Required限定はAUTHORING-QUERY-003のint/string/VOとvalid→null→invalidの契約にない。

修正条件: supported scalarのnullableを受理し、両方向とstable orderingを検証する。

### F13 — P1: Buildのconfig/source captureを一つのsnapshotとして確定しない

場所: `crates/masterdata-app/src/lib.rs:309–317`、`crates/masterdata-core/src/application.rs:56–64`、`crates/masterdata-core/src/project.rs:238–244`。

profileを最初のProject discoveryから取り、BuildPlan作成でProjectを再discoverする。source読込は列挙後一度ずつ読むだけで、config/source membership/bytesの読込前後再検査がない。途中の外部変更で古いProfileと新しいconfig/sourceが混在し得る。BUILD-REQUEST-001違反（静的確認、raceの実行再現は未実施）。

修正条件: captureしたconfigとselectionを同じPlanへ渡し、読込中のidentity/membership変更はpublication前に失敗させる。制御可能なread hookによるrace testを追加する。

### F14 — P2: Build失敗後にも古い成功を表示する

場所: `apps/gui/src/ProjectSurfaces.tsx:550–560,611`。

Build成功後に次のBuildが失敗してもbuildをclearせず、buildStateによらずBuild succeededを表示する。GUI-DELIVERY-002違反。

修正条件: captured operationごとに結果を分離し、前回成功を残す場合は過去の結果と明示する。

### F15 — P1: Publish部分失敗の結果を捨て、旧previewで再試行できる

場所: `apps/gui/src-tauri/src/lib.rs:437`、`apps/gui/src/ProjectSurfaces.tsx:578–593,612`。

adapterはPublishExecutionFailure.reportを捨て、GUIは成功reportも保存しない。target別success/failed/not_attemptedを表示できない。失敗後もpreviewを保持しConfirmが再び有効になる。GUI-DELIVERY-003/006違反。

修正条件: structured report/unknown outcomeをsurfaceまで保持し、失敗後は旧Confirmを失効させ全targetの新previewを要求する。

### F16 — P2: Build/Publishのbusy・Recovery契約がsurface間で不一致

場所: `apps/gui/src-tauri/src/lib.rs:541–544,599–615`、`apps/gui/src/App.tsx:597,1148–1183,1563–1573`、`apps/gui/src/ProjectSurfaces.tsx:578–579,612`。

Build全体でoperation guard/table session mutexを保持し、Plan確定後もsource Saveが拒否される。一方Deliveryのrunningは親のproject switching/close guardへ共有されない。またRecovery中のmutationBlockedがConfirm Publishにも適用され、receipt-valid Publish-onlyまで禁止する。GUI-DELIVERY-005違反。

修正条件: project operation stateを共有し、Build capture phaseと実行phaseを分ける。切替/closeはoperation確定待ちとし、Recovery時も独立したPublish-only eligibilityを使う。

### F17 — P2: Undo履歴を予告なく破棄する

場所: `apps/gui/src/App.tsx:445,1001`。

履歴をslice(-50)で自動削除し、事前通知がない。GUI-GRID-005違反。50回を超える編集で古い変更へUndoできなくなる。

修正条件: current bufferを保持しつつ、履歴廃棄前の通知と明示的な扱いを実装し、境界を検証する。

## Tests and Regression Evidence

- Candidate diffには新規GUI flowのtest変更がない。既存49件のGUI component testsだけではSettings/Delivery/Overview/gridの新契約を保護できない。
- local `cargo xtask check-all`の最終結果は下記実行結果欄に記録する。
- applicationの一時integration repro: stale Publish plan、config Overwrite、invalid target occurrenceの3件で期待する安全性assertionが失敗し、上記誤mutationを確認した。テスト用一時fileは削除済み。製品code変更はない。
- coreの一時standalone Rust repro: multiline string誤編集、comment loss、Flags/nullable Custom invalid誤判定、nullable sort拒否を確認した。current rlibにlinkして実行し、repository外へ置いた。
- [Candidateを含むCI run](https://github.com/kamahir0/masterdata/actions/runs/35301415510)はmacOS/Windowsともfailure。macOSはauthoring.test.tsxの7件、WindowsはMigration recoveryの1件でtimeout。flakyかregressionかは未確定で、単純なtimeout延長を解決とはしない。
- Evidence Gap（Blocking）: Current Objectiveが要求するDesktop実機でのProject作成→編集→設定→Build→Publishとfailure recoveryの完走証拠を確認できない。本reviewでも実機scenarioは未実施。correction後に新規flowのfocused testsと実機scenarioを揃える必要がある。

## Rationale Freshness

- Stale: Tauri Buildのsource race防止を理由とする全実行期間のguardは、GUI-DELIVERY-005のPlan確定後Save許可を満たさない。F16でlifetimeとrationaleを一緒に修正する。
- Freshだが不十分: delivery.rsのartifact/config identity再検査という説明自体は実装に一致するが、destination plan identity不足（F01）を安全とみなす根拠にはならない。
- Evidence Gap: reference check成功は新規GUI操作やconfig source preservationの意味的正しさを証明しない。上記再現を固定regressionへ移す。

## Evidence Integrity

- Requirement references: canonicalは475件。参照構造checkは通過。上記の実装不一致は参照の存在とは別問題。
- ADR/RFC references: shared core/applicationと.NET delegationの構成を確認。adapterがCLI subprocessへdomain処理を迂回する変更は確認していない。
- Regression test references: 既存test通過だけで新規flowを証明していない。上記の再現条件をcorrectionのtest inputとする。
- Benchmark/external references: [固定測定](desktop-v1-performance.md)とharnessは存在する。debug測定でquery約19.4秒、1万cell preview約43.2秒、peak約2.91 GiB。これはSLA違反とは断定しないが、日常制作の快適さを示す値ではない。release/Desktop実操作の応答性は別途確認が必要。本reviewでは性能測定を再実行していない。

## Architecture

Rust shared boundary/.NET delegationの基本方針は維持されている。ただしqueryの独自validity判定（F11）とsettingsのfrontend occurrence identity再構成（F05）がcanonical semanticsからずれている。sharedに置いたという構造だけで安全性は成立しない。

## Verdict

**Ready to merge: No**（既にmainにあるため、ここでは完成判定・次段階への進行可否を意味する）。

P1–P3を完了と評価できない。Stageは`correction-ready`、Candidateはreview対象SHAを維持する。次のimplementation passはF01–F17と不足検証に限定し、Approved specの再設計や新機能追加へ拡張しない。

## 今回の実行結果

- `cargo xtask check-all`: PASS（exit 0）。仕様・rationale・format・clippy・Rust workspace tests・frontend lint/test/build・Tauri tests・MasterMemory spike・.NET integrationまで完走。
- GUI component tests: 49 PASS。Tauri adapter tests: 8 PASS。
- review文書/state追加後の`cargo xtask check-specs`: PASS（435 relative links）。`cargo test -p xtask --test execution_state`: 3 PASS。`git diff --check`: PASS。
- 非Blockingの観測: frontend bundle size warningと.NET MessagePack 3.1.3のNU1902/NU1903 dependency audit warningが出た。このCandidateが導入したdependency変更ではなく、advisory適用条件の調査は今回未実施。全check PASSを「警告なし」や依存の安全性確認済みという意味にはしない。
- local PASSとremote CI timeout failureの両方を記録する。remote failureの原因解決、新規操作の回帰検証、Desktop実機scenario完了は未達。
