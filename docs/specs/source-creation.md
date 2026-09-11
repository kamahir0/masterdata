# Source Artifact Creation仕様

Status: Proposed

Domain: Source Authoring

## 概要

本仕様は、GUI等のauthoring surfaceからconfigured source root内へ新しいfolderまたはMasterdata source documentを安全に作成するobservable contractを定義する。

Table / Data semanticsは[Table / Primary Key / Secondary Key](table-and-keys.md)、Value Objectは[Value Objects](type-system/value-objects.md)、Enum / Flagsは[Enum / Flags](type-system/enums.md)、Custom Typeは[Custom Types](type-system/custom-types.md)、field shapeは[Field Modifiers](type-system/field-modifiers.md)、YAML syntaxは[Masterdata YAML subset](yaml-subset.md)、source root boundaryは[Project layout](project-layout.md)が所有する。本仕様はそれらを再定義せず、creation request、destination safety、initial source construction、exclusive create、result/recovery、およびshared application/host boundaryを所有する。

[Source Record Edit](source-edit.md)は既存fileのsource-preserving update contractであり、新規file creationのauthorityではない。ただしlost-update防止、Outcome Unknown、frontendへdomain semanticsを複製しない方針はcreation operationでも同じ安全性原則として維持する。

## 用語

- **Creation target**: 1つのconfigured source rootと、そのroot内のproject-relative destination pathの組。
- **Source artifact**: Table schema、Data document、Value Object、Normal Enum、Flags Enum、Custom Typeのいずれかを表す新規YAML source file。
- **Creation request**: destinationとartifact category、およびowner specificationが必要とするinitial declaration inputを含むtyped request。
- **Creation conflict**: target pathが既に存在する、またはexclusive commit直前に別process等によって作成され、既存contentを置換せずoperationを停止した状態。
- **Outcome Unknown**: destination mutation開始後のI/O/host failureにより、新規artifactがworkspace上に存在するか安全に断定できない状態。

## 規範要件

### SOURCE-CREATE-001

source creation operationは、exactly 1つのconfigured source rootをtargetとし、そのroot内にexactly 1つのnew folderまたは1つのnew source fileを作成しなければならない（MUST）。

folder path、file path、filename、directory nameはworkspace organizationであり、Table、type、recordその他のdomain identityを決めてはならない（MUST NOT）。domain identityは各source documentのcanonical contentから決定しなければならない（MUST）。

### SOURCE-CREATE-002

creation targetは選択されたconfigured source rootからのlogical relative pathとして解決しなければならず（MUST）、absolute path、`..`等のpath traversal、symlink / junction / reparse point等を利用してselected source root外へ到達してはならない（MUST NOT）。

hostはmutation開始前にtargetのparentがselected source root内の実在directoryであることを確認しなければならない（MUST）。initial sliceのfile createはmissing parent directoryを暗黙に作成してはならず（MUST NOT）、必要なfolderは明示的なfolder creation operationで作成する。

path safetyのplatform-specific mechanismはhost implementation detailであり、本仕様はOS separator、canonicalization API、filesystem identity APIを固定しない。

### SOURCE-CREATE-003

新規source fileはProjectのsource discovery対象となる`.yaml`または`.yml` extensionを持たなければならない（MUST）。GUI等のhuman-facing surfaceは`.yaml`をdefaultとして提案すべきである（SHOULD）が、`.yml`をinvalidとして扱ってはならない（MUST NOT）。

extensionやdestination directoryからdocument kindまたはdomain identityを推測してはならない（MUST NOT）。

### SOURCE-CREATE-004

initial source creation operationは次のartifact categoryをtyped requestとして区別できなければならない（MUST）。

- Table schema document
- Data document
- Value Object
- Normal Enum
- Flags Enum
- Custom Type

1つのcreation requestから複数source fileを暗黙に作成してはならない（MUST NOT）。特にTable schema作成を理由にData documentを自動作成してはならず、type作成を理由にschema/data fileを自動作成してはならない（MUST NOT）。

### SOURCE-CREATE-005

Table schema creation requestは、[Table / Primary Key / Secondary Key](table-and-keys.md)と[Field Modifiers](type-system/field-modifiers.md)のcurrent Approved contractに従うcomplete initial declarationを表現できなければならない（MUST）。少なくとも次をtyped inputとして扱う。

- logical `table` identity
- optional `csharpName`
- 1個以上のfield declaration
- 各fieldのMessagePack `key`、name、base type、field modifier
- exactly 1つのordered Primary Key
- 0個以上のSecondary Key

