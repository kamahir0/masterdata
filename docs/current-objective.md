# Current Objective

## Objective

**同一Project内のExplorer selectionをinteractiveなnavigationへする。critical pathを計測し、重複するdiscovery / enumeration / read / parse / validation / derivationを除去する。必要なread/session architectureを再設計し、authoritative writeのfresh preflightを維持する。**

## Completion slices

- cold / first visit / revisit / same Table / cross Table / schema / rapid selectionのbaseline traceとUI blocking診断。
- shared Applicationのreusable workspace read model、navigation critical pathとvalidationの分離、freshness / invalidation。
- immediate target feedback、latest-selection-wins、dirty / history / query / schema draft保持。
- Desktop rapid / long / wide evidence、外部変更とwrite preflightの回帰、固定100k/10k性能、focused / repository check / required CI / fresh review。

## Canonical requirements

- [Explorer](gui/explorer/spec.md) `GUI-EXPLORER-002`, `GUI-EXPLORER-STATE-003`, `GUI-EXPLORER-INT-001`
- [Data Editor](gui/data-editor/spec.md) `GUI-DATA-STATE-004`, `GUI-DATA-STATE-005`, `GUI-DATA-VAL-005`
- [Unified Table](gui/table-editor/spec.md) `GUI-UNIFIED-003`, `GUI-UNIFIED-004`, `GUI-UNIFIED-005`
- [Source Edit](specs/source-edit.md) `SOURCE-EDIT-008`, `SOURCE-EDIT-009`, `SOURCE-EDIT-011`

Measurement evidence: [interactive-navigation](evidence/interactive-navigation.md)。

## Explicit non-scope

Search / Filter / Sort、Diff、Batch、Explorer visual、Save scope、Undo semantics、YAML syntax、Migration semantics、Build / Publish、plugin、general incremental compilerの再設計。
