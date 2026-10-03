# Current Objective

## Objective

Legacy Decommission / Clean-slate Preparation。Ready legacyを`legacy-final`でfreezeし、`main`を保持したまま`rewrite/clean-room`だけから旧production / implementation-specific assetsを退役させる。

## Completion boundary

- [decommission manifest](rewrite-preparation/decommission-manifest.json)によるclassification、rewrite authority / corpus / governance保持。
- [handoff](rewrite-preparation/clean-room-handoff.md)、clean-slate integrity、Git/tag/branch安全性、branch CI、fresh review。
- 次のClean-room実装はfresh agent/contextへ渡す。

## Authority / non-scope

[正式rewrite input](rewrite-preparation/README.md)、[GUI baseline](gui/rewrite-baseline.md)、[独立oracle](../fixtures/rewrite-oracle/v1/README.md)、[readiness evidence](evidence/rewrite-readiness/report.md)。

新implementation / architecture / skeleton / framework選択、mainへのmerge、Git history rewriteは行わない。
