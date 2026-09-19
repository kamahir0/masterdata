# 仕様変更: Source file rename / move

Status: Applied

## Affected Specifications

- new canonical owner candidate: `docs/specs/source-path-mutation.md`
  - new requirements: `SOURCE-PATH-001..007`
- `docs/specs/project-layout.md` — `Status: Approved`
  - `PROJECT-006`, `PROJECT-CONVENTION-001`との整合を維持
- `docs/specs/source-creation.md` — `Status: Approved`
  - initial slice non-goalのrename / moveをP4-B ownerへroute
- `docs/gui/explorer/spec.md` — `Status: Approved`
  - new requirements: `GUI-EXPLORER-STATE-004`, `GUI-EXPLORER-INT-004..006`, `GUI-EXPLORER-ERR-002`
  - future rename / move記述をP4-B ownerへroute
- `docs/gui/app-shell.md` — `Status: Approved`
  - `GUI-SHELL-CAPABILITY-001`のRecovery Required gateを維持
- `docs/specs/runtime-hosts.md` — `Status: Approved`
  - host capability / workspace authority boundaryを維持

## 根拠と分類（Source Evidence and Classification）

- **Decision / Human priority**: 2026-09-19、P4の一部としてexisting source file rename / moveを次priorityに選択した。
- **Decision / Human scope**: rename / moveは同じconfigured source root内に限定し、configured roots間moveをinitial scope外とする。
- **Decision / Human scope**: target source fileがdirtyなら、mutation前にSave / Don't Save / Cancelでそのfileのdirty stateを解決する。
- **Decision / Human scope**: existing destinationはConflictとし、initial scopeではOverwriteを提供しない。
- **Decision / Human scope**: case-only renameをsupported Desktop環境で扱えるcontractにする。
- **Constraint / existing Approved contract**: source path、filename、directoryはTable identityを決定しない（`PROJECT-006`等）。
- **Constraint / existing Approved contract**: GUIはfilesystem mutationやpath safetyをfrontendへ実装せず、shared application / host boundaryを使用する。
- **Constraint / existing Approved contract**: Migration `Recovery Required`中はsource rename / moveを開始してはならない（`GUI-SHELL-CAPABILITY-001`）。
- **Constraint / existing safety**: source root外escape、symlink traversal等のpath safetyを弱めない。

## Confirmed Decisions

- sourceとdestinationはsame configured source rootに属する。
- destinationはMasterdata sourceとしてdiscover可能な`.yaml` / `.yml` file pathに限る。
- target source fileのdirty bufferはrename / move開始前に解決し、unrelated dirty buffersは保持する。
- destination overwriteは提供しない。
- case-only renameを通常のrenameとしてsupportする。
- file path mutationだけを行い、source bytesやdeclared logical identityは変更しない。

## New Requirements

### SOURCE-PATH-001

source file rename / moveは、existing canonical source fileのstorage pathだけを変更するsource mutation operationでなければならない（MUST）。Success時のdestination source bytesはmutation直前のpreflightで確定したsource bytesとbyte-for-byte一致しなければならず（MUST）、Table / Type等のdeclared logical identity、YAML content、record order、comments、formattingをrename / moveだけを理由に変更してはならない（MUST NOT）。

source path、filename、directoryから新しいdomain identityまたはsemantic renameを導出してはならない（MUST NOT）。

### SOURCE-PATH-002

sourceとdestinationは同じconfigured source root内でresolveしなければならない（MUST）。configured source roots間move、source root外destination、absolute destination、path traversal、symlinkを通じたworkspace authority escapeをinitial P4-B operationとして許可してはならない（MUST NOT）。

destination fileはMasterdata source discoveryの対象として有効な`.yaml`または`.yml` pathでなければならない（MUST）。rename / moveによってsource fileを非source extensionへ暗黙に退避するoperationとして使用してはならない（MUST NOT）。destination parent folderはoperation開始前に存在しなければならず（MUST）、rename / moveがmissing folderを暗黙に作成してはならない（MUST NOT）。

### SOURCE-PATH-003

mutation開始直前に、operationが対象としているsource pathのcurrent source content identityとdestination entry stateを再確認しなければならない（MUST）。sourceがbase/preflight stateから変更・消失した場合、またはdestinationに別entryが存在する場合はConflictとしてmutationを開始してはならない（MUST NOT）。

initial P4-Bではdestination Overwriteを提供してはならず（MUST NOT）、existing destinationを削除、truncate、merge、replaceして成功扱いしてはならない（MUST NOT）。

case-only renameでは、host上でsource自身がdestination lookupにも現れることだけを「別destinationが存在する」Conflictとして扱ってはならない（MUST NOT）。sourceと別entryがdestinationを占有する場合は通常のConflictとする。

### SOURCE-PATH-004

