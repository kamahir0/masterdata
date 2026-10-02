# Human decisions D1〜D6

Status: Resolved

2026-10-02 Human-selected Rewrite Contract Finalization promptにより、以下はHuman decisionとして確定。選び直す質問はない。approval / applicationは[0051](../spec-changes/0051-rewrite-contract-finalization.md)が所有する。監査時のalternativesはこのfileのGit historyに残る。

| Decision | Formal result | Canonical owner |
| --- | --- | --- |
| D1 | standalone Diffはbaseline外。unsaved / Conflict / Migration compareを保持 | [baseline](../gui/rewrite-baseline.md) / Data Editor / Source Edit |
| D2 | Typed Filter Deferred、Search / Find / Problems保持 | baseline / Authoring Query |
| D3 | View Sort Deferred、source presentation orderが基本 | baseline / Authoring Query |
| D4 | copy/paste lossless preflight all-or-none one Undo保持。Fill / range Set Null専用workflow / persistent Batch preview外 | baseline / Grid Authoring / Authoring Batch |
| D5 | standalone Saved Overview外。composition / inclusion / saved-vs-dirty保持 | baseline / Build Selection / Authoring Query |
| D6 | existing flow accept + preserve、安全な局所edit。new mappingはblock、unsafe localization fail closed | YAML-SUBSET-019 / SOURCE-EDIT-018 |

新たなmaterial product choice: None。test adapter、corpus形式、測定方法はAgent Decision。性能未測定やconsumer mismatchをHuman choiceへ転嫁しない。readiness evidenceは[finalization report](finalization-report.md)。
