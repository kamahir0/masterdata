# GUI仕様: Type Editor

Status: Approved

## 目的

既存Value Object / Enum / Flags Enum / Custom Type documentをWorkspace Explorerから選択し、[Type Migration v1](../../specs/type-migration.md)のsemantic operationをdeterministic Planとsource Diffを確認しながら安全に実行できるschema-aware Type Editorを提供する。

Type category、member/field、data representation、generated C# semanticsは[Type System](../../specs/type-system/README.md) family、dependency resolution、source-preserving rewrite、stale-plan preflight、destructive authorization、multi-file commit / rollbackはType Migration v1が所有する。本仕様はType Editorのlayout、state、interaction、dirty-buffer composition、error/recovery、adapter boundaryを所有する。frontendはYAMLを独自parse/rewriteしない。

## レイアウト（Layout）

### GUI-TYPE-LAYOUT-001

Workspace Explorerでshared source semantics上の`kind: type` documentを選択した場合、main areaはType Editorを表示しなければならない（MUST）。physical pathやfolder名だけでtype categoryまたはlogical identityを決定してはならない（MUST NOT）。

Type Editorは少なくともlogical type name、source provenance、resolved categoryを確認できなければならない（MUST）。表示値はshared application/domain snapshotから取得し、frontendがYAMLを独自parseしてdomain meaningを再構成してはならない（MUST NOT）。

### GUI-TYPE-LAYOUT-002

Value Objectではunderlying primitiveと`fromUnderlyingImplicit` / `toUnderlyingImplicit`、Normal Enum / Flags Enumではunderlyingとmember declaration order・name・numeric value、Custom Typeではfield declaration order・MessagePack key・name・base type・modifierを表示しなければならない（MUST）。

v1非対象のdeclaration name、underlying、member numeric value、field type/modifier/key/reorder等をdirect-edit可能なcontrolとして表示してはならない（MUST NOT）。read-onlyとして表示することはできる（MAY）。

### GUI-TYPE-LAYOUT-003

categoryごとの初期mutation actionは次に限定しなければならない（MUST）。

- Value Object: conversion setting変更。
- Normal Enum / Flags Enum: member Add / Rename / Drop。
- Custom Type: field Add / Rename / Drop。

Type category conversion、type rename、underlying変更、member numeric value変更、Custom field type/modifier/key変更またはreorderを同じType Editor v1のApply actionへ含めてはならない（MUST NOT）。

## 状態（States）

### GUI-TYPE-STATE-001

Type Editorは少なくともtype loading、ready、operation input editing、Plan loading、Plan ready、Apply in progress、stale plan / re-plan required、commit failure with rollback / no mutation、Recovery Requiredを観測上区別できなければならない（MUST）。以前選択していたtype declarationまたはold Planを、新selectionのcurrent editable / applicable stateとして表示してはならない（MUST NOT）。

### GUI-TYPE-STATE-002

Type Migration operation inputやPlanをData Editorのsource dirty bufferと同一conceptとして扱ってはならない（MUST NOT）。Type Editorの`Apply`だけがType Migration source mutationを開始し、Cmd+S / Ctrl+SをMigration Applyのshortcutへ暗黙に割り当ててはならない（MUST NOT）。Plan / Apply failureだけを理由にoperation inputを不必要に失ってはならない（MUST NOT）。

### GUI-TYPE-STATE-003

Type Migration Planがaffected sourceとして示すfileにData Editorのdirty bufferが存在する場合、Type EditorはApplyを開始してはならない（MUST NOT）。該当dirty fileを利用者が識別できなければならない（MUST）。

Planに含まれないunrelated dirty bufferだけを理由にPlanまたはApplyを全面禁止してはならず（MUST NOT）、Type Migration操作によってそのbufferを保存、破棄、reloadしてはならない（MUST NOT）。

### GUI-TYPE-STATE-004

Type Migration成功後、affected sourceのcurrent workspace authorityを再取得し、Type Editorをcurrent declarationへ更新しなければならない（MUST）。affected fileについて既存clean editor snapshotをstale contentのままeditableとして残してはならない（MUST NOT）。unrelated dirty bufferは保持しなければならない（MUST）。

## 操作（Interactions）

### GUI-TYPE-INT-001

Value Objectのconversion editingは`SetValueObjectConversions` semantic commandを構成するguided inputを提供し、`fromUnderlyingImplicit`と`toUnderlyingImplicit`を独立して設定できなければならない（MUST）。underlyingまたはtype nameを同じoperationで変更してはならない（MUST NOT）。

### GUI-TYPE-INT-002