resulting documentは`kind: schema`を宣言しなければならず（MUST）、field/key/index semanticsをpathやUI control orderから暗黙推論してはならない（MUST NOT）。Reference等、current creation Objectiveでcanonical ownerがApprovedでないschema featureをrequestへ先取りしてはならない（MUST NOT）。

### SOURCE-CREATE-006

Data document creation requestは既存のlogical Table identityを明示しなければならず（MUST）、current ProjectでそのTableへ対応するschema documentがexactly 1つ存在しなければならない（MUST）。

initial Data documentは`kind: data`、明示された`table`、およびempty `records` sequenceを持たなければならない（MUST）。record追加をcreation operationへ暗黙に含めてはならない（MUST NOT）。Data fileのdestination pathやfilenameからTable identityを推測してはならない（MUST NOT）。

### SOURCE-CREATE-007

Value Object creation requestは[Value Objects](type-system/value-objects.md)のcurrent Approved declaration surfaceを表現しなければならない（MUST）。少なくともtype `name`、key-compatible primitive `underlying`、およびsupported directional conversion optionをtyped inputとして扱う。

resulting documentは`kind: type`とValue Object categoryを明示しなければならず（MUST）、underlying type、name grammar、conversion defaultsをfrontend独自ruleで再定義してはならない（MUST NOT）。

### SOURCE-CREATE-008

Normal Enum / Flags Enum creation requestは[Enum / Flags](type-system/enums.md)のcurrent Approved declaration surfaceを表現しなければならない（MUST）。type `name`、explicit integer `underlying`、ordered member name/value list、およびNormal EnumとFlags Enumのcategoryをtyped inputとして扱う。

implicit numberingをcreation helperとして導入してはならず（MUST NOT）、Flags Enumの`None = 0`、atomic bit rule等をfrontendだけのauthorityとして実装してはならない（MUST NOT）。shared domain validationがcanonical owner specificationへ従わなければならない（MUST）。

### SOURCE-CREATE-009

Custom Type creation requestは[Custom Types](type-system/custom-types.md)、[Field Modifiers](type-system/field-modifiers.md)、および`SCHEMA-KEY-001`のcurrent Approved declaration surfaceを表現しなければならない（MUST）。type `name`と1個以上のordered field declarationをtyped inputとして扱い、各fieldのMessagePack `key`、name、base type、field modifierを保持しなければならない（MUST）。

zero-field Custom Typeやunsupported field shapeをfrontend側で別semanticへ補正して成功扱いしてはならない（MUST NOT）。

### SOURCE-CREATE-010

source file mutation開始前に、shared application/domain layerはcreation requestからcandidate documentを構成し、少なくとも次を確認しなければならない（MUST）。

- candidateがApproved YAML subsetとしてparse可能であること。
- requested declarationがそのartifact categoryのApproved owner specificationを満たすこと。
- Table schema / type declaration等のproject-local identityがcurrent Project内でcollisionしないこと。
- Data documentがcurrent Projectのexisting Tableへresolveすること。
- field base type等、requestが参照するcurrent Project symbolが必要に応じてresolveすること。

candidate自身が導入するsemantic errorまたはidentity collisionが存在する場合、destination mutationを開始してはならない（MUST NOT）。一方、candidateと無関係な既存Project diagnosticだけを理由にcreation requestを一律拒否してはならない（MUST NOT）。

### SOURCE-CREATE-011

same typed creation requestとsame relevant Project snapshotから生成されるinitial sourceはdeterministicでなければならない（MUST）。生成されたsourceをshared parserへ再入力した場合、requested artifact categoryとdeclarationへ一致しなければならない（MUST）。

exact indentation、blank-line layout、flow/block style等のpresentation detailはpublic compatibility contractとして固定しない。ただしfrontendがYAML textを手組みしてdomain semanticsを決定してはならず（MUST NOT）、shared core/application rendererがsource constructionを所有しなければならない（MUST）。

### SOURCE-CREATE-012

destination fileまたはfolderが既に存在する場合、creation operationは既存entryを上書き、truncate、merge、rename、deleteしてはならない（MUST NOT）。targetがpreflight後からcommit直前までに作成された場合もexclusive createとして停止し、Creation conflictとして観測可能にしなければならない（MUST）。

`force`、implicit overwrite、existing empty fileの再利用をinitial creation operationへ含めてはならない（MUST NOT）。

### SOURCE-CREATE-013

ordinary operation failureによってpartial/truncated destinationをsuccessとして公開してはならない（MUST NOT）。file creation implementationはstaging、temporary file、exclusive create、same-directory replace/link等、hostが提供するmechanismを組み合わせ、Success時にcomplete requested sourceだけがnew destinationとして観測されるようにしなければならない（MUST）。

