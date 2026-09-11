# GUI仕様: Source Creation

Status: Approved

## 目的

Workspace Explorerから新しいfolder、Table schema、Data document、Value Object、Enum / Flags Enum、Custom Typeをguided formで作成し、利用者がYAML textを手書きせずにProject authoringを開始できるようにする。

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

file-based artifactのcreation UIは、少なくともdestinationとartifact-specific declaration inputを同じguided flow内で確認できなければならない（MUST）。利用者へraw YAML textの入力を初期creationのprimary workflowとして要求してはならない（MUST NOT）。

exact componentはmodal、drawer、dedicated editor等から選択してよい（MAY）が、Create実行前にdestination source root / folder / filenameとdomain identityを区別して確認できなければならない（MUST）。

### GUI-CREATE-LAYOUT-003

複数source rootがconfiguredされている場合、creation UIはtarget source rootを明示的に識別できなければならない（MUST）。folder/file pathからTable/type identityを表示上も暗黙推論してはならない（MUST NOT）。

filenameはdomain nameからconventional defaultを提案してよい（MAY）が、利用者が変更可能でなければならず（MUST）、filename変更がdomain identityを変更してはならない（MUST NOT）。human-facing default extensionは`.yaml`とする。

## 状態（States）

### GUI-CREATE-STATE-001

creation formへ入力中のstateは、まだworkspace source fileではない一時的なform stateとして扱わなければならない（MUST）。既存Data Editorのdirty file countへ含めてはならず（MUST NOT）、Cancelではworkspace mutationを行わずformを閉じなければならない（MUST）。

Createを実行した時点で[Source Artifact Creation](../../specs/source-creation.md)のsingle-artifact commit operationを開始する。creation formを既存fileのunsaved bufferとして偽装してはならない（MUST NOT）。

### GUI-CREATE-STATE-002

Create実行中は二重submitを防ぎ、operation進行中であることを識別できなければならない（MUST）。既存Explorer、open editor、dirty bufferを不必要に破壊またはdisableしてはならない（MUST NOT）。

### GUI-CREATE-STATE-003

workspace write capabilityがgrantedされていない場合、creation commandを実行可能として扱ってはならない（MUST NOT）。UIはCreateが利用不能であることをcapability stateとして識別可能にしなければならない（MUST）。Desktop/Webというplatform名だけでavailabilityを決めてはならない（MUST NOT）。

## 操作（Interactions）

### GUI-CREATE-INT-001

Folder creationではselected source root / folder配下のnew folder nameを入力し、[SOURCE-CREATE-016](../../specs/source-creation.md)へ従ってexactly 1つのfolderを作成しなければならない（MUST）。domain kind、Table/type identityをfolder nameへ割り当ててはならない（MUST NOT）。

### GUI-CREATE-INT-002

Table creation formは少なくとも次を編集可能にしなければならない（MUST）。

- `table`
- optional `csharpName`
- ordered field list
- 各fieldのMessagePack `key`、name、base type、Required / Nullable / Array modifier
- ordered Primary Key field list
- 0個以上のSecondary Keyと`nonUnique`

field type choiceはcurrent Projectでshared application/domain layerが認識するPrimitive / Value Object / Custom Type / Enum / Flags Enumを利用できなければならない（MUST）。Primary/Secondary Keyに利用できないshapeを、frontend独自のtype ruleをauthorityとして決定してはならない（MUST NOT）。shared diagnostics / capability metadataを利用する。

simple caseを短くするため、new Table formは`key: 0`、`name: id`、`type: int`、Requiredの1 fieldをPrimary Keyとして初期提案してよい（SHOULD）。このdefaultは利用者がCreate前に変更できなければならない（MUST）。

### GUI-CREATE-INT-003

Data creation formはcurrent Projectのexisting Tableを明示的に選択できなければならない（MUST）。destination filename/folderはTable identityから独立して編集できなければならない（MUST）。

Create成功時のinitial Data documentはempty `records`から開始し、record追加UIをcreation dialogへ暗黙に含めてはならない（MUST NOT）。

### GUI-CREATE-INT-004

Value Object creation formはtype name、underlying primitive、`fromUnderlyingImplicit`、`toUnderlyingImplicit`を指定できなければならない（MUST）。underlying候補とvalidationは[Value Objects](../../specs/type-system/value-objects.md)のshared semanticsを使用し、frontendだけに許可type一覧をhard-codeしてcanonical authorityとしてはならない（MUST NOT）。

### GUI-CREATE-INT-005