Normal Enum / Flags Enumは`AddEnumMember`、`RenameEnumMember`、`DropEnumMember`のguided inputを提供しなければならない（MUST）。Addはexplicit nameとnumeric valueを入力できなければならず（MUST）、frontendがimplicit numberingを生成してsemantic defaultとして扱ってはならない（MUST NOT）。Renameはcurrent member nameをselectorとしてshared Type Migration boundaryへ渡し、numeric valueを編集してはならない（MUST NOT）。

Dropはdestructive actionとして明示しなければならない（MUST）。Flagsの`None = 0`はRename / Drop actionの対象として提供してはならない（MUST NOT）。

### GUI-TYPE-INT-003

Custom Typeは`AddCustomField`、`RenameCustomField`、`DropCustomField`のguided inputを提供しなければならない（MUST）。AddはMessagePack key、field name、base type、modifier、およびType Migration semantics上必要なexplicit initializerを入力できなければならない（MUST）。initializerのvalidityをfrontend独自Type Systemで確定してはならない（MUST NOT）。

Renameはcurrent field nameをselectorとしてshared Type Migration boundaryへ渡し、MessagePack key / base type / modifierを同時変更してはならない（MUST NOT）。Dropはdestructive actionとして明示しなければならない（MUST）。

### GUI-TYPE-INT-004

source mutation前に必ずcurrent semantic commandに対応するType Migration Planを取得しなければならない（MUST）。Plan surfaceは少なくともoperation / target、destructive state、affected source files、affected value occurrence count、migration validation / diagnosticsを表示しなければならない（MUST）。Plan作成・表示はsourceを変更してはならない（MUST NOT）。

### GUI-TYPE-INT-005

Planからaffected fileごとのbefore / after Diffへ移動できなければならない（MUST）。Diffはcurrent Planのbase sourceと同一Planのtransformed candidateを比較し、別のworkspace readやgenerated artifactを代用してはならない（MUST NOT）。Diff表示から戻るときにoperation input、Plan identity、target member/fieldを不必要に失ってはならない（MUST NOT）。

### GUI-TYPE-INT-006

Applyはcurrent UIが示しているsemantic commandとbase snapshotに対応するcurrent Planだけを対象にしなければならない（MUST）。Plan作成後にoperation inputを変更した場合、old PlanをcurrentとしてApplyしてはならない（MUST NOT）。

commit preflightがproject config、source membership、resolutionへ使用したsource bytesのstale stateを検出した場合はsource mutationを開始せず、stale planとして表示し、re-plan actionを提供しなければならない（MUST）。silent retry / silent overwriteを行ってはならない（MUST NOT）。

### GUI-TYPE-INT-007

`DropEnumMember`および`DropCustomField`はApply前にtargetとdestructive natureを確認する明示的confirmationを要求しなければならない（MUST）。GUI confirmationだけをbackend authorizationの代替にしてはならず（MUST NOT）、shared application boundaryへmachine-actionable destructive execution authorizationを渡さなければならない（MUST）。

backendがexisting occurrence等のType Migration precondition failureを返した場合、GUIはreplacementやdata conversionを推測して再試行してはならない（MUST NOT）。

### GUI-TYPE-INT-008

Type Migration成功をBuild / Publish / Git / generated artifact更新と同一操作へ結合してはならない（MUST NOT）。成功後にBuild等を別actionとして実行できてよい（MAY）。

### GUI-TYPE-INT-009

Type Editorからshared application boundaryへ渡すsemantic valueは、Approved Type System / YAML subsetが許すexact valueをlosslessに保持しなければならない（MUST）。特にEnum / Flagsの`long` / `ulong`を含むmember numeric value、Custom Type field initializer内の64-bit integerその他のscalar valueを、JavaScriptのsafe-integer rangeへ丸めたり、floating-point `number`へlossy coercionしてはならない（MUST NOT）。

このrequirementはspecific wire encodingを固定しない。string、tagged scalar、BigInt-safe adapter等の具体的transport representationは、round-tripでexact semantic valueを保持し、shared Rust semanticsが最終validationを行う限りimplementation detailとしてよい（MAY）。

## キーボード（Keyboard）

### GUI-TYPE-KEY-001

Type Editorはkeyboardだけでcategory-specific target selection、Add / Rename / Dropまたはconversion action開始、operation form入力、Plan確認、Diffへの移動、ApplyまたはCancelへ到達できなければならない（MUST）。destructive confirmationをkeyboard userだけが操作不能なsurfaceにしてはならない（MUST NOT）。

## フォーカス（Focus）

### GUI-TYPE-FOCUS-001

Explorerからtype documentを開いた場合、main Type Editorへ移動できる明確なkeyboard pathを持たなければならない（MUST）。operation dialog / panelを閉じた場合は、可能な範囲で開始元setting/member/field/actionへfocusを復元しなければならない（MUST）。Plan / Diffからoverviewへ戻る場合も、可能な範囲でtarget selectionを復元しなければならない（MUST）。