operationがFailureと確定した場合、best effortではなくcontractとしてexisting workspace entryを変更してはならず（MUST NOT）、new destinationへ不完全contentを残してはならない（MUST NOT）。process crash、OS crash、power lossを含むglobal filesystem transaction atomicityは保証しない。

### SOURCE-CREATE-014

creation resultは少なくとも`Success`、`Conflict`、`Failure`、`Outcome Unknown`を観測上区別できなければならない（MUST）。exact API enum名は固定しない。

- `Success`: requested new folderまたはcomplete new source fileがdestinationに存在する。
- `Conflict`: destinationが既に存在する等、mutationを安全に開始/完了できずexisting entryを保持した。
- `Failure`: operationが成功しなかったことを確定でき、destinationが作成されていないことをcallerが扱える。
- `Outcome Unknown`: mutation開始後のfailure等によりdestinationの存在/complete contentを安全に断定できない。

`Conflict`、`Failure`、`Outcome Unknown`を`Success`として報告してはならない（MUST NOT）。

### SOURCE-CREATE-015

`Outcome Unknown`では同じrequestを自動retryしてはならず（MUST NOT）、次のmutation前にworkspaceを再取得してdestinationのactual stateを確認しなければならない（MUST）。`Conflict`でも既存destinationを自動削除/上書きしてretryしてはならない（MUST NOT）。

Success後はworkspace snapshotを再取得または等価に更新し、新規entryをExplorer等のcallerへ返せる状態にしなければならない（MUST）。

### SOURCE-CREATE-016

folder creationはsource root内のworkspace organizationだけを変更しなければならず（MUST）、domain document、Table、type、record、Build artifactを作成してはならない（MUST NOT）。initial sliceではrequested folderのparentが存在しなければならず（MUST）、recursive missing-parent creationを暗黙に行ってはならない（MUST NOT）。

existing folderをSuccessとして黙って再利用してはならず（MUST NOT）、Creation conflictとして扱わなければならない（MUST）。

### SOURCE-CREATE-017

source artifact / folder creationはBuild、Publish、Git stage / commit / push、Schema Migration、generated C#更新、binary更新を暗黙に開始してはならない（MUST NOT）。作成後のvalidation表示は別operation/resultとして扱い、Success自体をProject全体がvalidであることの保証として扱ってはならない（MUST NOT）。

### SOURCE-CREATE-018

request validation、domain declaration construction、initial YAML rendering等のshared semanticsをTauri frontend、Browser Host、Native Host adapterごとに再実装してはならない（MUST NOT）。pureなcandidate construction / validationはshared core/application boundaryで再利用可能でなければならない（MUST）。

filesystem destination resolution、path safety、exclusive create、permission、Outcome Unknownの判定等のhost I/Oは[Runtime hosts](runtime-hosts.md)のcapability / host boundaryに従わなければならない（MUST）。

## 検証ルール

少なくとも次をfocused unit / integration / GUI workflow evidenceで検証する。

- configured source root外へのabsolute/path traversal/symlink escapeがmutation前にrejectされる。
- `.yaml`と`.yml`はsource fileとして作成可能で、別extensionはinitial source creationでrejectされる。
- Table requestからowner specificationを満たすschema documentが生成され、pathからTable identityを推測しない。
- Data requestはexisting Tableを明示してempty `records` documentを生成し、unknown Tableではwriteしない。
- Value Object、Normal Enum、Flags Enum、Custom Typeのvalid requestが各owner specificationへparse/resolveする。
- duplicate Table schema/type identity等、candidateが導入するcollisionではwriteしない。
- unrelatedな既存Project diagnosticだけではvalid creation requestを拒否しない。
- target file/folderが既に存在する場合、byte/content/metadataを変更しない。
- preflight後にtargetが出現するraceでもexisting destinationをoverwriteしない。
- write failureではpartial destinationをFailure/Successとして残さず、Outcome Unknownではautomatic retryしない。
- creationはBuild / Publish / Git operationを開始しない。

## 互換性

本仕様は新しいauthoring operationを追加するものであり、既存YAML document format、Table/type semantics、Build artifact、CLI grammarを変更しない。既存source fileはcreation operationを実行しない限り変更されない。

## 初期sliceの非目標

- rename / delete / move / duplicate。
- record追加・削除。
- schema/type declarationのpost-create edit operation。
- 1操作で複数source fileを作成するtransaction。
- Draft owner specificationのfeatureをcreation requestへ先取りすること。
- external template/import、Git operation。

## 未解決事項（Open Questions）

None identified for the initial Source Artifact Creation slice.
