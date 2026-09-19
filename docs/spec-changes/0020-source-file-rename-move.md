# 仕様変更: Source file rename / move

Status: Draft

## Affected Specifications

- new canonical owner candidate: `docs/specs/source-path-mutation.md`
  - new candidate IDs: `SOURCE-PATH-001..005`
- `docs/specs/project-layout.md` — `Status: Approved`
  - `PROJECT-006`, `PROJECT-CONVENTION-001`との整合を維持
- `docs/specs/source-creation.md` — `Status: Approved`
  - initial slice non-goalのrename / move boundary
- `docs/gui/explorer/spec.md` — `Status: Approved`
  - new candidate GUI source mutation requirements
- `docs/gui/app-shell.md` — `Status: Approved`
  - `GUI-SHELL-CAPABILITY-001`（Recovery Required中のsource rename / move禁止を既に含む）
- `docs/specs/runtime-hosts.md` — `Status: Approved`
  - host capability / workspace authority boundary

## 根拠と分類（Source Evidence and Classification）

- **Decision / Human priority**: 2026-09-19、P4の一部としてexisting source file rename / moveを次priorityに選択した。
- **Constraint / existing Approved contract**: source path、filename、directoryはTable identityを決定しない（`PROJECT-006`等）。rename / moveによってlogical Table / Type identityを暗黙に変更してはならない。
- **Constraint / existing Approved contract**: GUIはfilesystem mutationやpath safetyをfrontendへ実装せず、shared application / host boundaryを使用する。
- **Constraint / existing Approved contract**: Migration `Recovery Required`中はsource rename / moveを開始してはならない（`GUI-SHELL-CAPABILITY-001`）。
- **Constraint / existing safety**: source root外escape、symlink traversal等のpath safetyを弱めるchangeにはしない。
- **Open Question**: move先をsame source root内だけにするか、configured source roots間のmoveも許可するかは未決定。
- **Open Question**: dirty fileをmove可能にしてbuffer/pathをrebindするか、Save / Don't Save / Cancel等でclean stateへ解決してからmoveするかは未決定。
- **Open Question**: existing destination conflictでoverwriteを一切提供しないか、explicit overwriteを別authorizationとして許可するかは未決定。
- **Open Question**: case-only renameをinitial sliceでcross-platform supportするかは未決定。

## 提案する差分（Proposed Delta）

以下はHuman decisionでscopeが確定するまでDraft candidateであり、implementation authorityではない。

### SOURCE-PATH-001（candidate）

source file rename / moveは、existing canonical source fileのstorage pathだけを変更するsource mutation operationとして扱わなければならない（MUST）。source document bytesおよびTable / Type等のdeclared logical identityを、rename / moveだけを理由に変更してはならない（MUST NOT）。

source path、filename、directoryから新しいdomain identityまたはrename semanticsを導出してはならない（MUST NOT）。

### SOURCE-PATH-002（candidate）

rename / move requestのsourceとdestinationは、configured source root authorityとhost path safetyの内側でresolveしなければならない（MUST）。path traversal、source root escape、symlinkを通じたworkspace authority escapeを許可してはならない（MUST NOT）。

same-root / cross-rootの許可scopeはOpen Question 1のHuman decisionで確定する。

### SOURCE-PATH-003（candidate）

mutation開始前に、operationが対象としているsource fileのcurrent identityとdestination stateを再確認しなければならない（MUST）。preflight後にsourceまたはdestination stateが変わり、requested mutationを安全に同一operationとして確定できない場合は、stale/conflictとしてmutationを開始または継続してはならない（MUST NOT）。

destination overwriteの可否とauthorization modelはOpen Question 3のHuman decisionで確定する。

### SOURCE-PATH-004（candidate）

resultは少なくとも`Success`、mutation前に停止した`Conflict`、失敗が確定した`Failure`、final source/destination stateを安全に断定できない`Outcome Unknown`を観測上区別できなければならない（MUST）。

`Outcome Unknown`後はblind retryせず、source / destination actual stateを再取得してから次のmutationへ進まなければならない（MUST）。

### SOURCE-PATH-005（candidate）

rename / moveはBuild、Publish、schema/type Migration、Git stage / commit / pushを暗黙に開始してはならない（MUST NOT）。

frontendはfilesystem rename / move、path safety、lost-update / outcome classificationを再実装してはならず（MUST NOT）、shared application / host operationを使用しなければならない（MUST）。

### GUI candidate

