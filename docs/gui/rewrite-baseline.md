# Desktop rewrite baseline / transition

Status: Approved

このownerは0051のHuman D1〜D5によるsurface scopeを定義する。domain semantics、Save、Conflict、Migration、Build inclusionは各canonical ownerに残る。

### GUI-BASELINE-001

ordinary Table authoringはFind/Search、direct schema/value edit、copy/paste、Add/Delete/Reorder、source-local Undo/Redo、Problems、Saveをbaselineとする。capabilityの数をpersistent controlsの数へ写像しない。exact toolbar / component treeを固定しない。

standalone Diff、Typed Filter builder、View Sort、Advanced Batch surface（Fill / range Set Null専用workflow / persistent preview）、standalone Saved Table Overviewはclean implementation baselineに含めない。具体的なHuman-selected daily needが出ればcontextual toolとして再評価できる。

### GUI-BASELINE-002

Compareはunsaved Save candidate、Conflict、Migration / destructive structural operationで対象、before/after、affected physical sourcesを理解できるようにする（MUST）。dirty / historyを失わず到達可能にする。saved-source Git diff / history / review / staging / merge / revertはGit / IDE ecosystemへ委譲する。

### GUI-BASELINE-003

利用者がcurrent Tableのphysical sources、cross-source composition、saved/dirtyの区別、Build Profile inclusion / tags / selectionを理解する必要がある場合、関連Table / Build文脈から必要な詳細へ到達できる（MUST）。standalone destinationを要求しない。表示・inspectionだけでSave / Build / Publishを行わない。

### GUI-BASELINE-004

Search / Findはlarge Tableのmatching record/valueへ移動可能にし、source bytesを変更しない（MUST）。Search / selection stateはsource-localに保ち、dirty semanticsへ混入しない。default presentationはsource record order / schema declaration order。Build orderingは別ownerであり、viewから変更しない。

### GUI-BASELINE-005

current productionにlegacy capabilityが残ることを許容する。このtransitionは「must exist」の解除であり「must not exist today」ではない。Deprecated GUI OverviewとAuthoring Queryのlegacy filter / sort clauses、旧Batch preview testsはcurrent implementation regressionとしてのみ読む。clean implementationのacceptance gateにはしない。

基礎契約は[Data Editor](data-editor/spec.md)、[Grid Authoring](data-editor/grid-authoring.md)、[Authoring Batch](../specs/authoring-batch.md)、[Authoring Query](../specs/authoring-query.md)、[Build Selection](../specs/build-selection.md)。
