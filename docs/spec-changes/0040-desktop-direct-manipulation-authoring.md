# 仕様変更: Desktop direct-manipulation authoring

Status: Applied

## Affected Specifications

- `docs/gui/source-creation/spec.md` — creation layout/state/interaction/keyboard/focus
- `docs/gui/explorer/spec.md` — provisional creation itemとinline commit/cancel
- `docs/gui/table-editor/spec.md` — spreadsheet-first schema editing、hidden routine Plan、direct type change
- `docs/gui/data-editor/spec.md` — schema-aware header integration、row virtualization
- `docs/gui/data-editor/grid-authoring.md` — paste/fillのmandatory preview撤廃
- `docs/gui/data-editor/record-mutation.md` — grid bottom Add Row、Undo/Redo整合
- `docs/specs/schema-migration.md` — `ChangeFieldType` semantic operation

## 根拠と分類（Source Evidence and Classification）

- Human Decision: backendの堅牢性・慎重さ・重苦しさをGUIへ持ち込まず、Desktop UI/UXは純粋に使いやすさ・操作性を追求する。
- Human Requirement: record editorはExcel / Google Sheetsに近いtable editingとし、column headerでname / typeを直接変更、右端`+`でcolumn追加、最下部`+`でrecord追加する。
- Human Requirement: 大規模MasterDataに備えてvirtualizationを導入する。
- Human Requirement: 新規file作成はkind選択後にExplorerへprovisional nodeを作り、filenameをinline入力、Enterで作成、Escapeでcancelする。
- Human Decision: 差し当たっての次Objectiveはfrontendの大改修・再考である。
- Existing Authority: YAML/domain semantics、Migration safety、source-preserving commit、Conflict / Outcome Unknown / Recovery Requiredはshared Rust core/applicationが所有する。
- Agent Decision: GUIはuser intentをapplication operationへ翻訳する層とし、routine Plan / preflightを内部実行して、destructive / blocked / recoveryだけを前景化する。
- Agent Decision: `ChangeFieldType`はv1でvalue coercionを行わず、current valuesとdependenciesがtarget typeですでにlossless validな場合だけschema type tokenを変更する。変換UIは後続scopeとする。
- Agent Decision: starter sourceのcanonical default構成は`masterdata-app`側に置き、ReactでYAML/domain defaultを生成しない。

## 提案する差分（Proposed Delta）

- creationをmodal-firstからExplorer inline creationへ変更し、artifact-specific detailは作成後のtyped editorへ移す。
- creation-time filename stemからのidentity推論をstarter defaultとしてのみ許可し、作成後のpath / domain identity独立性は維持する。
- Table/Data editorをspreadsheet-firstへ統合し、field name/typeとfield/record追加を対象位置でdirect操作する。
- Migration Planはsource mutation前に必須のまま、non-destructive daily actionではmandatory visible stepにしない。Diff/Detailsは任意、destructive / stale / dirty dependency / recoveryは明示する。
- Data gridはrow virtualizationを要求し、virtualizationによってselection / focus / history semanticsを変えない。
- paste/fill/range mutationはshared applicationのall-or-none candidateをそのままlocal bufferへ1 Undo unitで反映し、mandatory preview / Apply clickを廃止する。
- Schema Migrationへ`ChangeFieldType`を追加し、coercionなし・dependency再検証・source-preserving type-only patchをcontract化する。

## 互換性（Compatibility）

YAML source format、persisted identity、CLI existing command behavior、Build / Publish、generated binary formatは変更しない。GUI操作手順とshared Migration capabilityはadditive / replacementであり、既存sourceをmigrationなしに読み書きできる。

`ChangeFieldType`は新しいexplicit authoring operationであり、既存operationの意味を弱めない。value coercionを行わないため、既存source valueを暗黙変換またはlossさせない。

## 受け入れと実装への影響（Acceptance and Implementation Impact）

- New -> Table -> inline filename -> Enterだけでvalid starter Tableが作成され、Escapeでは何も作成されない。
- creation conflict/failure/outcome-unknownでも既存dirty bufferを失わず、blind overwrite/retryを行わない。
- field header rename/type selection、rightmost Add Field、bottom Add Rowをmodalなしで日常操作できる。
- Rename/Add/ChangeTypeはshared Plan / commit safetyを通るが、安全なnon-destructive caseでPlan/Apply UIを必須にしない。
- ChangeFieldTypeはcurrent valuesまたはkey/reference dependencyがtarget typeに不適合ならsourceを変更せずlocalized diagnosticを返す。
- 10k+相当recordsでもrendered row数がviewport近傍にboundedで、keyboard / range / focus / Added/Pending stateが正しく動く。
- TSV pasteは通常操作でlocal bufferへ直接反映され1 Undoで戻り、shape/read-only/stale failureではbuffer不変。
- frontendへYAML parser、type validation、migration patch、starter domain semanticsを複製しない。

## 未解決事項（Open Questions）

None. exact component library、virtualization overscan、column width、popover geometry、starter cosmetic namingはobservable contractを変えない範囲でimplementation detail。

## レビュー（Review）

### Blocking Issues
None identified.

### Non-blocking Issues
- full Type Editorのdirect-manipulation redesignは今回の第一マイルストーン外であり、starter creation後は既存Type Editorへrouteする。
- type conversionを伴うChangeFieldTypeは後続capabilityとし、本changeではlossless-valid existing representationだけを許可する。

### Questions
None identified.

### Approved as Proposed
Yes.

### Autonomous approval eligibility
- Eligible: Yes
- Human gate: None
- Rationale: Humanがfrontend overhaul、direct creation、spreadsheet editing、virtualization、backend safetyをGUIへ露出しない方針を明示選択している。persisted format / stable identity / public compatibilityを破壊せず、new operationはadditiveかつfail-closedでtest可能。

### Review dimensions
- Intent fidelity: Humanが示したLegacy寄りのdirect操作とbackend/GUI分離をそのまま反映。
- Internal consistency: source safety / dirty buffer / recoveryは維持し、presentationだけでなく不足shared capabilityをapplication/coreへroute。
- Cross-spec consistency: Explorer/Creation/Table/Data/Grid/Migrationのowner境界を維持。
- Normative strength: direct interactionとsafety boundaryはMUST、exact component/layout調整値はimplementation detail。
- Testability: keyboard、Enter/Escape、bounded rendering、direct paste、migration success/failureをfocused test化可能。
- Backward compatibility: source format / identity / Build / Publishは不変。
- Unresolved ambiguity: materialなものなし。
- Implementation leakage: `@tanstack/react-virtual`等のlibrary選択はcanonical specへ固定していない。
- Documentation ownership: Current ObjectiveはWHAT、各specはobservable contract、code/testsはimplementation reality。

## 承認記録（Approval Record）

- Approval mode: Agent-autonomous
- Basis: Human-selected frontend overhaul objective and explicit direct-manipulation UX decisions
- Review result: Blockingなし、material ambiguityなし、Human gateなし、non-breaking / additive
- Canonical application: Source Creation, Explorer, Table Editor, Data Editor, Grid Authoring, Record Mutation, Schema Migration v1
