# GUI仕様: Source Creation

Status: Approved

## 目的

Workspace Explorerから新しいfolder、Table schema、Data document、Value Object、Enum / Flags Enum、Custom Typeを、日常操作ではmodal formを経由せず直接作成できるようにする。primary flowはartifact kindを選択し、Explorer上のprovisional itemへfilenameをinline入力し、Enterでcommit、Escapeでcancelするdirect manipulationとする。

artifact固有の詳細設定は作成後に通常のtyped editor上で編集する。作成時に利用者へbackend/domainの完全なdeclaration inputを要求しない。shared application layerはartifact kindとdestinationからcanonicalにvalidなstarter sourceを構成し、[Source Artifact Creation](../../specs/source-creation.md)のpath safety、validation、exclusive createを利用する。frontendがYAML text、domain default validation、filesystem writeを独自実装してはならない。

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

file-based artifactのprimary creation flowは、artifact kind選択後にtarget Explorer folder内へprovisional itemを表示し、その行でfilenameをinline編集できなければならない（MUST）。provisional itemはまだworkspace sourceではなく、dirty file countへ含めてはならない（MUST NOT）。

Enterまたは同等のexplicit commitでsingle-artifact creationを開始し、Escapeまたは同等のcancelでworkspace mutationなしにprovisional itemを破棄しなければならない（MUST）。Table / Value Object / Enum / Flags / Custom Type等の日常的な作成に、domain declaration一式を入力するmodal / drawer / wizardを必須にしてはならない（MUST NOT）。

Dataのtarget Tableなど、valid starterを作るためにfilename以外のdomain selectionが不可避なartifactは、kind selectionのsub-menuまたはcompact transient selectorで不足情報を選択させてよい（MAY）。その場合もraw YAMLや完全declaration formをprimary creation pathにしてはならない（MUST NOT）。

### GUI-CREATE-LAYOUT-003

複数source rootがconfiguredされている場合、creation targetはExplorer上のselected root / folder contextによって明示的に識別できなければならない（MUST）。target contextが曖昧な状態で別rootへ暗黙作成してはならない（MUST NOT）。

initial starter creationに限り、shared application layerはfilename stemからTable/type identityの初期値をdeterministically提案してよい（MAY）。これはcreation-time defaultであり、physical pathとdomain identityを同一conceptにしてはならない（MUST NOT）。作成後のfile rename / moveがdomain identityを暗黙変更してはならず、domain identityはtyped editorから独立して変更可能でなければならない（MUST）。

human-facing default extensionは`.yaml`とし、利用者が拡張子を省略した場合にGUI/application layerが補完してよい（MAY）。

## 状態（States）

### GUI-CREATE-STATE-001

provisional creation itemとそのinline filenameは、一時的なGUI stateとして扱わなければならない（MUST）。既存Data Editorのdirty file countへ含めてはならず（MUST NOT）、Escape / Cancelではworkspace mutationを行ってはならない（MUST NOT）。

Enter等でcommitした時点で[Source Artifact Creation](../../specs/source-creation.md)のsingle-artifact operationを開始する。commit開始前のprovisional itemを既存fileのunsaved bufferとして扱ってはならない（MUST NOT）。

### GUI-CREATE-STATE-002

Create実行中は二重submitを防ぎ、operation進行中であることを識別できなければならない（MUST）。既存Explorer、open editor、dirty bufferを不必要に破壊またはdisableしてはならない（MUST NOT）。

### GUI-CREATE-STATE-003

workspaceが開かれていない、または[GUI app shell](../app-shell.md)のRecovery Required gate等でsource mutationが停止中の場合、creation commandを実行可能として扱ってはならない（MUST NOT）。

## 操作（Interactions）

### GUI-CREATE-INT-001

Folder creationではselected source root / folder配下のnew folder nameを入力し、[SOURCE-CREATE-016](../../specs/source-creation.md)へ従ってexactly 1つのfolderを作成しなければならない（MUST）。domain kind、Table/type identityをfolder nameへ割り当ててはならない（MUST NOT）。