Workspace ExplorerはApproved scopeのsource fileにrename / move actionを提供し、operation進行、Conflict / Failure / Outcome Unknown、Success後のselection/editor path更新を観測可能にする。

dirty bufferの扱い、destination chooser、case-only rename UIはOpen QuestionsのHuman decision後にnormative wordingへ確定する。

## 互換性（Compatibility）

- source file pathだけを変更するoperationであり、YAML content、declared Table / Type identity、MessagePack field key、generated API / binary formatを直接変更しない。
- relative source provenanceとopen editor pathは変更されるため、GUI selection、dirty buffer binding、diagnostic/source navigation、Recent state等のpath-bearing stateはoperation結果へ追随する必要がある。
- Git上ではrename detectionはGit側のpresentationであり、このoperationがGit stage / commit semanticsを所有しない。
- cross-root move、case-only rename、overwriteをどう扱うかでhost/platform compatibilityが変わるため、approval前に決定する。

## 受け入れと実装への影響（Acceptance and Implementation Impact）

scope確定後、少なくとも次をevidence化する。

- shared application serviceにsource path mutation use-caseを追加し、Tauri/frontendへfilesystem semanticsを複製しない。
- configured source root boundary、traversal / symlink escape、destination raceをmutation前にfail closedする。
- source bytesとdeclared logical identityをrename / move前後で保持する。
- Explorer selection、open clean editor、diagnostic/source provenanceをSuccess後のnew pathへrebindする。
- dirty buffer policyをHuman decisionどおりに検証し、local inputを暗黙に失わない。
- destination collision / external race / I/O failure / Outcome Unknownでunrelated sourceを変更しない。
- Recovery Required中にrename / moveが引き続きblockされる。
- source moveだけでBuild / Publish / Git operationを開始しない。

## 未解決事項（Open Questions）

1. **Destination scope**: same configured source root内だけか、configured source roots間も許可するか。
2. **Dirty file**: dirty bufferを保持したままpath rebindしてmoveできるようにするか、move前にSave / Don't Save / Cancelで解決するか。
3. **Destination conflict**: existing destinationへのoverwriteをinitial sliceで禁止するか、explicit overwrite authorizationを設けるか。
4. **Case-only rename**: `Foo.yaml -> foo.yaml`のようなcase-only renameをinitial sliceでTier 1 cross-platform contractとして要求するか。

folder rename / move、source delete / duplicateは現在のP4 selectionからは導出せずnon-scopeとする。

## レビュー（Review）

### Blocking Issues

- **Destination scope未決定**: same source root限定かconfigured roots間moveまで含むかでtransaction/failure modelが変わる。cross-rootはfilesystem boundaryを跨ぎ、単純renameと同じatomicityを仮定できない。
- **Dirty buffer policy未決定**: dirty stateのpath rebindを許すか、move前にSave / Don't Save / Cancelで解決するかでGUI/application lifecycleが変わる。
- **Destination conflict policy未決定**: overwrite禁止かexplicit overwrite許可かはdata-loss boundaryに直結し、implementation convenienceで選べない。
- **Case-only rename未決定**: Tier 1で必須supportするかによってplatform-specific rename strategyとacceptanceが変わる。

### Non-blocking Issues

- pathはdomain identityではないという`PROJECT-006`等との整合は取れている。
- `GUI-SHELL-CAPABILITY-001`がRecovery Required中のfuture rename / moveを既に禁止しているため、新surfaceも同じgateを再利用すべきである。
- source creationのpath-safety implementationは有用なcurrent evidenceだが、rename / moveのproduct contractを自動的に決めるauthorityではない。

### Questions

1. same-root onlyかcross-rootも許可するか。
2. dirty bufferをmoveと同時にrebindするか、clean-state guardを要求するか。
3. destination overwriteをinitial sliceで許可するか。
4. case-only renameをTier 1でinitial supportするか。

### Approved as Proposed

**No**。source pathをidentityにしないこと、shared application/host boundary、Recovery Required gateは整合するが、上記4点がobservable safety / compatibility behaviorを変えるためHuman decisionが必要。

Review dimensions: Intent fidelity=Pass、Internal consistency=Pass for Draft、Cross-spec consistency=Pass、Terminology=Pass、Normative strength=Pass for candidate wording、Testability=Pass after decisions、Backward compatibility=Needs decision for cross-root/case-only/overwrite、Unresolved ambiguity=Blocking 4件、Implementation leakage=None identified、Unrequested behavior=None identified。

## 承認記録（Approval Record）

未承認。
