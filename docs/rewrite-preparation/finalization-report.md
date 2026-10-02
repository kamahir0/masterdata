# Rewrite Contract Finalization — evidence / readiness

Status: In progress

## Canonical application / purification delta

0051のHuman D1〜D6は[resolved record](human-decisions.md)から[GUI baseline](../gui/rewrite-baseline.md)、app shell / Data Editor / Grid Authoring / Project Workflow、Authoring Query / Batch、YAML subset / Source Editへ適用した。独立Diff / Typed Filter / View Sort / Advanced Batch / standalone Overviewの必須surfaceを解除し、current legacy capabilityを即削除する要件にはしていない。

KEEP: compare safety、Search、lossless paste、physical composition / Build inclusion、source safety、recent direct / spatial UX、interactive read principles。REFINE: lifecycle / request / surfaceとshared semanticsの分離、independent oracle / hard work counts。DEMOTE: 中間insertion line、revoked Save案、旧Complex footer、initial scope、監査時点のD1〜D6未決定。remove from rewrite baseline: D1〜D5の5surface群。旧testは消さず[3 gate classes](test-purification.md#rewrite-gate-manifest)へ分離した。

## Independent oracle coverage

[portable v1](../../fixtures/rewrite-oracle/v1/README.md)の期待値はsource bytesとdomain intentから独立に記述し、legacy helperによる生成・DTOのcopyを行わない。core / app / CLI-.NET adapterは別owner。

- Byte17: current adapterで16pass。new-mapping-blockはD6 target-only gap（legacy writerはflowを生成）。greenなgap regressionをconformance passと数えない。
- Interpretation9: current adapterで全case pass。
- Physical Save7: schema / selected records / both / inline / mixed / external Conflict / stale cached identityを全disk bytesとcommit範囲で検証しpass。
- Structural3: Rename / Add / Dropのmulti-source candidate exact bytes、Plan input不変、destructive分類がpass。
- Workflow23: UI / source lifetime / faults / migration / deliveryのportable event-state期待値。全caseの自動adapterが接続済みという主張ではない。
- capacity.json: 100k / 10files / 20columns / 10k pasteの数学的expected result。actual runは追記する。
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

macOS actual Tauri WebView distributionは実行中。Windows x64はexact CandidateのTier1 workflowで取得する。dirty revisit / rapid / first accepted interactionを上の4caseから推定しない。過去Linux数値は[historical evidence](../evidence/interactive-navigation.md)であり今回のfresh結果ではない。

## Consumer evidence / limits

最初のactual .NET8 / MasterMemory3.0.4 / MessagePack3.1.3 executionで、generated C# compileとrepeat binary hash一致は通ったが、nested Reward.ItemId.Valueがexpected2001からactual0となった。Arrayとscalar64bitは通った。oracleをactual値へ合わせない。2026-10-03再実行でも同じ不一致。PK / SK / composite / nonunique / required reference / 64-bit / Arrayは7check pass、nested VOの1check fail。raw reportは [consumer evidence](../evidence/rewrite-finalization/macos-consumer.json)。

2026-10-03 recoveryで前回観測したUnity Editor binary pathは存在せず、fresh installed file inventoryでもUnity executableを得られない。actual Unity runtimeを実行済みとはしない。.NET actual loadはMasterMemory managed consumerを検証するが、Unity Editor compile / player / package interoperabilityの全ては証明しない。

## Readiness blockers / fresh review checkpoint

現時点の判定: **Not Ready**。

1. actual consumerのnested Value Object値が不一致。期待値と現public build/loadの差を解消または十分に同定する必要がある。今回production修正はしない。
2. Tier1 UI / Windows distributionのreconciliationが未完。
3. fault position別 Unknown / Recoveryの独立byte corpusとadapter接続が未完。declarative期待値をcomplete Tier1 oracleと呼ばない。
4. required check-all / exact Candidate remote CI / final code-blind reviewが未完。

正式contractの採用とReady verdictを分離する。新implementationは旧code無しに目標を理解できるが、現時点で全Tier1 conformance evidenceが揃ったという回答はNo。rewriteは開始しない。
