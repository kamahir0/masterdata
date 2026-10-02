# Performance / Responsiveness Constitution

Status: Approved

性能は最初のvertical sliceからproduct contractとして扱う。次の数値は0051 Human-selected rewrite targetであり、現行実装が満たしたという主張でも、異なるCI hardwareへ固定thresholdを適用する承認でもない。[適用境界](README.md) に従う。

## Baselineの監査

基準inputは **3 Tables / 8 sources / 12,000 records、selected source 2,000 rows × 20 columns、inline / separate / mixed**。native測定と実Desktop測定を混同しない。

| 測定 | Before | 最終evidence | 意味 / 限界 |
| --- | --- | --- | --- |
| macOS arm64 release native cold | 2,326ms / 8 parses | 2,395ms / 8 parses | cold改善ではない |
| 同環境 native warm B〜F | 2,379–7,136ms / 8–24 parses | 26.2–32.2ms / project-wide parse 0 | discovery / enumeration / validationもselection内0。単発sampleでp95ではない |
| Linux release native warm | baselineとは別環境 | 45.3–57.7ms | hardware差を含む。Macの値でLinux SLA達成を主張しない |
| Linux actual Tauri Desktop grid到達 | 2,534–6,919ms | 306–384ms | native高速化後にもfrontend / IPC costが残る |
| 同Desktop selection React commit | — | 13–17ms | next committed selected stateであり、GPU paint完了ではない |
| 対象path / selection以降のgrid paint opportunity | — | 262–350ms | rAFによる機会。100–150ms targetは未達 |
| first usable操作までのWebDriver wall | — | 457–552ms | WebDriver往復を含む。上記paint値と加算しない |
| 最大frame gapの代表値 | 4,289ms | 通常136–188ms | 秒単位blockは解消したが、next-frame品質の証明ではない |

native final内訳はidentity I/O約0.3–0.6ms、projection約11–12ms、serialization約6ms。project-wide validation約94ms（Linux約157ms）はbackground。cold read/parseとwarm view derivationを分離したことが大きい。旧commandが同じclickからfull loadを重ね、schema redirectも追加readしていたことはphase traceで確認されている。

baseline readはnative UI ThreadId(1)、final readはworker ThreadId(9–13)。これはslow processingとUI blockedの両方があった証拠である。残るframe gapとpaintの差からfrontend / IPC / layoutもrewriteの対象となる。どれが最終支配項かの細分化は追加profileが必要。

