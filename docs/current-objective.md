# Current Objective

## Role

この文書はHuman-selectedなcurrent priorityとDONE boundaryのownerである。observable semanticsはcanonical specification、resume地点は[Development State](execution-state.md)、implementation realityはcode / tests / Gitで確認する。

## Objective

**Browser / Web / Native Hostをactive product scopeからretireし、CLI / Tauri Desktop中心のrepository authority、architecture、implementation、verificationを整合させる。**

## Completion slices

- [仕様変更0022](spec-changes/0022-retire-web-product-hosts.md)を適用し、Product VisionとApproved Web requirementをretireする。歴史的なRFC / Applied recordはcurrent authorityと区別する。
- Browser / WASM / static Web専用のcode、build、test、CI、dependencyを削除する。Desktop / CLIに有用なshared Rust core改善を保持する。
- Desktop / CLI、migration、Build / Publishの回帰、repository checks、exact Candidateのremote CIを確認する。

## Explicit non-scope

- Reference、P5、released compatibility等の新しいproduct feature。
- Git history rewrite、force push、過去のApplied record削除。
- Desktop / CLIのobservable behavior、YAML / binary / config formatの変更。

## Audit

このObjectiveは2026-09-21 JSTにHumanが選択した。前ObjectiveのStandalone Webは[仕様変更0022](spec-changes/0022-retire-web-product-hosts.md)により中止した。
