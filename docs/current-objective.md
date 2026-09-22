# Current Objective

## Role

この文書はHuman-selectedなcurrent priorityとDONE boundaryのownerである。observable semanticsはcanonical specification、resume地点は[Development State](execution-state.md)、implementation realityはcode / tests / Gitで確認する。

## Objective

**Advanced Authoringをproduction-readyにし、保存済みMasterdata snapshotから安全にderived informationを作るexpression / computed viewを、Git-reviewableなdefinition、shared Rust semantics、Desktop authoring/query workflowまで一貫して利用できる状態へ到達する。**

## Completion slices

### Computed view semantics

- Authoring system RFCでDeferredとなっているP5 expression / computed viewをspecifyし、canonical persisted definition、logical identity、target Tableとの関係、expression type checking、null / invalid propagation、deterministic evaluationを定義する。
- computed viewはauthoring/read-only derived projectionとして扱い、canonical Table field、MessagePack field、MasterMemory schema、generated C# public field、binary persisted valueへ暗黙昇格させない。
- arbitrary user code、filesystem/network access、time/random/environment dependence、mutation、side effectをexpression semanticsへ導入しない。
- Reference、Value Object、Enum / Flags、Custom Type等の既存shared semanticsを再利用し、frontend独自expression semanticsを作らない。

### Authoring workflow

- saved canonical source snapshotからshared Rust evaluatorがcomputed columns / values / diagnosticsを生成する。
- Table Overviewを中心にcomputed valuesをread-only表示し、適切な型でsearch / filter / sortへcompositionできる範囲をspecify / implementする。
- computed definitionのcreate/edit/removeをDesktopからsource-preservingかつstale-safeに行えるようにし、Git diffでdefinition changeをreviewできる。
- invalid expressionやdependency failureをraw source valueや0/nullへ黙ってcoerceせず、structured diagnosticとUnavailable/invalid stateを明示する。

### Product hardening

- expression dependency cycle、unknown field/type/reference、nullable/invalid input、overflow / invalid operation等をdeterministicにfail closedまたはtyped invalid resultとして扱う。
- same saved snapshot + same definitionsから同じderived result / orderingを生成する。
- current source/data mutation、Migration、Build、Publish、released compatibilityとの境界を明確にし、computed view authoringからそれらを暗黙実行しない。
- focused regression、fresh review、repository checks、exact Candidate、required remote CI reconciliationまで完了する。

## Directional boundary

Authoring system RFCのP5表現（expression / computed view）と既存architectureから、v1のdefault directionは**authoring-only derived view**とする。generated C# / MasterMemory binaryへcomputed fieldを追加するruntime featureへscopeを広げない。

definitionはHuman / AIがGitでreviewできるcanonical sourceとして永続化する方向をdefaultとする。exact YAML document shape、expression grammar、view identity、supported operator matrixはspecification refinementで決定する。複数のmaterially different product choiceが残る場合だけHuman gateへ戻す。

## Explicit non-scope

- generated C# / MasterMemory schemaへcomputed field/propertyを追加すること。
- Unity/runtimeでexpression evaluatorを実行すること。
- arbitrary scripting、C# / JavaScript / Lua等のembedded user code。
- filesystem / network / process / environment / clock / randomへのaccess。
- expressionからsource mutation、Migration、Build、Publishを開始すること。
- aggregate / join / group-by / cross-project queryを、single-row computed view semanticsが固まる前に一般purpose query engineとして導入すること。
- persistent stable member ID、cross-schema binary compatibility、external wire compatibility。
- Web / Browser product surfaceの再導入。

## Audit

2026-09-22 JST、Humanは前Objective完了後に次へ進むことを選択し、コード編集を伴う実装はimplementation agentへ指示書で委譲する運用を指定した。Authoring system RFCでDeferredとなっていたP5 expression / computed viewを中心に、Advanced Authoringを大きなproduct outcome単位で開始する。
