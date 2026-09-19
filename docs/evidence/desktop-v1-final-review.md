# Desktop制作v1 Final Candidate verification

確認日: 2026-09-19（JST）

## Scope

対象Candidate: `80944c66cf9c79903243a6584509660fef39df20`。Correction base: `9448edc1d6942e39c83248781279b2753718345d`。

Development Stateのexact Candidateをfreshに読み直し、旧reviewのF01–F17、Current Objectiveのcompletion boundary、Approved canonical specifications、Candidate diff、focused regressions、CI #360を別passで確認した。implementation中の自己評価は合格根拠にしていない。

## Specification Conformance

F01–F17の実装上の不一致はCandidateで解消を確認した。

- Publish Confirmはartifact/configに加えdestination plan identityをbindingし、manifest / managed path / binary destination state変更をstaleとして無mutationで拒否する。
- TOML editはmultiline string内のheader誤認を避け、comment付きarrayのno-op / add / removeをsource-preservingに扱う。config Overwrite APIは除去され、publish targetのsource occurrenceをDTOで保持する。
- Project Settingsはfile-level bufferへ複数editを合成し、Reload/selection guard、active Save、保存後workspace rebindを行う。
- Pasteはclipboardの2次元shapeとactive cellからtargetを決め、shared layerでもtarget rectangle shapeを照合する。
- Table / Type migration initializerはshared resolved authoring shapeを使うtyped controlとなり、unset / explicit nullを分け、type/modifier変更でPlanを失効する。
- queryはFlags / nested nullable validityを修正し、nullable scalar sortでvalid -> null -> invalidを維持する。
- Buildは同一Project snapshotでprofile/config/sourceをcaptureし、capture中のconfig/source bytes/membership変更を拒否する。Build executionはcaptured Planから分離される。
- Build/Publish結果、busy state、Recovery時Publish-only、project switch/close guard、Undo history discard noticeをCandidateのshared/GUI経路で確認した。

したがって、旧F01–F17に対するSpecification Conformance自体はPass。

ただし、Current Objectiveのcompletion boundaryに必要なacceptance evidenceが下記2件不足しているため、objective completionはPassにできない。

## Tests and Regression Evidence

Candidateには次のfocused regressionが追加されている。

- Publish destination / binary replacement stale preview
- multiline TOML / comment preservation / config overwrite prohibition / target occurrence
- Settings buffer composition / navigation / Reload / active Save / Delivery failure result / Recovery Publish-only
- 2x2 Paste geometry / clipboard shape
- typed Table / Custom initializerとtype/modifier invalidation
- Flags / nullable nested validity、nullable scalar sort
- Build capture中のsource bytes / config変更
- Undo history discard notice
- Tauri command層でProject Create -> source Create -> record Save -> Settings -> Build -> stale Publish rejection -> fresh Publish success

Required checks: GitHub Actions CI #360
`https://github.com/kamahir0/masterdata/actions/runs/35423054840`

- Ubuntu: PASS
- macOS: PASS
- Windows: PASS
- `cargo xtask check-all`: 各required jobでPASS
- Ubuntu `cargo xtask check-wasm`: PASS

## Rationale Freshness

- Build capture/execution separation: Fresh。Plan capture完了後はsource Saveをlong-lived operation guardから外す実装とcommentが一致する。
- Publish destination identity: Fresh。artifact/configだけでは不足するfailure modeをdestination state hashで保護している。
- Config source preservation: Fresh。unsafe/ambiguous editをfail-closedにする方針と実装・regressionが一致する。
- Test-only source polling control: Fresh。production defaultは1600msを維持し、interaction testだけpoll競合を無効化する。

## Evidence Integrity

- Requirement references: Candidateのrequired checksでreference/spec checksがPASS。
- ADR/RFC references: architecture owner変更は確認されない。
- Regression test references: 旧F01–F17の主要再現条件はCandidateのfocused testsへ移されている。
- Benchmark/external references: `docs/evidence/desktop-v1-performance.md`は2026-09-18測定のままで、今回変更されたquery / batch implementationに対する再測定記録がない。
- Desktop scenario: Candidateの`desktop_workflow_reaches_build_publish_and_stale_recovery`はTauri command関数を直接呼ぶnative integration testであり、window / WebView / actual Desktop GUI操作を通す実機scenarioではない。

## Architecture

core / application / Tauri / frontendのowner分離は維持されている。新しいdomain semanticsをfrontendへ移した変更は確認しない。Publish/config/query/build correctionはshared Rust boundaryに置かれている。

## Findings

### V01 — Blocking Evidence Gap: Desktop実機制作scenarioが未証明

Current Objectiveは「focused tests、repository required checks、Desktop実機制作scenarioを通す」をcompletion boundaryとしている。

Candidateにはnative Tauri integration testが追加され、Project CreateからBuild/Publish failure recoveryまでapplication/adapter/filesystem経路を保護している。これは有効な回帰証拠だが、`#[cfg(test)]`内でcommand関数を直接呼んでおり、実際のDesktop window / GUI interaction / packaged runtimeを操作していない。

したがって、旧reviewで不足していた「Desktop実機でProject作成 -> 編集 -> 設定 -> Build -> Publishとfailure recoveryを完走した証拠」はまだ閉じていない。

解消条件: actual Desktop GUI/runtimeを通した制作scenarioを実行し、対象Candidate・環境・操作範囲・結果をevidenceとして記録する。自動E2Eで代替する場合も、Tauri command直接呼出しではなく実Desktop surfaceを通ること。

### V02 — Blocking Evidence Gap: 性能evidenceがCorrection Candidateに対してfreshでない

Current Objectiveは100,000 records / 20 columns / 10,000-cell paste固定inputについてload/query/preview/validation時間とpeak memoryをevidenceとして残すことをcompletion boundaryとしている。

既存`docs/evidence/desktop-v1-performance.md`は2026-09-18の測定を保持しているが、Correction Candidateでは`crates/masterdata-core/src/authoring_query.rs`と`crates/masterdata-app/src/batch.rs`を変更している。既存文書にはCandidate SHAもなく、変更後のquery / paste preview pathに対する再測定は記録されていない。

解消条件: fixed performance harnessをCandidate系のcorrection実装に対して再実行し、環境・exact SHA・load/query/10k-cell preview/validation/peak memoryをevidenceへ更新する。

### E01 — Non-blocking Evidence Gap: source membership raceの個別regression

Build snapshot実装はcapture後に`source_files()`を再列挙し、membership差分を`E-BUILD-SNAPSHOT-STALE`で拒否するため実装上の契約は確認できる。controlled hook testはsource bytes変更とconfig変更を固定しているが、file追加/削除によるmembership変更そのものの個別testはない。

現行実装は単純なpath list equalityで明示的に保護されており、これ単独ではmergeをblockしない。将来のsnapshot refactor時の退行防止としてfocused test追加が望ましい。

## Verdict

**Ready to merge: No**

旧F01–F17のcorrectness findingは解消し、CI #360も3 OSでgreen。ただしCurrent Objectiveのcompletion boundaryに対するV01/V02のBlocking Evidence Gapが残る。

次Stageは`correction-ready`。次passはV01/V02のevidence解消に限定し、Approved behaviorの再設計やunrelated refactorを行わない。
