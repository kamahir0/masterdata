# Final rewrite input

Status: Approved

0051による正式rewrite target input。current productionのcleanup / rewrite着手を許可するものではない。**Readyの判定は別のevidence gate**で、[Gap Closure readiness evidence](../evidence/rewrite-readiness/report.md)とexact Candidateの[Development State](../execution-state.md)へrouteする。[finalization report](finalization-report.md)は前ObjectiveのHistorical Evidence。現行codeをarchitecture templateとして読む必要はない。

## Authority hierarchy

1. [Product Constitution / UX / Non-goals](constitution.md)と適用済み[canonical domain](../specs/README.md) / [GUI](../gui/README.md)。constitutionはproduct scopeとrouting、詳細意味論はcanonicalが所有する。
2. [独立oracle](../../fixtures/rewrite-oracle/v1/README.md)、[acceptance matrix](acceptance-matrix.md)、[compatibility inventory](compatibility-corpus.md)。期待結果は旧helperから生成しない。corpusとcanonicalの衝突は検査し、勝手に「後に読んだfile優先」としない。
3. durable ADR。共有Rust意味論、source authority、fresh write boundaryを保持し、具体型 / command / transport / lockの例を拘束にしない。
4. historical spec-change / RFC / rationale。経緯の復元用であり、Applied artifact、superseded MUST、初期non-goalをcurrentへ復活させない。
5. legacy production implementation。互換性調査資料であり、新architectureのauthorityではない。

[Domain / Safety](domain-safety.md)は最小invariantと理由、[Performance](performance.md)は測定protocolとreference budgets、[test purification](test-purification.md)はrewrite / legacy / historicalの分離を所有する。D1〜D6は[resolved record](human-decisions.md)。normal development governanceは引き続き[workflow](../execution-workflow.md)。

## Historical audit boundary

[classification](classification.md)と[audit report](audit-report.md)は`eb2acd00af4bdd3c9751fab134ba7e46300f7e59`に対する監査資料。Draft labelは当時の分類提案を示し、最新Human D1〜D6の未解決を意味しない。内部API名・component名はlegacy evidenceであり、正式rewrite contractへ持ち込まない。

Preserve semantics, UX knowledge, compatibility, and performance expectations — not legacy topology.

Interactive latency is part of correctness for a Desktop authoring tool.

Shared semantics do not require shared lifecycle, request topology, or UI topology.
