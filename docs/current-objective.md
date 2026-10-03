# Current Objective

## Objective

**Rewrite Readiness Gap Closure。nested Value Object consumer defectのfirst-loss boundaryを同定し最小production correctionを行う。controlled Tier1 actual Desktop baselineを取得し、正式rewrite inputのReady / Not Readyを再判定する。rewriteは開始しない。**

## Completion slices

- minimal / full consumer reproduction、value-path観測、first-loss boundary、direct / nested / key-order regression、actual consumerとBuild determinism。
- 少なくとも1つのcontrolled Tier1 Desktopでclean / same Table / cross Table / schema / dirty / rapidを測定。invalid environmentを分離しstage別distributionを記録。
- macOS / Windows native hard invariant evidenceのreconciliation、fresh review / check-all / required CI、readiness再判定。

## Authority / output routing

consumer authorityは[Value Object](specs/type-system/value-objects.md)、[Custom Type](specs/type-system/custom-types.md)、[Build](specs/build-pipeline.md)、[portable consumer oracle](../fixtures/rewrite-oracle/v1/consumer/scenario.json)。performance targetは[GUI performance](gui/performance.md)と[rewrite performance](rewrite-preparation/performance.md)。

Human-selected Gap ClosureのDecision Aは最小consumer correctness修正を許可する。Decision Bはrewrite前baselineにcontrolled Tier1 actual Desktop少なくとも1platform、rewrite final conformanceに双方を要求する。成果と限界は[finalization report](rewrite-preparation/finalization-report.md)へrouteする。

## Explicit non-scope

clean-room rewrite、architecture全面刷新、GUI redesign / general performance optimization、新機能、D1〜D6再審査、source format再設計、broad cleanup、unrelated warnings / dependency upgrade。
