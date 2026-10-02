# Test Purification Proposal

Status: Approved

testはexecutable evidenceであり、存在だけでproduct contractにならない。意味の分類は [registry](classification.md)、input/oracleは [corpus](compatibility-corpus.md)、workflowは [matrix](acceptance-matrix.md)。旧testはcurrent regressionとして保持し、rewrite gateは独立corpusへのconformanceで切る。

## 6つの役割

| Role | 継承するもの | rewriteで要求しないもの |
| --- | --- | --- |
| Acceptance | user intent → observable result / focus / dirty / save scope | exact component・command・state name |
| Compatibility | bytes / public generated API / source-config / documented automation output | internal DTO / helper topology |
| Safety | fresh identity、no mutation on rejection、partial / unknown / recovery、path boundaries | exact transaction implementation |
| Performance | fixed inputs、work counts、end-to-end tail latency、bounded work/render | old full-load APIを再実装してbenchmarkをgreenにすること |
| Implementation-specific | 現実装の局所bug / rationaleを調査する証拠 | module tree、regex、DOM固定、exact transform |
| Historical | superseded decisionのWHY | 古いUI / Web / retired capabilityの再導入 |

1 testに複数roleがある。case全体をKEEP/DELETEする前にoracleを分解する。特にdirect PK editとbatch既存PK edit禁止は違うscopeであり、単純な矛盾ではない。

## 現行suiteの移植方針

| Suite | 主role / portするevidence | 除外 / refineする拘束 |
| --- | --- | --- |
| GUI `authoring.test.tsx`, `editor-state.test.ts` | Acceptance/Safety: commit、dirty、Save/All、local history、Added/Pending、query state | named state shape、exact command mock、history capacityのcalibrationを独立product premiseにしない |
| `value-editor.test.tsx` | Compatibility/Acceptance: lossless64-bit、unknownEnum、nullable、nested text、Array occurrence、Escape / close | exact `ValueEditor`名、旧Apply/Cancel footer |
| `table-editor.test.tsx`, `type-editor.test.tsx` | Acceptance/Safety: typed drafts、initializer、reference-aware Plan、stale authorization、inline composition | old page separation、exact Plan DTO、dialog path |
| `creation.test.tsx`, `source-creation-request.test.ts` | Acceptance/Safety: typed init、collision、exclusive outcome、lossless、focus intent | request JSONの全fieldをpublic compatibilityにすること |
| `project-surfaces.test.tsx`, `authoring-workflow.test.ts` | Acceptance/Safety: config identity、dirty guards、Recovery、Build / Publish / preview freshness | persistent toolbar placement、publicにしないcontrol label / API |
| `interactive-navigation.test.tsx` | Acceptance/Safety: target即時、stale拒否、dirty/schema/history/query保持、binding、missing deps、Problems、focus replay | exact `editors` / `schemaDrafts`名、revalidate command名 |
| `workspace-navigation.test.ts` | Performance/Safety: latest pending selection、bounded work、workspace change無効化 | one microtask / exact hook名。cancelをoperation errorにしない意味を残す |
| `grid-reorder.test.ts` | UX/Safety: whole target preview、neighbors、uneven geometry、cancel、virtualization/context loss | exact100px/32px測定値、固定DOM順序、transform式、timer90ms |
| `color-theme.test.tsx`, `user-state.test.ts` | Compatibility/UX: theme scopeと旧preference入力 | exact storage implementationの永久固定 |
| `app.test.mjs`（13 source-text checks） | 一部はshared boundary / dirty safety / losslessを意図するが、その仕組みはImplementation-specific | source regexでexact invoke/function/state/dependency-arrayを検査する条件はrewrite入力から除外。least privilegeの意味をactual capability auditへport |
| `workspace-fixtures.ts` | mock入力・期待値の補助資料 | legacy `openDataFile` / `openTableContext` adapterの再現は不要 |
| `desktop-e2e.mjs` | Actual Desktop: focus、geometry、creation、Settings、Build / stale/fresh Publish wiring | geometry calibrationとWebDriver往復をproduct latencyにしない |
| `navigation-desktop.mjs` | Actual Desktop/Performance: selected state、old-content防止、rapid・dirty・keyboard・bounded DOM | synthetic clickを全てOS pointer入力と呼ばない。`reads <= clicks`だけをcoalescing proofにしない |
| core integration（15 files）+ inline tests | Domain/Compatibility/Safety: Tables/types/reference、source patch、migration、parse、config | parser/AST型、helper API、D6はportable flow byte casesで保護 |
| app integration（8 files）+ inline tests | Shared application/Safety: Save / context / receipt / .NET / Publish / workspace | exact stateless service / session type / command split |
| CLI integration（2 files） | Public automation / delivery boundary | internal reportの全JSONfieldの永久固定。変更前にpublic/undocumentedを確認 |
| codegen integration（1 file）+ .NET tests | Consumer Compatibility: generated code / binary compile-load、selection、canonical order | formatting snapshotやinternal bridge requestをsemantic contractと混同しない |
| xtask integration（1 file）/ Unity checker | Repository integrity / static package boundary | workflow stageをproduct runtimeへ写すこと、string scanをUnity runtime証明にすること |

