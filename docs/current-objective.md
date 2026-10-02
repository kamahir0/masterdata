# Current Objective

## Objective

**Rewrite Contract Finalization。Human確定D1〜D6をcanonicalへ適用し、現行内部API非依存のCompatibility / Acceptance Oracle、Tier1 p95とconsumer evidence、正式rewrite inputを完成させる。production rewriteは開始しない。**

## Completion slices

- D1〜D6のcanonical application、stale / superseded proseとowner conflict解消。
- 独立source / safety / topology / paste / UI / Build / Publish / consumer oracle、legacy-only test分離。
- Tier1 warm p95、cold / first usable / rapid evidence、100k correctness/capacity oracle、performance contract確定。
- adversarial / code-blind review、focused tests / check-all / required CI、Ready / Not Ready判定。

## Authority / output routing

authorityは[Product Vision](product/vision.md)、[canonical specs](specs/README.md)、[GUI specs](gui/README.md)。D1〜D6のHuman decisionはspec-changeのapproval recordから各ownerへ適用する。

rewrite inputは[rewrite preparation](rewrite-preparation/README.md)、executable inputは独立corpusへrouteする。現行legacy capabilityを直ちに削除する要件へ変換しない。

## Explicit non-scope

production architecture / UI / CSS rewrite・refactor、current runtime性能最適化、framework / dependency replacement、新機能、clean-room implementation着手。
