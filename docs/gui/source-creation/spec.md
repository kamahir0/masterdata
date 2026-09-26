# GUI仕様: Source Creation

Status: Approved

## 目的

Workspace Explorerから新しいfolder、Table schema、Data document、Value Object、Enum / Flags Enum、Custom Typeを短いdefault creation flowで作成し、利用者がYAML textを手書きせずにProject authoringを開始できるようにする。詳細な初期宣言が必要な場合はAdvanced creationを提供する。

このsurfaceは[Source Artifact Creation](../../specs/source-creation.md)のshared operationを利用する。Table / Type semantics、YAML rendering、path safety、exclusive createをfrontend独自に実装しない。

## レイアウト（Layout）

### GUI-CREATE-LAYOUT-001

Workspace Explorerはsource rootまたはその配下folderを対象に、新規itemを作成する`New` command surfaceを提供しなければならない（MUST）。少なくとも次を選択できなければならない（MUST）。

- Folder
- Table
- Data
- Value Object
- Enum
- Flags Enum
- Custom Type

exact icon、menu placement、toolbar/context menu併用は固定しない。fileを選択中の場合は、そのfileのparent folderをdefault destination contextとしてよい（MAY）。

### GUI-CREATE-LAYOUT-002

file-based artifactの通常creation UIはExplorer内に未commitの仮nodeを表示し、filenameをinline編集できなければならない（MUST）。Enterで作成を開始し、Escapeでmutationなしに中止できなければならない（MUST）。kind、destination source root / folder、filename、作成されるdomain identityの候補を作成前に識別できなければならない（MUST）。raw YAML text入力をprimary workflowにしてはならない（MUST NOT）。

defaultで表現できない初期declarationはAdvanced creationから指定できなければならない（MUST）。exact componentは固定しない。

### GUI-CREATE-LAYOUT-003

複数source rootがconfiguredされている場合、target rootを明示的に識別できなければならない（MUST）。filenameは変更できなければならず（MUST）、仮nodeのfilename変更中だけApplicationが提案するdomain identity候補を更新してよい（MAY）。候補identityは作成されるcanonical sourceに明示され、作成後のfilename変更 / moveはdomain identityを変更してはならない（MUST NOT）。human-facing default extensionは`.yaml`とする。

## 状態（States）

### GUI-CREATE-STATE-001

仮nodeまたはAdvanced入力中のstateは、まだworkspace source fileではない一時stateとして扱わなければならない（MUST）。既存Data Editorのdirty file countへ含めてはならず（MUST NOT）、Escape / Cancelではworkspace mutationを行わずstateを閉じなければならない（MUST）。Enter / Createで[Source Artifact Creation](../../specs/source-creation.md)のsingle-artifact commit operationを開始する。仮nodeを既存fileのunsaved bufferとして偽装してはならない（MUST NOT）。

### GUI-CREATE-STATE-002

Create実行中は二重submitを防ぎ、operation進行中であることを識別できなければならない（MUST）。既存Explorer、open editor、dirty bufferを不必要に破壊またはdisableしてはならない（MUST NOT）。

### GUI-CREATE-STATE-003

workspaceが開かれていない、または[GUI app shell](../app-shell.md)のRecovery Required gate等でsource mutationが停止中の場合、creation commandを実行可能として扱ってはならない（MUST NOT）。

## 操作（Interactions）

### GUI-CREATE-INT-001

Folder creationではselected source root / folder配下のnew folder nameを入力し、[SOURCE-CREATE-016](../../specs/source-creation.md)へ従ってexactly 1つのfolderを作成しなければならない（MUST）。domain kind、Table/type identityをfolder nameへ割り当ててはならない（MUST NOT）。

### GUI-CREATE-INT-002

Tableの通常作成はshared Applicationが現行Table semanticsで有効な最小宣言を作り、Required `id: int`、MessagePack `key: 0`、Primary Key `id`を初期提案すべきである（SHOULD）。empty inline `records: []`を通常の初期提案とし、schema-only fileを作る選択も提供しなければならない（MUST）。

Advanced creationではlogical `table`、optional `csharpName`、ordered field、MessagePack key / type / modifier、Primary / Secondary KeyをCreate前に編集できなければならない（MUST）。field type choiceとvalidationはshared Application / Domainをauthorityとする。

### GUI-CREATE-INT-003

Data作成では既存のlogical Table identityを明示しなければならない（MUST）。現在のTable文脈から一意に決まる場合は提案してよい（MAY）が、一意でなければ作成前にTable選択を求める（MUST）。filename / folderはTable identityから独立して編集できなければならない（MUST）。成功時はempty `records`で開始し、record追加をcreationへ暗黙に含めてはならない（MUST NOT）。

### GUI-CREATE-INT-004

Value Objectの通常作成はshared Applicationが有効な最小宣言を構成しなければならない（MUST）。underlying primitiveと`fromUnderlyingImplicit` / `toUnderlyingImplicit`を初期作成時に指定するAdvanced導線を提供しなければならない（MUST）。候補とvalidationはshared semanticsをauthorityとし、frontend独自に許可typeを決めてはならない（MUST NOT）。

### GUI-CREATE-INT-005

Normal Enum / Flags Enumの通常作成はshared Applicationが有効な最小memberを含む宣言を構成しなければならない（MUST）。underlyingとordered member name / explicit numeric valueを初期作成時に指定するAdvanced導線を提供しなければならない（MUST）。member valueは`long` / `ulong`を含めlossless textで保持し、JavaScript `number`へ強制変換してはならない（MUST NOT）。Flagsの`None = 0`とatomic bitはshared semanticsをauthorityとする。

### GUI-CREATE-INT-006