### GUI-CREATE-INT-002

Table creationは、利用者がartifact kindとしてTableを選びfilenameをcommitした時点で、shared application layerが直ちに編集可能なvalid starter Tableを構成しなければならない（MUST）。starterは少なくとも通常のTable Editorからfield / key / record authoringを開始できる状態でなければならない（MUST）。

simple starterのdefaultとして`id: int` Required fieldをPrimary Keyにしたschemaを使用してよい（SHOULD）。MessagePack key、initializer、その他canonical declaration値の構成とvalidationはshared application/domain layerが所有し、frontendへdomain ruleとして複製してはならない（MUST NOT）。

`csharpName`、Secondary Key、Record storage、追加field等をCreate前に入力することをprimary flowの必須条件にしてはならない（MUST NOT）。これらは作成成功後のTable Editor / Advanced settingsから編集する。

### GUI-CREATE-INT-003

Data creationはcurrent Projectのexisting Tableをshared application authorityから選択し、そのTableに対するempty `records` documentを作成しなければならない（MUST）。target TableがExplorer contextから一意に決まらない場合は、kind selectionに続くcompact selectorで明示選択させる（MUST）。

destination filename/folderはTable identityから独立していなければならない（MUST）。Create成功時のinitial Data documentはempty `records`から開始し、作成後は通常のData Editorでrecordを追加する。

### GUI-CREATE-INT-004

Value Object creationはfilename commitからshared application layerがcanonicalにvalidなstarter Type documentを構成し、成功後にType Editorへ開かなければならない（MUST）。type name / underlying / conversionのcanonical validityはshared semanticsへ委譲し、frontendが独自validatorやYAML renderingを持ってはならない（MUST NOT）。

underlyingやconversionの変更をCreate前modalの必須入力にしてはならず（MUST NOT）、作成後のType Editorで編集できなければならない（MUST）。

### GUI-CREATE-INT-005

Normal Enum / Flags Enum creationはfilename commitからshared application layerがcanonicalにvalidなstarter Type documentを構成し、成功後にType Editorへ開かなければならない（MUST）。Flagsの`None = 0`等のcanonical requirementはshared semanticsが所有する。

underlyingやmember listをCreate前modalの必須入力にしてはならない（MUST NOT）。member valueは作成後のeditorでも`long` / `ulong` rangeを含めlosslessなtext representationとして扱い、JavaScript `number`へ強制変換してはならない（MUST NOT）。

### GUI-CREATE-INT-006

Custom Type creationはfilename commitからshared application layerがcanonicalにvalidなstarter Type documentを構成し、成功後にType Editorへ開かなければならない（MUST）。starter fieldの具体的なname/typeはshared applicationのdeterministic starter policyに従う。

ordered field list、MessagePack key、type、modifierをCreate前modalの必須入力にしてはならない（MUST NOT）。作成後のType Editorがcanonical field semanticsをshared application/domain layer経由で編集する。

### GUI-CREATE-INT-007

Create submitはtyped requestとしてshared application serviceへ渡さなければならず（MUST）、frontendがYAML stringを組み立ててfilesystemへ直接writeしてはならない（MUST NOT）。

obviousなrequired form valueの未入力をclient-sideで案内/disableしてよい（MAY）が、domain name grammar、type capability、key/index validity、identity collision、YAML semanticsのcanonical判定をfrontendへ移してはならない（MUST NOT）。

### GUI-CREATE-INT-008

CreateがSuccessした場合、Explorerはworkspace stateを更新し、作成itemのancestor folderを必要に応じて展開して、そのitemを選択状態にしなければならない（MUST）。

