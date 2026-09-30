# Development State

Stage: correction-ready
Candidate: 4a3746be449bdc6e92aa8a1f47442496c382a269
Work base: a228d2b15ca1844c3fb9c4aa728c28cdca6bb61d

## Active work

Completed: GUI-UNIFIED-008 / GUI-GRID-007 drag preview実装、local check、self-review。
In progress: scroll後のsticky header clip補正とregression evidence。

## Blocking findings

GUI-GRID-007: thead自体はstickyではなく、縦scroll後にrow previewのclip上端が固定headerより上へずれる。
