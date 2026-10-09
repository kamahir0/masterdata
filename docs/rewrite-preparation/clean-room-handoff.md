# Clean-room Handoff

`rewrite/clean-room`は意図的なnon-buildable境界。旧product runtimeも新product skeletonもない。**次の実装は完全にfreshなagent/chat/contextで開始する。同じbranchを使い、このconversationを継承しない。**

## Start here / authority

1. [AGENTS](../../AGENTS.md)、[workflow](../execution-workflow.md)、[Objective](../current-objective.md)、[State](../execution-state.md)。
2. [正式input / hierarchy](README.md) → [Product / UX / Non-goals](constitution.md) → [Domain / Safety](domain-safety.md)。
3. [canonical domain index](../specs/README.md)、[GUI baseline](../gui/rewrite-baseline.md)、[acceptance matrix](acceptance-matrix.md)。詳細意味論は各canonical ownerが所有する。
4. [独立oracle](../../fixtures/rewrite-oracle/v1/README.md)、[compatibility inventory](compatibility-corpus.md)。input / intent / expectedはdata-only。consumerのC#は公開成果物を試すoracleでありgeneratorではない。
5. [Performance Contract](performance.md)、[Ready baseline](../evidence/rewrite-readiness/report.md)。nativeとactual Desktop、p95と単発、paint機会とaccepted interactionを区別する。最初のvertical sliceから検証する。

corpusとcanonicalの矛盾は勝手に片方へ合わせずspecific questionとして扱う。durable ADRは意味論共有・source authority・native delegation・read/write分離の理由を示す。旧具体module / transport / type構成を再現する拘束ではない。

## Forbidden default inputs / forensic exception

`legacy-final`、旧production Git history、historical RFC / spec-change / audit内の実装名はarchitecture derivationのdefault inputに使わない。旧codeをsolution templateとしてcopyしない。

contract / oracleだけでは既存compatibilityを解釈できない場合に限り、先に具体的なquestionと調査範囲を立て、targeted legacy archaeologyを行ってよい。結果はbehaviorの根拠として精製し、旧解法を持ち込まない。

## Boundary verification

`python3 tools/check-clean-slate.py`（Python 3.11以上、検査専用PyYAML）。これはdocs / corpus / state / frozen asset integrityのみを検証する。product runtime conformance、実consumer、Desktop性能の再測定ではない。

[manifest](decommission-manifest.json)は退役前tracked assetsのexact分類。[退役review](decommission-report.md)は境界の検証結果。新architecture / framework / workspaceの選択は今回行っていない。
