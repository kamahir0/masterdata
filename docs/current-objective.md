# Current Objective

## Objective

**Rewrite Input Purification。既存spec / test / fixture / evidence / Git historyを再審査し、production codeをarchitecture templateにしなくても再実装できる、contractとacceptance corpusのDraftを精製する。**

## Completion slices

- fresh recoveryとrepository-wide provenance audit。重要項目のKEEP / REFINE / DEMOTE TO HISTORY / DELETE候補 / HUMAN DECISION NEEDED分類。
- Product / Desktop UX / Non-goals / Domain・Safety / Performance constitution、acceptance matrix、compatibility corpus、test purification proposal。
- superseded decisionとimplementation拘束の特定、materialなHuman decision queueの集約。
- Archaeology / Product critic / Rewrite architect / Adversarial reviewerの4 pass、rewrite readiness assessment、doc/state checks、required CI、fresh review。

## Authority / output routing

現行authorityは[Product Vision](product/vision.md)、[canonical specs](specs/README.md)、[GUI specs](gui/README.md)。今回はこれら自体が監査対象であり、Approvedを自動KEEPしない。

Draft成果物は `docs/rewrite-preparation/` に置く。current canonical authorityを置換せず、Human decision前の削除・互換性変更を適用しない。

## Explicit non-scope

production implementationのrewrite / refactor / cleanup、canonical spec / test / fixtureの削除・大規模移動、framework / dependency replacement、rewrite着手。
