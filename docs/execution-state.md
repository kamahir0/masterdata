# Development State

Stage: decision-required
Candidate: none

## Blocking findings

None.

## Human decision needed

`docs/spec-changes/0015-complex-value-authoring.md`の2つのobservable source behaviorを選択する。

1. Structural complex editのsource preservation
   - P1: Fine-grained preservation。target subtree内も必要rangeだけpatchし、安全にlocalizeできなければfail closedする。
   - P2: Target-subtree replacement。structural editではedited value subtree全体のcanonical renderingを許す。
2. Added record draftのunset value representation
   - D1: YAML `null` placeholder。未入力値もSave candidateへ表現し、Required等ではvalidation diagnosticにする。
   - D2: Local-only `Unset`。具体的YAML nodeへ変換されるまでSave candidate生成をblockする。

Recommendation: **P1 + D1**。