## 検証（Validation）

### GUI-TYPE-VAL-001

Type Editorのtype/member/field/dependency/initializer validationはshared Type Migration / Type System semanticsを使用しなければならない（MUST）。frontend独自のYAML parser、name collision rule、numeric range/bit rule、type dependency rule、initializer compatibility ruleをdomain authorityとして実装してはならない（MUST NOT）。

Planのblocking diagnosticとproject-wideのunrelated diagnosticを同一のApply gateとして誤解釈してはならない（MUST NOT）。Type Migration Resolvableの判定はType Migration v1へ委譲する。

### GUI-TYPE-VAL-002

Plan / diagnosticがsource locationまたはaffected fileへmap可能な場合、利用者が該当source / member / fieldを識別できる形で表示しなければならない（MUST）。unmappable project-level diagnosticも不必要に失ってはならない（MUST NOT）。

## エラー（Errors）

### GUI-TYPE-ERR-001

Type Migration commitがmutation開始前に失敗した場合、またはcommit failure後にcomplete old source setへrollbackできた場合、Successとして表示してはならない（MUST NOT）。operation inputとPlan情報を可能な範囲で保持し、原因確認とre-plan / retryへのpathを提供しなければならない（MUST）。

### GUI-TYPE-ERR-002

Type Migration commitが`Recovery Required`となった場合、Type Editorはその状態を明示し、affected file stateと利用可能なrecovery informationを表示できなければならない（MUST）。cross-surfaceなsource mutation停止は[GUI app shell](../app-shell.md)の`GUI-SHELL-STATE-001` / `GUI-SHELL-CAPABILITY-001`へ委譲し、Type Editor独自の別gateを実装してはならない（MUST NOT）。

### GUI-TYPE-ERR-003

selected sourceをshared semanticsで安全にtype declarationとしてresolveできない場合、推測したcategoryで編集可能にしてはならない（MUST NOT）。structured error stateを表示し、unrelated dirty bufferやExplorer selectionを不必要に破壊してはならない（MUST NOT）。

## アクセシビリティ（Accessibility）

### GUI-TYPE-A11Y-001

type category/declaration surface、member/field list、operation controls、Plan summary、Diff navigation、destructive confirmation、error / Recovery Required stateはassistive technologyからpurposeとstateを識別できなければならない（MUST）。stateやdestructive severityを色・iconだけで伝えてはならない（MUST NOT）。

## Adapter boundary

### GUI-TYPE-ADAPTER-001

Tauri frontendはType Migration Plan derivation、YAML source patch、dependent occurrence resolution、postcondition、lost-update preflight、destructive authorization decision、rollback transactionを実装してはならない（MUST NOT）。shared Native Application ServiceをTauri commandから呼び出し、frontendはserializable view model / semantic command inputを扱うthin adapterでなければならない（MUST）。

shared Type Migration implementationが不足しているoperationをGUI内で代替実装してはならない（MUST NOT）。

## 参照artifact（Reference Artifacts）

None.

## 検証evidence

少なくとも次をfocused core/application / Tauri adapter / React workflow evidenceで検証する。

- Value Object / Enum / Flags / Custom Type selectionでresolved categoryに応じたType Editorが開き、frontendがYAML parseでcategoryを再構成しない。
- v1非対象propertyはdirect-edit可能にならない。
- Value Object conversion、Enum/Flags Add/Rename/Drop、Custom field Add/Rename/Dropがshared Type Migration command / Planへ接続される。
- Enum/Flags Addのnumeric value、Custom field Addのinitializer等がshared diagnosticsで検証される。
- `long` / `ulong`のEnum/Flags numeric valueとinitializer内scalarがfrontend/Tauri境界をlosslessにround-tripする。
- Flags `None = 0`にRename/Drop actionを提供しない。
- Plan / Diffはmutation前で、affected files / occurrence count / diagnosticsをcurrent Planから表示する。
- plan後のexternal source/config/membership changeをApplyがstaleとして拒否し、sourceを上書きしない。
- affected dirty Data Editor bufferがApplyをblockし、unrelated dirty bufferは保持される。
- destructive operationがGUI confirmationとbackend authorizationの両方を必要とする。
- successful Apply後にaffected clean snapshotがrefreshされ、unrelated dirty bufferを破棄しない。
- commit failure + rollback successとRecovery RequiredをSuccessと混同しない。
- Tauri / frontendにType System、dependency resolution、filesystem transaction、YAML patch semanticsを複製しない。

## 未解決事項（Open Questions）

None identified for Type Editor v1. exact Ant Design component、Plan panelのmodal / inline配置、member/field row action placement、numeric input widget、initializer widget / serialized wire shape、Diff layout、minor focus stylingは、`GUI-TYPE-INT-009`のlossless transportを満たす限りimplementation detailとする。