Normal Enum / Flags Enum creation formはtype name、integer underlying、ordered member name/value listを指定できなければならない（MUST）。member valueは`long` / `ulong` rangeを含めlosslessなtext representationで保持しなければならず（MUST）、JavaScript `number`へ強制変換してはならない（MUST NOT）。

Normal Enumでは少なくとも1 memberを入力できるflowを提供しなければならない（MUST）。Flags Enumではcanonical `None = 0` requirementを利用者が明確に確認できる形で初期提案してよく（SHOULD）、削除/変更を許す場合もshared validationがcanonical authorityでなければならない（MUST）。implicit numberingをUI convenienceとして導入してはならない（MUST NOT）。

### GUI-CREATE-INT-006

Custom Type creation formはtype nameと1個以上のordered field listを指定できなければならない（MUST）。各fieldではMessagePack `key`、name、base type、Required / Nullable / Array modifierを編集できなければならない（MUST）。

Table formとCustom Type formは同じcanonical field semanticsを共有し、frontendごとに別のtype/modifier validation ruleを実装してはならない（MUST NOT）。

### GUI-CREATE-INT-007

Create submitはtyped requestとしてshared application serviceへ渡さなければならず（MUST）、frontendがYAML stringを組み立ててfilesystemへ直接writeしてはならない（MUST NOT）。

obviousなrequired form valueの未入力をclient-sideで案内/disableしてよい（MAY）が、domain name grammar、type capability、key/index validity、identity collision、YAML semanticsのcanonical判定をfrontendへ移してはならない（MUST NOT）。

### GUI-CREATE-INT-008

CreateがSuccessした場合、Explorerはworkspace stateを更新し、作成itemのancestor folderを必要に応じて展開して、そのitemを選択状態にしなければならない（MUST）。

source fileの場合は既存typed-editor routingへ渡さなければならない（MUST）。現在専用editorを持たないschema/type documentはexisting unsupported/read-only stateで開いてよく（MAY）、別document kindとして誤解釈して編集可能にしてはならない（MUST NOT）。Folderの場合は作成folderをselection / focus対象にし、展開可能な状態にする。

### GUI-CREATE-INT-009

creation dialogを開く、Cancelする、または別folderへtargetを変更するだけで、既存dirty fileをSave、Save All、discardしてはならない（MUST NOT）。Create成功後に新規itemへselectionが移動しても、既存dirty bufferは通常Explorer navigationと同様に保持しなければならない（MUST）。

### GUI-CREATE-INT-010

Creation conflictでは既存destinationを上書きしてはならず（MUST NOT）、creation formの入力を保持し、少なくともdestination変更またはCancelを行えるようにしなければならない（MUST）。initial sliceではcreation conflictに対する`Overwrite` actionを提供してはならない（MUST NOT）。

### GUI-CREATE-INT-011

`Outcome Unknown`ではCreateを自動retryしてはならず（MUST NOT）、同じdestinationへの次のmutation前にworkspaceをRecheck / Reloadしてactual stateを確認できるrecovery actionを提示しなければならない（MUST）。actual destinationがcomplete sourceとして存在することを確認できた場合はSuccess相当のworkspace refreshへ収束してよい（MAY）。

## キーボード（Keyboard）

### GUI-CREATE-KEY-001

Explorerの`New` command、artifact type選択、form controls、field/member row追加・削除・reorder、Create / Cancelはkeyboardだけで操作可能でなければならない（MUST）。exact shortcutは固定しない。

Enterによるform submitを提供する場合、multiline/list editing中の意図しないCreateを発生させてはならない（MUST NOT）。DestructiveではないCancelはEscape等のplatform-standard dialog interactionから到達可能にしてよい（MAY）。

## フォーカス（Focus）

### GUI-CREATE-FOCUS-001

creation UIを開いた場合、focusはartifact typeまたは最初の必要inputへ移動しなければならない（MUST）。validation errorでは最初のmappable error controlへ移動できなければならない（MUST）。

Cancel後は可能な範囲でcreation開始元のExplorer itemへfocusを戻し、Success後は作成itemをExplorer selection / focus対象にしなければならない（MUST）。

## 検証（Validation）

### GUI-CREATE-VAL-001

creation formはshared application/domain layerから返されたstructured diagnosticを使用しなければならない（MUST）。field/member/path等へ一意に対応づけられるdiagnosticは該当controlへinline表示してよい（SHOULD）。一意にmapできないdiagnosticを失ってはならず（MUST NOT）、dialog内のsummary/error surfaceから確認できなければならない（MUST）。

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