source fileの場合は既存typed-editor routingへ渡さなければならない（MUST）。現在専用editorを持たないschema/type documentはexisting unsupported/read-only stateで開いてよく（MAY）、別document kindとして誤解釈して編集可能にしてはならない（MUST NOT）。Folderの場合は作成folderをselection / focus対象にし、展開可能な状態にする。

### GUI-CREATE-INT-009

creation dialogを開く、Cancelする、または別folderへtargetを変更するだけで、既存dirty fileをSave、Save All、discardしてはならない（MUST NOT）。Create成功後に新規itemへselectionが移動しても、既存dirty bufferは通常Explorer navigationと同様に保持しなければならない（MUST）。

### GUI-CREATE-INT-010

Creation conflictでは既存destinationを上書きしてはならず（MUST NOT）、creation input stateを保持し、少なくともdestination変更またはCancelを行えるようにしなければならない（MUST）。initial sliceではcreation conflictに対する`Overwrite` actionを提供してはならない（MUST NOT）。

### GUI-CREATE-INT-011

`Outcome Unknown`ではCreateを自動retryしてはならず（MUST NOT）、同じdestinationへの次のmutation前にworkspaceをRecheck / Reloadしてactual stateを確認できるrecovery actionを提示しなければならない（MUST）。actual destinationがcomplete sourceとして存在することを確認できた場合はSuccess相当のworkspace refreshへ収束してよい（MAY）。

## キーボード（Keyboard）

### GUI-CREATE-KEY-001

Explorerの`New` command、artifact type選択、provisional filename editing、commit / cancelはkeyboardだけで操作可能でなければならない（MUST）。inline filename中のEnterはcreation commit、Escapeはcreation cancelとして動作しなければならない（MUST）。IME composition中のEnterをcommitとして誤解釈してはならない（MUST NOT）。

追加のdomain selectorが必要なartifactでもkeyboard-only操作を失ってはならない（MUST NOT）。

## フォーカス（Focus）

### GUI-CREATE-FOCUS-001

artifact kind選択後は作成先Explorer行のinline filename inputへfocusを移さなければならない（MUST）。Escape / failure後は可能な範囲でcreation開始元またはprovisional itemへfocusを戻し、Success後は作成itemをExplorer selection / focus対象にしてtyped editorへ移動できる状態にしなければならない（MUST）。

structured diagnosticがfilenameまたは追加domain selectorへ一意に対応づけられる場合は、そのinputへfocus / inline errorを提供してよい（SHOULD）。

## 検証（Validation）

### GUI-CREATE-VAL-001

creation input surfaceはshared application/domain layerから返されたstructured diagnosticを使用しなければならない（MUST）。field/member/path等へ一意に対応づけられるdiagnosticは該当controlへinline表示してよい（SHOULD）。一意にmapできないdiagnosticを失ってはならず（MUST NOT）、creation contextのsummary/error surfaceから確認できなければならない（MUST）。

frontend独自validatorをcanonical authorityとして使用してはならない（MUST NOT）。

### GUI-CREATE-VAL-002

Create前のpreview validationを実装する場合、latest form revisionに対応する結果だけを表示しなければならない（MUST）。stale async resultが新しいform inputを上書き、clear、valid扱いしてはならない（MUST NOT）。exact debounce / request cancellation mechanismは実装調整値とする。

Create operation自体は、shared creation preflightがinvalidと判定したrequestでdestination writeを開始してはならない（MUST NOT）。

## エラー（Errors）

### GUI-CREATE-ERR-001

path safety、permission、destination conflict、semantic validation、I/O failure、Outcome Unknownは互いに識別可能なstructured stateとして表示しなければならない（MUST）。error modalを閉じることでform inputを失わせてはならない（MUST NOT）。

### GUI-CREATE-ERR-002

Failureではexisting source / dirty editor stateを保持し、form inputを修正してretryできなければならない（MUST）。Outcome Unknownではblind retryを提供せず、[GUI-CREATE-INT-011](#gui-create-int-011)のrecheck lifecycleを使用しなければならない（MUST）。

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