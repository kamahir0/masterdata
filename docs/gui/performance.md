# Interactive performance contract

Status: Approved

### GUI-PERF-001

通常の同一Project warm navigation critical pathにproject-wide discovery / enumeration / YAML parse / validationを含めない（MUST NOT、各count = 0）。changed source / required dependencyの局所freshness / reparse / projectionは許容する。selectionは開いたProjectのview selectionでありreopenではない。重い処理をparallel full rebuildへ移すだけでは満たさない。

### GUI-PERF-002

selectionのtarget identityをbackend completionを待たず原則next frameで反映する。old contentをnew targetとして表示しない（MUST NOT）。heavy read / validationがnative UI interactionをblockしない。obsolete result / diagnostics / focus intentをcurrentへ適用しない（MUST NOT）。latest selection wins、pending workとDOM/renderはboundedとする。

### GUI-PERF-003

reference input 3 Tables / 8 sources / 12,000 records（selected 2,000 × 20、inline / separate / mixed）のrelease Tier1測定では、feedback next-frame（目安16–33ms）、warm backend derivation p95 < 50ms級、selection → first accepted interaction p95 < 150ms、stretch < 100msをrewrite targetとする。現行implementationのpassを主張しない。hardware差を無視したCI wall-time thresholdではなく、各caseのdistributionとabsolute gapを評価する。

### GUI-PERF-004

cold open、first projection、revisit、same Table、cross Table、schema selection、dirty revisit、rapid selectionを分離する（MUST）。warm各caseは原則100 samples以上・複数runsでmedian / p95 / maxを記録する。native、serialization、IPC、state publication、React commit、layout、target-filtered rAF opportunity、first accepted interaction、WebDriver wallを同一指標としない。single sampleをp95として報告しない。

### GUI-PERF-005

100,000 records / 10 files / 20 columns / 10,000 paste cellsのcapacity oracleは正しいresult / paste targets / diagnostics、stage latency、peak RSS、bounded renderingを検証する。one-shot harnessのrequest topologyや毎navigation全量transferを要件にしない。semantic datasetとUI projectionを分離可能にし、測定なしにpagination protocolを必須化しない。

fresh write preflight、cache freshness、dirty overlay ownershipは[Source Edit](../specs/source-edit.md) / [Explorer](explorer/spec.md)のsafety boundaryを維持する。測定protocolとevidence gapは[rewrite performance](../rewrite-preparation/performance.md)へrouteする。
