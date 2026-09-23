# Current Objective

## Role

この文書はHuman-selectedなcurrent priorityとDONE boundaryのownerである。observable semanticsはcanonical specification、resume地点は[Development State](execution-state.md)、implementation realityはcode / tests / Gitで確認する。

## Objective

**Product Simplification & Scope Cleanupを完了し、新機能へ進む前に、現在のproduct scope・canonical specification・implementationを根源的な用途へ照らして再監査し、不要・過剰・誤って昇格した機能をretireして、保守すべきsurfaceを意図的に小さくする。**

## Completion slices

### Full-scope audit

- activeなproduct capability、canonical specification、CLI/Tauri/GUI surface、tests/fixtures、CI/dependencyを横断し、各領域を **core / retained-but-deferred / retire** に分類する。
- 「既に実装済みだから残す」を理由にせず、根源的な利用価値、保守コスト、conceptual complexity、他機能とのcouplingから判断する。
- Humanが選択していないObjectiveや、Deferred/Ideaからagentが過度に具体化したsurfaceを特に監査する。

### Known direction

- Git-native Collaboration & Automationは次Objectiveとして進めない。仕様変更0029はRejectedとし、Git client / stage / commit / push / PR等をproduct scopeへ追加しない。
- Released Compatibilityは独立product capabilityとしてretireする方向とし、baseline/current snapshot comparison、4-axis compatibility classification、compatibility CLI/GUI等をcleanup対象とする。
- Programmable Viewという将来要件自体は保持するが優先度は低い。現在の独自DSL型Computed Viewを将来設計の最終形として固定せず、このCleanup中に新機能・DSL拡張を行わない。将来再開時に汎用language/runtimeを含めて再設計する。
- source-preserving edit、lost-update protection、migration operation自身のcorrectness、Build/Publish safety等、Compatibility機能とは独立したcore safety invariantはretirementの巻き添えにしない。

### Retirement execution

- retire対象はproduction codeだけでなく、public command/adapter、GUI、canonical requirement、tests/fixtures、docs routing、CI/dependency、stale examplesまでdependency closureを追って削除する。
- 未release/不要surfaceを「将来使うかもしれない」だけでcompatibility shim、dead abstraction、disabled UI、placeholderとして残さない。
- historical rationaleが必要なものはcompact audit recordまたはGit historyへ寄せ、current authorityにretired behaviorを残さない。
- 削除によってより単純なownership boundaryへ戻せる場合は、retired subsystem専用のDTO/service/adapterを合わせて除去する。

### Verification

- cleanup後にProject discovery、YAML authoring、Table/Type/Data editing、Reference等のretained domain semantics、Migration、Build/Publish、Unity deliveryの主要core workflowがregressしていないことを確認する。
- repository checks、fresh review、exact Candidate、required remote CI reconciliationまで完了する。
- cleanup結果として残るcurrent product surfaceをREADME / Product Vision / specification indexから復元可能にする。

## Explicit non-scope

- cleanup中に新しいproduct featureを追加すること。
- Programmable Viewの次期runtime/language設計をこのObjectiveで完成させること。
- Git integrationの代替実装。
- Released Compatibilityの後継となる別名のimpact-analysis subsystemを作ること。
- Web / Browser product surfaceの再導入。
- cleanupを理由にretained core semanticsやsource safetyを全面再設計すること。

## Audit

2026-09-23 JST、Humanは次の新機能へ進む前に、十分なコストを掛けて不要な機能・仕様・実装を徹底的に除去し、productを一度小さく綺麗にすることをCurrent Objectiveとして選択した。直前のGit-native Collaboration & AutomationはHuman-selected priorityではなかったため進行を中止する。会話上の方向として、Git-nativeはretire、Released Compatibilityは独立機能として全retire、Programmable Viewの要件は将来向けに保持しつつ現行Computed View設計は凍結・再設計対象とする。
