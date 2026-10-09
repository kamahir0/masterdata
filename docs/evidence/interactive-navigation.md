# Interactive navigation performance evidence

Historical Evidence。Current authorityはGUI spec / ADR / Development State。baselineと改修後を同じinput・環境で比較する。

## Baseline（2026-10-01 JST）

開始HEAD: `1194a59f9d7493b186a92cc418efbeeac449443d`。この文書と同時のinstrumentation checkpointで測定。macOS / Apple Silicon、release build。固定入力は[旧implementation evidence: navigation_performance.rs](https://github.com/kamahir0/masterdata/blob/legacy-final/crates/masterdata-app/examples/navigation_performance.rs)が所有する（3 Table、8 source、12,000 records、表示sourceごと2,000行×20列、inline / separate / mixed）。

Command: `cargo run --release -p masterdata-app --example navigation_performance`。single pass、ms。phaseはinclusiveであり加算しない。

| Case | wall ms | parse count | parse ms |
|---|---:|---:|---:|
| A-cold-project | 2326.3 | 8 | 2317.0 |
| B-first-source | 4859.4 | 16 | 4635.2 |
| C-revisit-frontend-buffer | 2379.4 | 8 | 2285.8 |
| D-same-table-record-source | 4770.5 | 16 | 4568.2 |
| E-cross-table | 4772.4 | 16 | 4565.4 |
| F-schema-redirect | 7135.8 | 24 | 6842.3 |
| G-rapid-four | 19102.3 | 64 | 18285.5 |

初回のdiscoveryは2回、source enumerationは3回、file read / parseは16回。schema redirectは24 parse。Data projectionは約100ms、そのうち全体validationが約80–100ms、JSON serializationは約6ms。再訪でもcontext側の8 parseが残る。全体parseの繰返しが支配的であり、payload paginationを先行導入する根拠はない。

## Desktop evidence

[旧implementation evidence: Navigation Desktop harness](https://github.com/kamahir0/masterdata/blob/legacy-final/apps/gui/tests/navigation-desktop.mjs)は実Tauri/WebKitでpointer / keyboard、React commit、paint opportunity、first usable interaction、timer / frame gap、rapid selectionを記録する。Rust traceはopt-inの`MASTERDATA_READ_TRACE`でcommand threadとphase countを記録する。実GPU paintの完了時刻をJSだけで断言せず、rAFはpaint opportunityとして扱う。

Baseline: [Desktop Evidence run 36790153897](https://github.com/kamahir0/masterdata/actions/runs/36790153897)、Candidate input `16d8077e620beb8ff2218f8d0c467f31922e8fdc`。Linux / WebKit / release。cold openは7,408ms。通常初回のgrid到達4,731ms、再訪2,534ms、同Table切替2,540ms、別Table4,738ms、schema selection6,919ms。first usable操作を含むW3C往復はgrid到達後に約250msを加えるため、product latencyの判定ではfrontend traceも併記する。

同期Rust commandのthreadは`ThreadId(1)`。初回切替ではrAF間隔に4,289ms、schema選択では2秒以上の空白が出た。JS timerの最大間隔は約300msで、WebKit側のJS event loopとnative windowのrepaint停止は同一ではない。重複parseによるprocessingの遅さとnative UI threadのblockingが併存する。

## Session導入後のnative measurement（途中checkpoint）

同じmacOS / release / fixed input。rustc 1.96.0、Node 25.9.0。cold loadは2,415ms / 8 parseのまま。B初回29.5ms、C再訪26.6ms、D同Table26.2ms、E別Table30.2ms、F schema28.8ms。すべてnavigation中のdiscovery / enumeration / YAML parse / validationは0回。4 selectionのsequential native sampleは106.6ms（Desktop rapid-click latencyとは区別する）。

約11msがselected-sourceのData projection、約6msがserialization、残りがexact source identity確認とDTO準備。Project validation約93msはcaptured generationのbackground operationへ移した。Table contextはlocal declarationから導出し、Reference resolutionのcompletionは同じgenerationのvalidation結果から合成する。

## Session checkpointのDesktop measurement

[run 36798464348](https://github.com/kamahir0/masterdata/actions/runs/36798464348)、input `7b2b3cbefd29d98f177c7f35d118f96bab8d872e`。Linux / WebKit / release、rustc 1.98.1、Node 22.23.2。cold 3,736ms、通常grid到達491–736ms。selectionのReact commitは21–29ms、IPC return約104–119ms、grid paint opportunityは445–554ms（対象path・selection以降のeventだけを比較）。旧generationのscheduled rAFを新selectionのpaintとして数えない。

readは`ThreadId(9–11)`へ移り、約45–59ms、navigation parse / discovery / enumeration / validationは0回。40 selectionは18 readへcoalesceし、last target、dirty value、bounded DOMとkeyboard grid focusを保持した。ordinary Desktop workflowも成功。native UIの数秒blockingは解消したが、React commit約200–250ms + layout/paint約110–150msが残る。primitive cellにも閉じたPopoverを生成していたため、Complex columnだけに限定して次のcheckpointで再測定する。

固定100,000 records / 10 files / 20 columns / 10,000 paste cellsのharnessも成功。baselineのEPYC 9V74に対し改修後runnerはEPYC 7763であり、同一hardwareの速度比較とは扱わない。load / query / validationは約69秒→76–80秒、paste previewは約146秒→90秒、peak RSS約2.4GB→1.8GB。入力と出力契約を保持し、batch previewのduplicate project loadを除いた。

## Grid render measurement

[run 36799719390](https://github.com/kamahir0/masterdata/actions/runs/36799719390)、input `d811361807fb6d0c8b7b541ac512d09ebdc32355`。Complex columnだけにPopoverを生成する変更後、6 navigationの対象grid paint opportunityは276–378msへ短縮した。native read / IPCは約100ms、初回grid commitは約190–263ms。その後viewportの640px初期値を実寸へ補正するsecond commitがあり、paintは約276–378msまで待っていた。viewportはfileごとではなくeditor面に属するため、実測サイズをsource切替で再利用する根拠となった。

このrunのkeyboard stressは、Arrow移動だけでfile openを期待したharnessが既存のEnter-open仕様に合わず失敗した。上記6 sample以外を成功evidenceに含めない。修正後の最終product treeはexact Candidateを指定したDesktop Evidence artifactでpointer / keyboard / 40 selectionとfirst usable interactionを再検証する。W3C操作往復を含むwall timeとfrontend paint opportunityを区別し、cold openをwarm navigationの改善と混同しない。
