> **Historical evidence — not current authority.** Current behavior / statusはcanonical specifications、Development State、code / tests / Gitから確認する。このfileはactive Candidate / Objectiveから必要な場合だけ参照する。

# Desktop制作v1 固定性能証跡

測定日: 2026-09-18（JST）

この文書はDesktop制作v1の固定入力に対する実測値であり、製品のSLAや上限値を宣言するものではない。入力生成と計測の再現手順は [`desktop_v1_performance.rs`](../../crates/masterdata-app/examples/desktop_v1_performance.rs) に固定している。

## 固定入力

- 100,000 records
- 10個のsplit YAML data file（各10,000 records）
- 20 columns（Primary Keyの`id`を含む）
- 10,000-cell paste preview（1,000 rows × 10 editable columns）
- query: `99999` の文字列検索
- validation: profile指定なしの通常プロジェクト全体検証

## 実行環境

- macOS 26.6.2 (25G83)
- Apple Silicon arm64
- Rust `rustc 1.96.0 (ac68faa20 2026-05-25)`
- Node.js `v25.9.0`
- npm `11.12.1`

## 実行コマンド

```text
cargo build --quiet -p masterdata-app --example desktop_v1_performance
/usr/bin/time -l target/debug/examples/desktop_v1_performance
```

## 実測値

| 項目 | 実測値 |
| --- | ---: |
| load | 19,388.833 ms |
| query | 19,366.154 ms |
| 10,000-cell preview | 43,196.525 ms |
| validation | 18,896.576 ms |
| wall clock | 101.77 s |
| maximum resident set size | 3,128,705,024 bytes（約2.91 GiB） |

`load`、`query`、`preview`は対象sourceの再読込・検証を含む。`maximum resident set size`はmacOS `/usr/bin/time -l` の値である。

## Correction Candidate 再計測

測定日: 2026-09-19（JST）

Correctionでquery / batch経路を変更したため、V02解消として同じ固定harnessをrelease buildで再実行した。

- exact tested SHA: `d79e67753859025f33b6852eb712593136988177`
- GitHub Actions: https://github.com/kamahir0/masterdata/actions/runs/35440111781
- runner: `ubuntu-latest` / Linux 6.17.0-1022-azure x86_64
- Rust: `rustc 1.98.1 (48a229cea 2026-09-01)`
- command:
  - `cargo build --release -p masterdata-app --example desktop_v1_performance`
  - `/usr/bin/time -v target/release/examples/desktop_v1_performance`

| 項目 | Correction Candidate実測値 |
| --- | ---: |
| load | 3,888.852 ms |
| query | 3,482.990 ms |
| 10,000-cell preview | 8,760.736 ms |
| validation | 3,382.527 ms |
| wall clock | 19.84 s |
| maximum resident set size | 2,207,028 KiB（約2.10 GiB） |

この再計測は100,000 records / 20 columns / 10,000-cell pasteの固定inputをCorrection Candidate系で再実行したfresh evidenceである。