Source: [navigation evidence](../evidence/interactive-navigation.md)、[最終exact Candidate reconciliation](https://github.com/kamahir0/masterdata/commit/eb2acd00af4bdd3c9751fab134ba7e46300f7e59)。最終Candidateは `ab93c30df14fb95905ec4aaf11a59848a9be5901`。Linuxはrustc 1.99.0 / Node 22.23.3 / Tauri-WebKit / release。個別artifact、intermediate checkpoint、最後のcommit記録は区別する。

### 100k evidenceの扱い

固定 **100,000 records / 10 data files / 20 columns / 10,000 paste cells（1,000 × 10）** はcapacity regression assetとして残す。[harness](../../crates/masterdata-app/examples/desktop_v1_performance.rs) はone-shot load / query / preview / validationを測り、Explorer → usable gridを測っていない。

final Linux EPYC7763 / 4 vCPU: load 78,161ms、query 82,069ms、preview 91,766ms、validation 77,237ms、peak RSS 1,811,608KiB。baseline EPYC9V74や過去Mac debug sampleとの比較で因果的な改善・退行を主張しない。[旧performance evidence](../evidence/desktop-v1-performance.md) の短い別sampleも同一条件ではない。

固定input維持は有用だが、harness成功だけでquery結果やpaste全cellの期待値が証明されたとは扱わない。新corpusでは正しい出力oracleとstage別latency / RSSを付ける。100k全量のone-shot処理を毎navigationで再現する要件は除外する。

## 構造上の契約

通常の同一Project warm selectionのcritical pathでは、**project-wide discovery = 0、enumeration = 0、YAML parse = 0、validation = 0**。changed source / required dependencyのfreshness確認、局所reparse、必要なprojectionは許容する。inventoryのbackground reconciliationを禁止する意味ではないが、clickごとの全体rebuildや並列rebuildは許容しない。

Projectは既にopenしている。selectionはviewを選ぶ。parsed readとresolved shapeを再利用できるlifetimeを持ち、heavy reads / validationをnative UI threadから分離する。publicationのための短い同期は許容し、parse / validation中のglobal exclusive lockでnavigationを長時間待たせない。exact session型、lock、Tauri command、React hookは拘束しない。

selectionはbackend completionを待たずtargetを示す。new target header / loading stateを表示し、old contentをnew targetとして見せない。A→B→C→DではDだけがactive resultになり、obsolete result / diagnostics / focus intentを拒否する。可能なworkはcancel、実行中cancel不能ならpendingをcoalesceし、queueとCPU/I/O負荷をboundedにする。

dirty overlayはread baseと別所有。read cacheはwrite authorizationではない。freshness不明なsnapshotをcurrent editableとして出さず、必要なidentity確認を経る。writeのfresh preflightは [domain-safety](domain-safety.md) を維持する。

## Reference-hardware budgets

| 操作 / boundary | Rewrite target | 必須の測定区別 |
| --- | --- | --- |
| selection visual feedback / target identity | next frame、通常16–33ms級 | React commitとpaint opportunityを別記録 |
| warm backend view derivation（2,000 × 20） | p95 < 50ms級 | freshness + projection + serializationまで。IPCは別 |
| selection → usable editor（同input） | p95 < 150ms、stretch < 100ms | next targetにkeyboard/cell操作が受理されるまで |
| first source projection | 上記と独立報告、warm同等を目指す | unvisitedでもProject loadを再実行しない |
| same Table record source / cross Table / schema redirect | 各p95を独立に報告、warm budgetを適用 | schema selectionから追加full loadを出さない |
| cold Project open（12k corpus） | 2秒級を改善目標として検討 | initial feedbackはnext frame、usableまで別。未測定platformへSLA固定しない |
| scalar / Complex operation / Undoのlocal反映 | next frame、commit critical path < 50ms級 | semantic patch・validation完了とは分離 |
| diagnostics refresh | UIをblockしない。12k corpusのcompletionは1秒級を候補 | generation、pending時間、dirty projectionを記録 |
| local single-source Save（2,000 × 20） | 200ms級を候補として測定から確定 | fresh preflight / commit / fs latency / Unknownを分離。安全性を省略しない |
| large / wide scroll・editing | bounded render、通常frame < 33ms級、main-thread long taskを追跡 | 100k dataset全量転送・mountとviewportを区別 |

50ms / 150msはHuman-selected reference-hardware targetであり、異なるhardwareへ一律適用するhard CI SLAではない。現行Desktopはwarm UI target未達であり、rewrite readinessはこのgapを明示した状態で判断する。capacity operationsの絶対budget / memory capは現evidenceだけでは定まらない。reference hardwareでthroughput / peak RSSを測り、baseline比とabsolute tailの両方を最初のsliceから残す。

## 測定protocol

release、Tier1 macOS arm64 / Windows x64を主対象に、CPU / core / RAM / OS / runtime / toolchain / fixture hash / selected dimensions / dirty stateを記録する。Linux CIは補助evidence。同じ条件のbaselineなしにhardware間の改善率を出さない。

cold Project open、未訪問source、再訪source、same Table、cross Table、schema source、dirty再訪を別caseにする。warm各caseは少なくとも100 selections、複数runでmedian / p95 / maxとwork countsを記録する。OS cache warmとProject session warmも区別し、system cacheの強制消去を通常testの前提にしない。

traceはpointer/keyboard input → selection publication → backend start → discovery/enumeration/I/O/parse/index/context/projection/validation/serialization → IPC return → state publication → React commit → layout → target-filtered paint opportunity → first accepted interaction。各stageのexclusive時間と重複時間を区別する。WebDriver round-trip、rAF opportunity、実際のusable判定を同じlatencyにしない。

rapid testは20–50 selections、最後のtarget、dirty/history/query保持、bounded pending work、obsolete result拒否、no full parseをassertする。現行Desktopの40 synthetic clicks / 10ms → 11 readsは有用なsampleだが、`reads <= selections`だけではcoalescingの強いoracleにならない。blocking readをcontrolledに遅らせ、pendingが最新に集約され、drain後にobsolete queueが残らないことをdeterministicに検証する。exact11 readsやmicrotask topologyは固定しない。

## Frontend auditとarchitecture自由度

closed per-cell Popoverのmount costが旧Desktopで見つかり、`d811361`で改善された。source switchingによる過剰なcomponent recreation、geometry再測定、duplicate commit、global update fan-out、offscreen controls、巨大DTO、unbounded DOMを計測対象にする。exact componentの再利用をtestが要求しても、product契約にはしない。

semantic datasetとcurrent UI projectionは分離できること。DOM virtualizationだけでserialization / IPC costが消えたと扱わない。現行2k native serialization約6msだけを理由にpagination protocolを必須化しない。bounded transportは支配costの測定で必要になった時に選ぶ。最初のvertical sliceに上記inputとwork-count / end-to-end harnessを置き、機能完成後のtuningへ延期しない。

## Finalization時のfresh evidence

今回のdistribution、current adapterのgap、未取得platform / boundaryは [finalization report](finalization-report.md) を参照する。上の0050の数値はHistorical Evidenceであり今回の再実行結果ではない。正式のobservable contractは [GUI performance](../gui/performance.md)。dirty revisit / rapid / first accepted interactionをnativeの4caseから推定しない。
