# Interactive navigation performance evidence

Historical Evidence。Current authorityはGUI spec / ADR / Development State。baselineと改修後を同じinput・環境で比較する。

## Baseline（2026-10-01 JST）

開始HEAD: `1194a59f9d7493b186a92cc418efbeeac449443d`。この文書と同時のinstrumentation checkpointで測定。macOS / Apple Silicon、release build。固定入力は[navigation_performance.rs](../../crates/masterdata-app/examples/navigation_performance.rs)が所有する（3 Table、8 source、12,000 records、表示sourceごと2,000行×20列、inline / separate / mixed）。

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

[Navigation Desktop harness](../../apps/gui/tests/navigation-desktop.mjs)は実Tauri/WebKitでpointer / keyboard、React commit、paint opportunity、first usable interaction、timer / frame gap、rapid selectionを記録する。Rust traceはopt-inの`MASTERDATA_READ_TRACE`でcommand threadとphase countを記録する。実GPU paintの完了時刻をJSだけで断言せず、rAFはpaint opportunityとして扱う。

Baseline / afterのexact CI evidenceとUI blocking診断は測定完了後に記載する。