case-only rename（例: `Foo.yaml`から`foo.yaml`）は、source file rename / move capabilityを提供するhostで通常のrename requestとして扱えなければならない（MUST）。case-insensitive filesystemであることだけを理由にunsupportedまたはno-opとして扱ってはならない（MUST NOT）。

exact intermediate path、host primitive、temporary name等のmechanismはimplementation detailとし、利用者が要求していないtemporary source entryをSuccess後に残してはならない（MUST NOT）。

### SOURCE-PATH-005

resultは少なくとも`Success`、`Conflict`、`Failure`、`Outcome Unknown`を観測上区別できなければならない（MUST）。

- `Success`: workspace listing / host directory stateがrequested destination path（case-only renameではrequested destination spellingを含む）をcurrent source locationとして示し、destinationにpreflight-confirmed complete source bytesが存在し、sourceとは別のold entryが残っていない。
- `Conflict`: source/destination preflightによりmutation開始前に停止し、source entryとdestination entryを変更していない。
- `Failure`: requested path mutationが成功しなかったことを確定でき、preflight-confirmed sourceがold locationにcompleteなまま存在し、distinct destination entryを作成・変更していないことを確認できる。
- `Outcome Unknown`: mutation開始後のhost/I/O failure等によりold source / destinationの最終状態を上記`Success`または`Failure`として安全に断定できない。

`Outcome Unknown`後はblind retryしてはならず（MUST NOT）、old source pathとdestinationのactual stateを再取得してから次のmutationへ進まなければならない（MUST）。

### SOURCE-PATH-006

rename / moveはBuild、Publish、schema/type Migration、Git stage / commit / pushを暗黙に開始してはならない（MUST NOT）。source file path mutation、path safety、preflight、result classificationをTauri/React frontendへ再実装してはならず（MUST NOT）、shared application / host operationを使用しなければならない（MUST）。

Migration `Recovery Required`中は`GUI-SHELL-CAPABILITY-001`に従いsource rename / moveを開始してはならない（MUST NOT）。

### SOURCE-PATH-007

source file rename / move自体はdirty local bufferを移送またはpath-rebindするoperationであってはならない（MUST NOT）。callerがtarget source fileにdirty bufferを保持している場合、mutation開始前にそのbufferをSaveまたは明示Discardしてclean stateへ解決するか、operationをCancelしなければならない（MUST）。

unrelated source fileのdirty bufferをrename / moveのためにSave、Discard、またはblockしてはならない（MUST NOT）。

## Changed Requirements

### GUI-EXPLORER-STATE-004（new）

rename / move実行中、Explorerはtarget source fileのpath mutationが進行中であることを識別可能にしなければならない（MUST）。Success前にdestination pathを確定したcurrent sourceとして編集可能に表示してはならない（MUST NOT）。

Success後はworkspaceをshared application authorityから更新し、selected/open target sourceをnew pathへ追従させなければならない（MUST）。old pathのclean editor snapshotをcurrent editable sourceとして残してはならず（MUST NOT）、unrelated dirty bufferは保持しなければならない（MUST）。

### GUI-EXPLORER-INT-004（new）

Workspace Explorerはexisting Masterdata source fileに対するRename / Move actionを提供しなければならない（MUST）。destinationはsame configured source root内のfolder / filenameとして指定できなければならず（MUST）、source root間moveをinitial P4-B UIから実行可能にしてはならない（MUST NOT）。

frontendはfolder配置またはfilenameからTable / Type identity changeを推測してはならない（MUST NOT）。

### GUI-EXPLORER-INT-005（new）

target source fileがdirtyな場合、Rename / Move開始前に`Save` / `Don't Save` / `Cancel`を選択できなければならない（MUST）。

- `Save`: target fileだけを通常Saveし、Successした場合だけRename / Moveへ進む。Conflict / Failure / Outcome Unknownではpath mutationへ進まずlocal bufferを保持する。
- `Don't Save`: target fileのlocal dirty bufferを明示的に破棄してclean workspace sourceをrename / move対象とする。
- `Cancel`: path mutationを開始せずlocal bufferを保持する。

unrelated dirty filesへこの確認を適用してはならず（MUST NOT）、それらを暗黙Save / Discardしてはならない（MUST NOT）。

### GUI-EXPLORER-INT-006（new）

Rename / Move成功後も、source documentはdocument kindとdeclared domain identityに従う同じtyped editorへrouteされなければならない（MUST）。case-only renameも通常のSuccess lifecycleを使用しなければならない（MUST）。

destination ConflictではOverwrite actionを提示してはならず（MUST NOT）、destination変更またはCancelへ戻れるようにしなければならない（MUST）。

### GUI-EXPLORER-ERR-002（new）

