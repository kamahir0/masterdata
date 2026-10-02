# Rewrite Contract Finalization — evidence / readiness

Status: Finalized contract / Not Ready evidence gate

## Canonical application / purification delta

0051のHuman D1〜D6は[resolved record](human-decisions.md)から[GUI baseline](../gui/rewrite-baseline.md)、app shell / Data Editor / Grid Authoring / Project Workflow、Authoring Query / Batch、YAML subset / Source Editへ適用した。独立Diff / Typed Filter / View Sort / Advanced Batch / standalone Overviewの必須surfaceを解除し、current legacy capabilityを即削除する要件にはしていない。

KEEP: compare safety、Search、lossless paste、physical composition / Build inclusion、source safety、recent direct / spatial UX、interactive read principles。REFINE: lifecycle / request / surfaceとshared semanticsの分離、independent oracle / hard work counts。DEMOTE: 中間insertion line、revoked Save案、旧Complex footer、initial scope、監査時点のD1〜D6未決定。remove from rewrite baseline: D1〜D5の5surface群。旧testは消さず[3 gate classes](test-purification.md#rewrite-gate-manifest)へ分離した。

## Independent oracle coverage

[portable v1](../../fixtures/rewrite-oracle/v1/README.md)の期待値はsource bytesとdomain intentから独立に記述し、legacy helperによる生成・DTOのcopyを行わない。core / app / CLI-.NET adapterは別owner。

- Byte17: current adapterで16pass。new-mapping-blockはD6 target-only gap（legacy writerはflowを生成）。greenなgap regressionをconformance passと数えない。
- Interpretation9: validityとlexical text / nullの意味をcurrent adapterで全case pass。
- Physical Save7: schema / selected records / both / inline / mixed / external Conflict / stale cached identityを全disk bytesとcommit範囲で検証しpass。
- Structural3: Rename / Add / Dropのmulti-source candidate exact bytes、Plan input不変、destructive分類がpass。
- Paste4: rectangular long / ulong boundary、text null、source-safe semantic invalidity、ragged / PK unsafe all-or-noneをcandidate exact bytesとno implicit Saveで検証しpass。one UndoのUI確認はworkflow oracleへ分離。
- Empty Table: schemaのみcontext、record source未作成、input bytes保持をpass。
- Fault3: migration Recovery Requiredのexact mixed disk、write gate、read navigationがpass。Unix single-source precommit Failureもexact OLD disk保持をpass。UnknownとUI draft/historyは未接続。
- Workflow23: UI / source lifetime / faults / migration / deliveryのportable event-state期待値。全caseの自動adapterが接続済みという主張ではない。
- capacity.json: 100k / 10files / 20columns / 10k pasteの数学的expected result。2026-10-03 isolated release runで全assertions pass。load85969 / query85964 / paste preview95861 / validation85814ms、process peak RSS2,407,645,184bytes、wall354.44s。compile時間を含めない。source0の全200,000 candidate cellsを期待式と比較し、non-target保持、10k targets、query0matches、diagnostics0、implicit Saveなしを検証。DOM / transport boundednessはこのnative one-shot測定だけでは証明しない。[raw capacity](../evidence/rewrite-finalization/macos-capacity.txt)。
- Consumer.cs: generated public API / actual binary load、PK / SK / reference / 64bit / nested valueを検証。static regexではない。

## Fresh native Tier1 baseline

2026-10-02、local main 0ea1d71 + finalization harness（uncommitted）、Apple M1 / arm64 / RAM16GiB / macOS26.6.2、rustc1.96.0、release。inputは3Tables / 8sources / 12krecords、selected2k×20。各warm case100 selections×3fresh session runs、alternate sourceを挟む。native + serializationであり、IPC / usable editorではない。OS filesystem cacheの消去はしていない。

| Case | median range ms | p95 range ms | max ms |
| --- | --- | --- | --- |
| revisit | 24.40–24.65 | 24.67–25.84 | 35.94 |
| same Table | 24.43–24.77 | 24.95–25.27 | 86.78 |
| cross Table | 22.67–22.90 | 22.84–23.23 | 27.59 |
| schema selection | 26.49–26.77 | 26.81–27.14 | 28.64 |

cold2320 / 2292 / 2355ms、first projection29.74 / 24.91 / 25.00msは各run単発でp95ではない。warmのproject-wide discovery / enumeration / parse / validationは全case0。局所identity I/O / projection / serializationは含む。raw distributionをevidenceへ保管する。nativeはHuman50ms target内だが、UI150ms達成の証拠ではない。

macOS actual Tauri WebViewは900 samples（各完了case100、run1の4case / run2の4case / run3 revisit）を取得した。raw metricsは [partial UI evidence](../evidence/rewrite-finalization/macos-ui-partial.json)。foreground / occlusion interruptionとcapture server errorにより予定1200の完了は主張しない。dirty / rapid追加runはfirst projectionのrAF機会が5秒間なくfail closedで停止した（[raw failed run](../evidence/rewrite-finalization/macos-dirty-unavailable.json)）。timerをpaintへ代用せず、dirty / rapid p95を推定しない。selected p95は19–25ms、paint opportunity p95138–218ms、first accepted keyboard interaction p95163–254ms。maxは137.8秒の描画機会待ちを含み、native processing durationと混同しない。cold harness injectionから3103msとfirst source13014msも前面状態が未制御でreference cold / usable値として採用しない。

Windows x64 / macOS arm64 CI nativeは [Tier1 run](https://github.com/kamahir0/masterdata/actions/runs/37071675531)（exact checkpoint2e5b6c1）で各100×3×4が成功。Windows Xeon8573C / RAM16GiB: cold2553–2621ms、warm median30.15–32.37ms / p9533.84–37.44ms / max46.78ms。macOS M1 Virtual / RAM7GiB: cold3737–3973ms、warm median33.25–47.56ms / p9543.93–69.08ms / max80.20ms。同fixture・boundaryだが別hardwareでありlocal Macとの改善率を主張しない。4つのproject-wide work countsは両platform0。CI Macは50ms targetに最大19ms gap。Windowsはnative target内でありUI150msの達成証拠ではない。host / raw distributionsは [evidence directory](../evidence/rewrite-finalization)。dirty revisit / rapid / first accepted interactionを上の4caseから推定しない。過去Linux数値は[historical evidence](../evidence/interactive-navigation.md)であり今回のfresh結果ではない。

## Consumer evidence / limits

最初のactual .NET8 / MasterMemory3.0.4 / MessagePack3.1.3 executionで、generated C# compileとrepeat binary hash一致は通ったが、nested Reward.ItemId.Valueがexpected2001からactual0となった。Arrayとscalar64bitは通った。oracleをactual値へ合わせない。2026-10-03再実行でも同じ不一致。PK / SK / composite / nonunique / required reference / 64-bit / Arrayは7check pass、nested VOの1check fail。Build does not publish、standalone Publishはinvalid current sourceでもeligible receiptを配布、unmanaged / Unity meta保持、corrupt artifact拒否、Build --publish compositionも実行pass。raw reportは [consumer evidence](../evidence/rewrite-finalization/macos-consumer.json)。

2026-10-03 recoveryで前回観測したUnity Editor binary pathは存在せず、fresh installed file inventoryでもUnity executableを得られない。actual Unity runtimeを実行済みとはしない。.NET actual loadはMasterMemory managed consumerを検証するが、Unity Editor compile / player / package interoperabilityの全ては証明しない。

## Readiness blockers / fresh review checkpoint

現時点の判定: **Not Ready**。

1. actual consumerのnested Value Object値が不一致。期待値と現public build/loadの差を解消または十分に同定する必要がある。今回production修正はしない。
2. Tier1 UIのstable distributionは未完。Windows nativeは取得済みだがactual Desktop first accepted interactionは未取得。macOS UIはforeground interruptionを含むpartial evidence。
3. faults.jsonのRecovery Requiredは接続・pass。single-source Outcome Unknownのdeterministic fault adapterとUI state oracle実行が未完。現write boundaryに該当fault seamが公開されていないため、production architecture変更を行わず未実行とする。declarative期待値をcomplete Tier1 oracleと呼ばない。
4. exact Candidate verificationは[Development State](../execution-state.md)を参照する。CI pendingをReady evidenceに数えない。

正式contractの採用とReady verdictを分離する。新implementationは旧code無しに目標を理解できるが、現時点で全Tier1 conformance evidenceが揃ったという回答はNo。rewriteは開始しない。

## Fresh adversarial / code-blind review

2026-10-03、同一agentの別pass。constitution → canonical owners → data-only corpus → matrixの順に読み、production implementationをarchitecture authorityとして使わず次の観点を審査した。別reviewerによる独立承認とは主張しない。

| Attack | Result / correction |
| --- | --- |
| Product: D1〜D5の旧surfaceがcanonicalから復活しないか | baseline / transitionとlegacy query・Overview scopeを照合。compare / Search / paste / compositionを残し、常設surfaceは要求しない |
| UX: Human feedbackをpixelや旧componentへ縮退していないか | direct Complex、operation Undo、stable grid、sticky clipping、non-overlap、whole-target drag、cancel、one-shot focusをworkflow oracleへ保持。0047 lineを再導入しない |
| Safety: simple architectureのためsource safetyを削らないか | exact bytes / fresh identity / dirty physical ownership / Unknown / Recovery / no automatic retryを保持。partial outcomeとcrash-atomic全保証を混同しない |
| Architecture / tests: old codeを模倣しないとpass不能でないか | data-only JSONに旧session / command / private helper名なし。legacy adaptersとsource regexはcurrent regressionのみ。navigation / capacity生成条件もJSONへ抽出 |
| Oracle ambiguity: inputだけではscenarioを再構成不能でないか | rapid A〜Dを具体source aliasへ、large viewportをcapacity.jsonへ、pasteを4 concrete inputsへ結び付けた。Save / structural / Recoveryのexpectedは独立exact disk bytes |
| Performance:機能完成後のtuningへ戻れないか | 最初のvertical sliceに4 work counts=0、bounded work/render、target-filtered paint / accepted interactionとp95を置く。single sample / native / rAF / WebDriverを分離 |
| Consumer: compile / regexだけを互換性と呼んでいないか | actual MasterMemory loadとpublic PK / SK / reference / 64-bitを実行。nested mismatchを隠さず、actual Unityなしも別記録 |

Scope review: production semantics / React / CSS / dependenciesを変更していない。Tauriに追加したものはdefault無効のopt-in measurement adapterと専用identifier / permissionだけ。旧testsは削除していない。rationaleはmeasurementのforeground / native callback boundaryとknown gap追跡に限定し、production architectureの指定にしない。

**Rewrite inputのmerge review: Blockingなし。Rewrite readiness: 上記consumer・Tier1 UI・fault executionのEvidence GapがBlocking。** contractを読んで異なるarchitectureを設計することは可能だが、要求された全証拠が閉じたという判定はしない。Ready gateの阻害要因を解決するためのproduction変更・Unity環境追加は今回行っていない。

## Verification routing

focused core 3 tests（byte16 + target gap1、interpretation9、structural3）とapp 5 tests（Save7、Paste4、empty Table、Recovery、Unix precommit Failure）を実行した。known-gap testをD6 conformance passに数えない。corpus全JSON parse、input / expected存在、旧internal API名非依存、JavaScript syntaxとdiff whitespaceを確認した。

spec / rationale / state integrity、workspace format / Clippy / tests、GUI build、required exact Candidate CIの結果はverification checkpointとGit / CIが所有する。本reportのhistorical measurementsを新Candidateの再測定結果へ置換したようには表示しない。最終verdictは **Not Ready — consumer nested value mismatch / stable Tier1 Desktop distribution不足 / single-source fault実行不足**。rewrite開始は別Human-selected Objective。
