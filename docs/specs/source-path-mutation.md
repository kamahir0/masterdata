# Source Path Mutation仕様

Status: Approved

Domain: Source Editing

## 概要

本仕様は、Projectのconfigured source root内にあるexisting Masterdata source fileをrename / moveするためのobservable contractを定義する。

source file pathはstorage / provenanceであり、Table / Type等のdomain identityではない。[Project layout](project-layout.md)のidentity contract、[Runtime hosts](runtime-hosts.md)のhost capability boundary、[GUI app shell](../gui/app-shell.md)のRecovery Required gateを維持する。record value編集、Source Creation、Schema / Type Migration、Build / Publish、Git operationはそれぞれのowner specificationが所有する。

## 用語

- **Source path mutation**: existing source fileのstorage pathだけを変更するrename / move operation。
- **Source path**: configured source root内のproject-relative storage location。domain identityではない。
- **Conflict**: mutation開始前のsource / destination preflightにより安全にoperationを開始できず停止した状態。
- **Outcome Unknown**: mutation開始後のhost / I/O failure等によりold source / destinationのfinal stateを安全に断定できない状態。

## 規範要件

### SOURCE-PATH-001

source file rename / moveは、existing canonical source fileのstorage pathだけを変更するsource mutation operationでなければならない（MUST）。Success時のdestination source bytesはmutation直前のpreflightで確定したsource bytesとbyte-for-byte一致しなければならず（MUST）、Table / Type等のdeclared logical identity、YAML content、record order、comments、formattingをrename / moveだけを理由に変更してはならない（MUST NOT）。

source path、filename、directoryから新しいdomain identityまたはsemantic renameを導出してはならない（MUST NOT）。

### SOURCE-PATH-002

sourceとdestinationは同じconfigured source root内でresolveしなければならない（MUST）。configured source roots間move、source root外destination、absolute destination、path traversal、symlinkを通じたworkspace authority escapeをinitial operationとして許可してはならない（MUST NOT）。

destination fileはMasterdata source discoveryの対象として有効な`.yaml`または`.yml` pathでなければならない（MUST）。rename / moveによってsource fileを非-source extensionへ暗黙に退避するoperationとして使用してはならない（MUST NOT）。destination parent folderはoperation開始前に存在しなければならず（MUST）、rename / moveがmissing folderを暗黙に作成してはならない（MUST NOT）。

### SOURCE-PATH-003

mutation開始直前に、operationが対象としているsource pathのcurrent source content identityとdestination entry stateを再確認しなければならない（MUST）。sourceがbase / preflight stateから変更・消失した場合、またはdestinationに別entryが存在する場合はConflictとしてmutationを開始してはならない（MUST NOT）。

destination Overwriteを提供してはならず（MUST NOT）、existing destinationを削除、truncate、merge、replaceして成功扱いしてはならない（MUST NOT）。

case-only renameでは、host上でsource自身がdestination lookupにも現れることだけを「別destinationが存在する」Conflictとして扱ってはならない（MUST NOT）。sourceと別entryがdestinationを占有する場合は通常のConflictとする。

### SOURCE-PATH-004

case-only rename（例: `Foo.yaml`から`foo.yaml`）は、source file rename / move capabilityを提供するhostで通常のrename requestとして扱えなければならない（MUST）。case-insensitive filesystemであることだけを理由にunsupportedまたはno-opとして扱ってはならない（MUST NOT）。

exact intermediate path、host primitive、temporary name等のmechanismはimplementation detailとし、利用者が要求していないtemporary source entryをSuccess後に残してはならない（MUST NOT）。

### SOURCE-PATH-005

resultは少なくとも`Success`、`Conflict`、`Failure`、`Outcome Unknown`を観測上区別できなければならない（MUST）。

- `Success`: workspace listing / host directory stateがrequested destination path（case-only renameではrequested destination spellingを含む）をcurrent source locationとして示し、destinationにpreflight-confirmed complete source bytesが存在し、sourceとは別のold entryが残っていない。
- `Conflict`: source / destination preflightによりmutation開始前に停止し、source entryとdestination entryを変更していない。
- `Failure`: requested path mutationが成功しなかったことを確定でき、preflight-confirmed sourceがold locationにcompleteなまま存在し、distinct destination entryを作成・変更していないことを確認できる。
- `Outcome Unknown`: mutation開始後のhost / I/O failure等によりold source / destinationの最終状態を上記`Success`または`Failure`として安全に断定できない。

`Outcome Unknown`後はblind retryしてはならず（MUST NOT）、old source pathとdestinationのactual stateを再取得してから次のmutationへ進まなければならない（MUST）。

### SOURCE-PATH-006

rename / moveはBuild、Publish、Schema / Type Migration、Git stage / commit / pushを暗黙に開始してはならない（MUST NOT）。source file path mutation、path safety、preflight、result classificationをTauri / React frontendへ再実装してはならず（MUST NOT）、shared application / host operationを使用しなければならない（MUST）。

Migration `Recovery Required`中は[GUI app shell](../gui/app-shell.md)の`GUI-SHELL-CAPABILITY-001`に従いsource rename / moveを開始してはならない（MUST NOT）。

### SOURCE-PATH-007

source file rename / move自体はdirty local bufferを移送またはpath-rebindするoperationであってはならない（MUST NOT）。callerがtarget source fileにdirty bufferを保持している場合、mutation開始前にそのbufferをSaveまたは明示Discardしてclean stateへ解決するか、operationをCancelしなければならない（MUST）。

unrelated source fileのdirty bufferをrename / moveのためにSave、Discard、またはblockしてはならない（MUST NOT）。

## 検証ルール

少なくとも次をfocused core / application / host / GUI workflow evidenceで検証する。

- same-folder renameとsame-root folder moveでsource bytesとdeclared logical identityを保持する。
- `.yaml` / `.yml` destinationを許可し、configured roots間move、non-source extension、missing parent、path traversal / source-root escape / symlink escapeをmutation前にrejectする。
- existing destinationとpreflight後のdestination raceでConflictとなり、Overwriteしない。
- sourceがpreflight stateから変更または消失した場合にConflictとなる。
- case-only renameがcase-sensitive / case-insensitive filesystem差によらずrequested destination spellingへ収束する。
- known Failureはcomplete old source + unchanged distinct destinationへ収束し、それを確認できないmutation-after-failureはOutcome Unknownとなる。
- Outcome Unknown後にblind retryせず、old/new pathをrecheckする。
- target dirty bufferをSave / Don't Save / Cancelで解決し、unrelated dirty bufferを保持する。
- Recovery Required中はrename / moveを開始しない。
- rename / moveだけでBuild / Publish / Migration / Git operationを開始しない。

## 互換性

source file pathだけを変更し、YAML content、declared Table / Type identity、MessagePack field key、generated API / binary formatを直接変更しない。source-relative provenance、Explorer selection、open editor path、diagnostic/source navigation等のpath-bearing runtime stateはSuccess後にnew pathへ追従する。

configured roots間move、destination overwrite、source delete / duplicate、folder rename / moveはこのinitial contractへ含めない。Git rename detection、stage / commit behaviorはGit側のpresentationであり本仕様のownerではない。

## 未解決事項

None identified. exact native rename primitive、case-only renameのintermediate path、temporary filename、platform-specific filesystem callはimplementation detailである。

## 非目標

- configured source roots間move。
- destination overwrite。
- source file delete / duplicate。
- folder rename / move。
- source content / declared identity mutation。
- Build / Publish / Migration / Git operation。