`Conflict`、`Failure`、`Outcome Unknown`をSuccessとして表示してはならない（MUST NOT）。Outcome Unknownではold/new両pathのactual stateをRecheck / Reloadできるrecovery pathを提示し、recheck前に同じRename / Moveを自動retryしてはならない（MUST NOT）。

Failure / Conflictによってunrelated dirty buffer、Explorer state、別source fileを不必要に破壊してはならない（MUST NOT）。

### Existing non-goal routing

`docs/specs/source-creation.md`と`docs/gui/explorer/spec.md`のinitial-slice proseにあるrename / move非目標は、P4-B適用後は本source-path mutation ownerへrouteする。source file delete / duplicate、folder rename / moveは引き続きP4-B非目標とする。

## Open Questions

None identified for P4-B initial scope.

## Potential ADRs

None identified. filesystem/domain responsibility、shared Rust application boundary、host capability modelは既存ADR/specを維持する。case-only renameの具体mechanismはarchitecture decisionではなくhost implementation detailとする。

## Compatibility Impact

- source file pathだけを変更し、YAML content、declared Table / Type identity、MessagePack field key、generated API / binary formatを直接変更しない。
- source-relative provenance、Explorer selection、open editor path、diagnostic/source navigation等のpath-bearing runtime stateはSuccess後にnew pathへ追従する。
- Git上のrename detection、stage / commit behaviorはGit側のpresentationであり、本operationのcontractではない。
- configured roots間move、destination overwrite、folder mutationはinitial scope外のため、cross-filesystem transactionやdestructive replacement compatibilityは導入しない。
- case-only renameをcapabilityのあるsupported Desktop環境で一貫して扱うため、case-insensitive filesystemでもuser-visibleに同じoperationを提供する。

## Implementation Impact

- shared application service: source path mutation request / preflight / result / recheckを追加する。
- native host boundary: same-root path safety、no-follow traversal、destination existence race、case-only renameを安全に実行する。
- Tauri adapter: typed request/resultを薄くtransportし、filesystem semanticsを持たない。
- React / Explorer: Rename / Move action、same-root destination selection、target-only dirty resolution、progress/result/recovery、Success後rebindを実装する。
- Tests: same-folder rename、same-root folder move、`.yaml <-> .yml` rename、cross-root rejection、non-source extension rejection、destination Conflict/no overwrite、source/destination race、case-only rename、dirty Save/Don't Save/Cancel、unrelated dirty preservation、Recovery Required gate、Outcome Unknown recheckをfocused evidenceにする。
- Build / Publish / .NET adapter、Git integrationは変更対象外。

## レビュー（Review）

### Blocking Issues

None identified.

Fresh reviewでは、初版Proposedの`SOURCE-PATH-005`がcase-insensitive filesystemでcase-only renameのSuccessを「old pathが存在しない」と判定しており、G1と両立しない問題を検出した。Successをdirectory/workspace上のrequested destination spellingとdistinct old entryの有無で定義し直し、解消した。

また、known `Failure`のpostconditionが弱くpartial/destructive stateをFailureとして確定できる余地があったため、old source complete + distinct destination unchangedを確認できる場合だけFailureとし、それ以外をOutcome Unknownへ送るよう明確化した。これは新しいproduct choiceではなく、選択済みno-overwrite/data-safety boundaryをtestableにしたもの。

### Non-blocking Issues

- case-only renameのtemporary/intermediate path strategyはhost/filesystem依存であり、specificationはobservable final stateだけを固定している。
- source path mutationは新canonical ownerを必要とするが、既存Runtime Host / Project identity architectureを変更しないため新ADRは不要。
- invalid YAML等のsemantic validityをrename/move preconditionにするruleは追加していない。P4-Bはpath mutation contractであり、semantic validationの新gateを発明しない。

### Questions

None identified.

### Approved as Proposed

**Yes**。Human-selected D1 + E1 + F1 + G1を反映し、same-root/no-overwrite、dirty pre-resolution、case-only behavior、Outcome Unknown recoveryがtestableなdata-safety contractとして閉じている。Human Approvalとcanonical applicationはまだ必要。

Review dimensions: Intent fidelity=Pass、Internal consistency=Pass、Cross-spec consistency=Pass、Terminology=Pass、Normative strength=Pass、Testability=Pass、Backward compatibility=Pass within selected scope、Unresolved ambiguity=None、Implementation leakage=None identified、Unrequested behavior=None identified。

## 承認記録（Approval Record）

- Human approval: 2026-09-20 JST
- Canonical application commit: `5ce8e0e8154e3d4f152c883f88bac34537d89ca6` — added the Approved `docs/specs/source-path-mutation.md` owner and applied the Explorer / Source Creation routing delta.
- Implementation authority: canonical Approved specifications only; this artifact remains an audit record.