Custom Typeの通常作成はshared Applicationが1個以上のfieldを含む有効な最小宣言を構成しなければならない（MUST）。type name、ordered field、MessagePack key / type / modifierを初期作成時に指定するAdvanced導線を提供しなければならない（MUST）。frontendがfield semanticsを再実装してはならない（MUST NOT）。

### GUI-CREATE-INT-007

Create submitはtyped requestとしてshared application serviceへ渡さなければならず（MUST）、frontendがYAML stringを組み立ててfilesystemへ直接writeしてはならない（MUST NOT）。

obviousなrequired form valueの未入力をclient-sideで案内/disableしてよい（MAY）が、domain name grammar、type capability、key/index validity、identity collision、YAML semanticsのcanonical判定をfrontendへ移してはならない（MUST NOT）。

### GUI-CREATE-INT-008

CreateがSuccessした場合、Explorerはworkspace stateを更新し、作成itemのancestor folderを必要に応じて展開して、そのitemを選択状態にしなければならない（MUST）。

source fileの場合は既存typed-editor routingへ渡さなければならない（MUST）。現在専用editorを持たないschema/type documentはexisting unsupported/read-only stateで開いてよく（MAY）、別document kindとして誤解釈して編集可能にしてはならない（MUST NOT）。Folderの場合は作成folderをselection / focus対象にし、展開可能な状態にする。

### GUI-CREATE-INT-009

creation surfaceを開く、Cancelする、または別folderへtargetを変更するだけで、既存dirty fileをSave、Save All、discardしてはならない（MUST NOT）。Create成功後に新規itemへselectionが移動しても、既存dirty bufferは通常Explorer navigationと同様に保持しなければならない（MUST）。

### GUI-CREATE-INT-010

Creation conflictでは既存destinationを上書きしてはならず（MUST NOT）、creation入力を保持し、少なくともdestination変更またはCancelを行えるようにしなければならない（MUST）。initial sliceではcreation conflictに対する`Overwrite` actionを提供してはならない（MUST NOT）。

### GUI-CREATE-INT-011

`Outcome Unknown`ではCreateを自動retryしてはならず（MUST NOT）、同じdestinationへの次のmutation前にworkspaceをRecheck / Reloadしてactual stateを確認できるrecovery actionを提示しなければならない（MUST）。actual destinationがcomplete sourceとして存在することを確認できた場合はSuccess相当のworkspace refreshへ収束してよい（MAY）。

## キーボード（Keyboard）

### GUI-CREATE-KEY-001

Explorerの`New`、kind選択、仮nodeのfilename入力、Enterによる作成、Escapeによる中止はkeyboardだけで操作可能でなければならない（MUST）。Advanced creationのform controls、field/member row追加・削除・reorder、Create / Cancelもkeyboardだけで操作可能でなければならない（MUST）。multiline/list editing中のEnterが意図しないCreateを起こしてはならない（MUST NOT）。

## フォーカス（Focus）

### GUI-CREATE-FOCUS-001

creation UIを開いた場合、focusは仮nodeのfilenameまたは最初の必要inputへ移動しなければならない（MUST）。validation errorでは最初のmappable error controlへ移動できなければならない（MUST）。

Cancel後は可能な範囲でcreation開始元のExplorer itemへfocusを戻し、Success後は作成itemをExplorer selection / focus対象にしなければならない（MUST）。

## 検証（Validation）

### GUI-CREATE-VAL-001

creation surfaceはshared application/domain layerから返されたstructured diagnosticを使用しなければならない（MUST）。field/member/path等へ一意に対応づけられるdiagnosticは該当controlへinline表示してよい（SHOULD）。一意にmapできないdiagnosticを失ってはならず（MUST NOT）、仮nodeまたはAdvanced surfaceのsummary/errorから確認できなければならない（MUST）。

frontend独自validatorをcanonical authorityとして使用してはならない（MUST NOT）。

### GUI-CREATE-VAL-002

Create前のpreview validationを実装する場合、latest form revisionに対応する結果だけを表示しなければならない（MUST）。stale async resultが新しいform inputを上書き、clear、valid扱いしてはならない（MUST NOT）。exact debounce / request cancellation mechanismは実装調整値とする。

Create operation自体は、shared creation preflightがinvalidと判定したrequestでdestination writeを開始してはならない（MUST NOT）。

## エラー（Errors）

### GUI-CREATE-ERR-001

path safety、permission、destination conflict、semantic validation、I/O failure、Outcome Unknownは互いに識別可能なstructured stateとして表示しなければならない（MUST）。error surfaceを閉じることでcreation入力を失わせてはならない（MUST NOT）。

### GUI-CREATE-ERR-002

Failureではexisting source / dirty editor stateを保持し、creation入力を修正してretryできなければならない（MUST）。Outcome Unknownではblind retryを提供せず、[GUI-CREATE-INT-011](#gui-create-int-011)のrecheck lifecycleを使用しなければならない（MUST）。

## アクセシビリティ（Accessibility）

### GUI-CREATE-A11Y-001

artifact type、destination、field/member list、Primary/Secondary Key selection、form error、Create progress/resultをassistive technologyから識別可能にしなければならない（MUST）。required/read-only/invalid stateを色だけで表現してはならない（MUST NOT）。

reorderable listをdrag操作だけに依存させてはならず（MUST NOT）、keyboardまたは明示buttonで同等のorder変更を可能にしなければならない（MUST）。

## 参照artifact（Reference Artifacts）

None.

## 初期sliceの非目標

- rename / delete / move / duplicate。
- Data record追加・削除。
- 作成後のschema/type専用editor。
- Table + Data等のmulti-artifact creation transaction。
- raw YAML template editor、external template/import。
- Draft domain featureの先取り。
- Git operation。

## 未解決事項（Open Questions）

None identified for the initial Source Creation surface.
