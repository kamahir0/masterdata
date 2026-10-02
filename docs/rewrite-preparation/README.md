# Rewrite Input Purification

Status: Draft

これは **rewrite preparation / audit artifact** であり、current canonical spec、ADR、public compatibility、production behaviorを変更しない。監査基準点は `eb2acd00af4bdd3c9751fab134ba7e46300f7e59`。今回のHuman-selected Objectiveは精製案の作成までで、rewrite着手とcanonicalへの適用は別Objectiveである。

## 入力の優先順位

現在の開発authorityは引き続き [Product Vision](../product/vision.md)、[canonical specs](../specs/README.md)、[GUI specs](../gui/README.md)、[execution workflow](../execution-workflow.md) が所有する。このDraftをcurrent MUSTとして扱わない。

将来、Human decisionとpurification applicationが完了したrewrite入力は、次の順序にする。

1. 適用済みconstitutionとcurrent canonical contract。未解決decisionを明示する。
2. acceptance / compatibility corpus。期待結果と由来を持ち、古い内部APIをoracleにしない。
3. durable architecture constraintsを説明するADR。型名やtransportの例は拘束しない。
4. historical spec changes / rationale。経緯の復元用であり、superseded MUSTを復活させない。
5. legacy implementation。互換性の調査資料であり、architecture templateではない。

矛盾した入力を「後に読んだfileが勝つ」で解決しない。未解決は [decision queue](human-decisions.md) へ戻す。Approved specも監査対象だが、このDraftの分類だけで無効にはならない。

## このdirectoryのowner

| 知識 | Owner |
| --- | --- |
| Product Constitution / Desktop UX / Non-goals | [constitution](constitution.md) |
| Domain / Safetyの最小契約と理由 | [domain-safety](domain-safety.md) |
| Performance監査・budget・測定方法 | [performance](performance.md) |
| workflowごとのobservable acceptance | [acceptance matrix](acceptance-matrix.md) |
| fixture / byte / public compatibility資産 | [compatibility corpus](compatibility-corpus.md) |
| 重要項目の分類・両方向のrisk・provenance | [classification registry](classification.md) |
| testの移植・除外方法 | [test purification](test-purification.md) |
| materialな選択だけ | [Human decisions](human-decisions.md) |
| coverage / 4 audit passes / obsolete decision / readiness | [audit report](audit-report.md) |

分類単位は重要なcontract・decision・test群であり、全Requirement / test caseの個別判定件数ではない。registryのaudit labelはcanonical Requirement IDではない。

## 継承の原則

Preserve knowledge, not implementation inertia.

Interactive latency is a product contract, not a post-implementation optimization.

Shared semantics do not require shared lifecycle, request topology, or UI topology.

現行codeなしで次実装を設計する自由と、既存sourceを安全に扱う義務は両立する。重要なdomainの詳細はcanonical ownerへ参照を残す。全文の複製や旧module mapを新設計の代わりにしない。
