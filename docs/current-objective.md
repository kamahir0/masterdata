# Current Objective

## Role

この文書はHuman-selectedなcurrent priorityとDONE boundaryのownerである。observable semanticsはcanonical specification、resume地点は[Development State](execution-state.md)、implementation realityはcode / tests / Gitで確認する。

## Objective

**Released Compatibility v1を定義し、2つのexplicitなMasterdata project snapshot間のschema evolutionを比較して、generated API・source migration・artifact/binary・external contractを混同せずcompatibility impactを判定できるshared semanticsへ到達する。**

## Completion slices

### Compatibility model

- current-schema semanticsとreleased compatibility semanticsを分離し、既存Approved Table / Type / Index / Reference contractを変更しない。
- Table、field、type、Enum / Flags、Primary / Secondary Key、Reference、generated C# presentationの変更について、どのcompatibility axisへ影響するかを定義する。
- MessagePack `key`、Secondary `indexNo`、Reference `csharpName`等を、既存authorityに反してstable identityへ昇格させない。
- compatibility判定のbaseline/current input、version metadataとの関係、unknown/unsupported changeのfail-closed behaviorを確定する。

### Shared analysis / product surface

- compatibility comparison semanticsをshared Rust core/applicationへ置き、CLI / Desktopがdomain ruleを複製しない。
- machine-actionableなstructured change reportを生成できるようにする。
- exact public CLI / Desktop surfaceは、Human decisionで選択したcompatibility scopeから必要になる範囲だけ定義する。

### Verification

- representative schema evolution matrixでcompatible / breaking / review-required等の判定をfocused evidence化する。
- repository checks、fresh review、exact Candidateのrequired remote CI reconciliationを完了する。

## Explicit non-scope

Human decisionで明示的にscopeへ入れない限り、以下はv1へ含めない。

- external save data / network protocol / external databaseのwire compatibility保証。
- cross-schema MasterMemory binary reader compatibilityや旧binaryを新generated C#で読む保証。
- global stable Table / Field / Enum / Reference IDの新設。
- automatic Reference-aware migrationやarbitrary migration scripting。
- semantic versionの自動bump、release publication、Git tag作成。
- artifact-set receiptをreleased compatibility identityへ昇格させること。

## Audit

2026-09-21 JST、Reference v1 Objective完了後、Humanがreleased compatibility / schema evolution方向へ進むことを選択した。仕様変更0024ではHumanがOption Aを明示採用し、explicit baseline/current canonical project snapshot comparisonとmulti-axis reportをv1 boundaryとした。cross-schema MasterMemory binary compatibility、external save/network/database wire contract、persistent release identity / stable member IDはv1非対象とする。