合計はGUI `.test.*` 15 files、Rust `crates/*/tests/*.rs` 27 files。inline Rust tests・Tauri tests・.NET・Desktop scriptsは別であり、この数は実行case数でもpass数でもない。

## 特に危険なoracleの例

`app.test.mjs`は `App.tsx` のfunction slice、`open_workspace` / `select_source` / `source_content`、`pendingCellFocus`、`[activePath, editors, loadingPaths]` 等のliteralを検査する。これは移動やrenameでfalse failureを作り、別fileにdomain処理を追加しても検出できない場合がある。保持するのは「frontendがdomain semanticsを再実装しない」「navigationがdirtyを失わない」「64-bit exact」「focus replayしない」であり、このregex条件ではない。

reorder testの「source DOM orderは変わらずtransformだけ変化」は現実装のpreview mechanism。observableなのは掴んだ対象とneighborの位置、click/focus不能なghost、cancel無変更、commitの一回性。新実装がDOMを移動しても安全で高速なら不合格にしない。

phase instrumentationのcounterはarchitectureを検証できる有用なseam。名前やcounter構造は自由でも、「通常warm navigationにproject-wide parse / validationを戻さない」というoracleは残す。

## 後続suite案

acceptance、compatibility、safety、performanceを意味ごとに識別し、同じsource bytes caseを共有できる。directory名やtest frameworkは拘束しない。core semanticsは最小boundaryでdeterministicに試し、actual Desktopはgeometry / focus / native-thread / IPC / first usable interactionに集中する。

port順序は、source/config/consumer corpus → fresh write safety → daily authoring → navigation / bounded renderを最初のvertical sliceで同時に検証する。全機能後にperformance suiteを追加する順序にはしない。

## Rewrite gate manifest

| Gate class | 対象 |扱い |
| --- | --- | --- |
| Rewrite acceptance | fixtures/rewrite-oracle/v1 のinput / intent / expected、canonical domain / GUI contract | 新adapterで同じ意味を検証。legacy helper不要 |
| Current implementation regression | 現Rust / GUI / Tauri / .NET test、source-text app.test.mjs、exact DTO mock、transform calibration | 現製品維持に実行する。新architectureのAPI / topologyを要求しない |
| Historical / obsolete | 0047 insertion line、0044 revoked Save案、旧Complex footer、retired Web / Computed / Released、D1〜D5 legacy surface test | future rewrite gate外。現testに残るならhistorical / legacy guardとして読む |

core/appのrewrite_oracle.rsとconsumer scriptはcurrent implementation adapterでありoracleそのものではない。known gapをassertしてgreenにする現adapter testはconformance passではなくgap固定・期待値保全のregression。rewrite implementationではgap exemptionを認めない。

source regexで偶然守られていたdirty / lossless / stale / focus boundaryはworkflows.json、byte / Save scenarios、performance work-countへ移した。GUI mocked testだけでactual Desktop interactionを証明しない。
