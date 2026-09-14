# Type Migration v1仕様

Status: Approved

Domain: Type Migration

## 概要

この文書は、既存Value Object / Enum / Flags Enum / Custom Type declarationを、canonical YAMLをSource of Truthのままproject-wideに安全に変更するType Migration v1のsemantic contractを定義する。

Type category、declaration、data representation、generated C# semanticsは既存の[Type System](type-system/README.md) familyが所有する。本仕様はそれらを再定義せず、type-definition mutationのoperation set、dependency resolution、Plan / Diff、source-preserving rewrite、stale-plan preflight、destructive authorization、およびType Migration operationからshared source-set commit protocolへのcompositionを所有する。[Schema Migration v1](schema-migration.md)のTable field mutationとは別のcanonical ownerであり、Type-specific semanticsを`MIGRATION-*`へ追加しない。複数YAML file commitのobservable Success / rollback / Recovery Required state自体は、既存の`MIGRATION-010`を再利用し、本仕様で重複定義しない。

GUI compositionは[Type Editor v1](../gui/type-editor/spec.md)が所有する。public CLI grammar、specific Rust struct layout、wire format、filesystem journal layoutは本仕様で固定しない。

## 用語

- **Type Migration**: canonical source snapshot上のlogical type declarationへsemantic operationを適用し、依存するcanonical sourceを含めてdeterministicに変換するoperation。
- **Type Migration resolution closure**: target declaration、operation validation、dependent value occurrence分類、patch location、postcondition確認に必要なcanonical source集合。
- **Type Migration Plan**: mutation前に生成するdeterministicなoperation summary、affected source、affected value occurrence count、diagnostics、および同一candidateに対応するsource Diffのbase identityを含む計画。
- **Execution authorization**: destructive operationのcommitを許可するmachine-actionableな実行時許可。semantic commandやGUI confirmationとは別conceptである。
- **Value occurrence**: Table record、nested Custom Type、Array等を含むcanonical data source内で、shared Type System resolutionにより対象typeまたはmember/fieldへ解決されたvalue位置。

## 規範要件

### TYPE-MIGRATION-001

Type Migrationのauthorityはcanonical YAML sourceと、そこからshared parser / Type System / Table semanticsを用いて構成したType Migration resolution closureでなければならない（MUST）。generated C#、canonical binary、artifact-set receipt、binary inspection resultをsource authorityとして使用してはならない（MUST NOT）。physical file pathはprovenance/storage locationであり、type declaration identityそのものとして扱ってはならない（MUST NOT）。

### TYPE-MIGRATION-002

v1で成功operationとして扱ってよいType Migration Operationは次だけでなければならない（MUST）。

- `SetValueObjectConversions`
- `AddEnumMember`
- `RenameEnumMember`
- `DropEnumMember`
- `AddCustomField`
- `RenameCustomField`
- `DropCustomField`

v1ではtype declaration name変更、Value Object / Enum / Flagsのunderlying変更、Enum / Flags member numeric value変更またはreorder、Custom Type field type / modifier / MessagePack key変更またはreorder、type category conversionを成功operationとして扱ってはならない（MUST NOT）。

### TYPE-MIGRATION-003

Type Migration Commandはtext edit命令ではなく、shared Type Systemでresolve可能なlogical type declaration target、operation-specific selector、およびsemantic argumentsを表さなければならない（MUST）。member / field renameまたはdropではcurrent member / field nameを対象snapshot上のselectorとして使用する。path、line number、raw substringだけをsemantic target identityとして扱ってはならない（MUST NOT）。

public serialized command schema、CLI grammar、specific Rust enum/struct layoutは本仕様では固定しない。

### TYPE-MIGRATION-004

同じproject config、同じcanonical source snapshot、同じType Migration resolution closure、同じsemantic command、および同じexecution optionsから生成されるtransformed semantic result、Plan、affected source setはdeterministicでなければならない（MUST）。環境依存のpath traversal順序、AI生成値、implicit numbering、暗黙のconversion guessによって結果を変えてはならない（MUST NOT）。

Type Migrationは少なくとも次のsemantic flowを満たさなければならない（MUST）。

```text
resolve project / canonical source snapshot
→ resolve target type declaration and migration closure
→ validate semantic command and operation-specific preconditions
→ construct expected transformed semantic state
→ construct deterministic source patch candidate
→ apply patches in memory
→ canonical reparse / Type System / dependent semantics resolution
→ verify operation-specific postconditions
→ construct deterministic Plan and per-file before/after Diff candidate
→ execution authorization checks
→ stale-plan preflight
→ multi-file source mutation commit
```

text patchが生成できただけで成功扱いしてはならない（MUST NOT）。

### TYPE-MIGRATION-005

`SetValueObjectConversions`は、対象Value Objectの`fromUnderlyingImplicit`と`toUnderlyingImplicit`だけを変更できなければならない（MUST）。underlying primitive、type name、data representation、key/comparison/equality capabilityを変更してはならない（MUST NOT）。settingのmeaningとdefaultは`SCHEMA-VO-008` / `SCHEMA-VO-009`へ委譲する。

### TYPE-MIGRATION-006

`AddEnumMember`はNormal EnumまたはFlags Enumへexplicit member nameとexplicit numeric valueを追加しなければならない（MUST）。implicit numberingまたはauto-incrementを導入してはならない（MUST NOT）。name、numeric value、underlying range、Flags atomic-bit rule、generated-name collision等は[Enum / Flags仕様](type-system/enums.md)のcurrent Approved semanticsで検証しなければならない（MUST）。

追加memberは既存member sequenceの末尾へappendし、既存memberをreorderしてはならない（MUST NOT）。このappend ruleはsource-preserving deterministic mutationのためのv1 behaviorであり、member declaration orderへ新しいdomain identityを与えるものではない。

### TYPE-MIGRATION-007

`RenameEnumMember`は対象snapshot上のcurrent member nameをselectorとし、member numeric valueとdeclaration positionを保持したままdeclaration nameを変更しなければならない（MUST）。対象memberへshared semanticsでresolveされた既存symbolic data occurrenceを、Normal Enum scalar、Flags sequence、nested Custom Type、Arrayを含めてnew nameへ更新しなければならない（MUST）。

rename後のname validityおよびcollisionはcurrent Enum / Flags / C# naming semanticsで検証する。numeric value、underlying、member orderを変更してはならない（MUST NOT）。Flagsの`None = 0`をrenameしてはならない（MUST NOT）。

### TYPE-MIGRATION-008

`DropEnumMember`はdestructive operationでなければならない（MUST）。対象memberへresolveされたexisting data occurrenceが1件でも存在する場合、replacement memberまたはnumeric valueを推測せずprecondition failureとしてfail closedしなければならない（MUST）。

existing data occurrenceが0件の場合だけ、explicit machine-actionable destructive execution authorization付きでdeclarationからmemberを削除できる（MAY）。authorizationがない場合はPlanを生成してよいがsource mutationを開始してはならない（MUST NOT）。Flagsの`None = 0`はdrop対象にしてはならない（MUST NOT）。

### TYPE-MIGRATION-009

`AddCustomField`はexplicit MessagePack `key`、field `name`、base `type`、field modifierを持つfield declarationを追加しなければならない（MUST）。declaration validity、field count、type dependency cycle、field modifier、MessagePack key、generated namingは既存Approved ownerへ委譲しなければならない（MUST）。

対象Custom Typeのexisting value occurrenceが1件以上ある場合、operationはexplicit constant initializerを要求しなければならない（MUST）。initializerは対象fieldのApproved canonical data representationとしてvalidなconstant valueでなければならず（MUST）、すべてのexisting Custom Type mapping occurrenceへ同じvalidated valueを導入しなければならない（MUST）。expression、other-field reference、function call、recordごとのAI-generated value、implicit runtime/language defaultをinitializerとして使用してはならない（MUST NOT）。existing occurrenceが0件の場合はinitializerを省略してよい（MAY）。

新field declarationは既存`custom.fields` sequenceの末尾へappendし、existing mappingへmemberを追加する場合もexisting memberをreorderしてはならない（MUST NOT）。

### TYPE-MIGRATION-010

`RenameCustomField`は対象snapshot上のcurrent field nameをselectorとし、MessagePack `key`、base type、modifier、declaration positionを保持したままdeclaration nameを変更しなければならない（MUST）。対象fieldへresolveされたすべてのexisting Custom Type mapping occurrenceについてmember nameを更新しなければならない（MUST）。nested Custom TypeまたはArray内のoccurrenceも同じsemantic resolutionで扱う。

raw textの同名memberをproject-wideに文字列置換してはならない（MUST NOT）。new nameのvalidity/collisionはcurrent Custom Type / C# naming semanticsで検証しなければならない（MUST）。

### TYPE-MIGRATION-011

`DropCustomField`はdestructive operationとして、対象field declarationと、そのfieldへresolveされたすべてのexisting Custom Type mapping occurrenceから該当memberを削除しなければならない（MUST）。commitにはexplicit machine-actionable destructive execution authorizationが必要であり（MUST）、GUI confirmationだけをauthorizationの代替にしてはならない（MUST NOT）。

field削除後のCustom Type declarationがcurrent Approved Type System semanticsを満たさない場合、またはdependent sourceを安全に分類/変換できない場合はfail closedし、source mutationを開始してはならない（MUST NOT）。

### TYPE-MIGRATION-012

Type Migrationはsource mutation前にdeterministicなPlanを生成可能でなければならない（MUST）。Planは少なくともoperation、target type、target member/fieldまたはsetting、destructive state、affected source files、affected value occurrence count、validation resultまたはstructured diagnosticsをconceptually表現できなければならない（MUST）。Plan作成はcanonical sourceをmutationしてはならない（MUST NOT）。

Planはaffected fileごとのexact before/after Diff candidateを導出可能でなければならず（MUST）、DiffのbeforeはPlan作成に使用したexact source bytes、afterは同じPlan candidateのtransformed bytesでなければならない（MUST）。current workspaceの別readやgenerated artifactをDiffの代替にしてはならない（MUST NOT）。

### TYPE-MIGRATION-013

Type Migrationのsource rewriteはsource-preservingかつdeterministicでなければならない（MUST）。変更不要fileはbyte-for-byte unchangedでなければならず（MUST）、affected fileでもoperationと無関係なcomments、blank lines、member ordering、quote/indentation等を不必要に再serializeしてはならない（MUST NOT）。

source位置を安全に特定できない、同一semantic targetへのpatchがambiguous、またはcanonical parser resultとsource provenanceを対応付けられない場合はfail closedしなければならない（MUST）。formatterまたは全source再serializeをType Migration成功経路の代替にしてはならない（MUST NOT）。

### TYPE-MIGRATION-014

patched sourceはcanonical parserで再parseし、shared Type Systemおよびoperationに必要なdependent Table/value semanticsで再resolveし、expected transformed semantic stateとoperation-specific postconditionを確認しなければならない（MUST）。

Project全体がerror-freeであることをType Migration success preconditionにしてはならない（MUST NOT）。closure外でmigrationと無関係であることを安全に分類できるdiagnosticだけを理由にrejectしてはならない（MUST NOT）。一方、sourceがtarget/dependencyへ関係するか安全に分類できない場合はunrelatedと推測せずblockingとして扱わなければならない（MUST）。

### TYPE-MIGRATION-015

Apply直前のstale-plan preflightは、Plan作成に使用したproject config、canonical source membership、およびresolution / dependency classification / Diff生成に使用したexact source bytesがcurrent workspaceと一致することを確認しなければならない（MUST）。不一致を検出した場合はsource mutationを開始せずstale planとして失敗しなければならない（MUST）。silent re-plan、silent overwrite、current bytesへold patchをbest-effort適用してはならない（MUST NOT）。

### TYPE-MIGRATION-016

Type Migrationが複数YAML fileのsource mutation commitへ進む場合、observable source-set stateとrollback / Recovery Required semanticsは[Schema Migration v1](schema-migration.md)の`MIGRATION-010`へ従わなければならない（MUST）。Type Migration固有の別transaction state machineを導入してはならず（MUST NOT）、`Recovery Required`をSuccessとして扱ったり、rollback failure後にintentional source mutationを継続してはならない（MUST NOT）。

Type Migration implementationは`MIGRATION-010`のobservable protocolをshared application/host boundaryへ返し、[GUI app shell](../gui/app-shell.md)の既存Recovery Required gateとcompositionできなければならない（MUST）。backup layout、journal format、staging directory、exact recovery command、process crash / OS crash / power loss保証は既存ownerの境界を越えて本仕様で再定義しない。

### TYPE-MIGRATION-017

Type Migration成功はBuild、Publish、Git commit/push、generated C#、canonical binary、artifact-set receiptの更新を暗黙に開始してはならない（MUST NOT）。source mutation成功後のBuild / Publish / Gitは既存ownerの別operationとして明示的に実行する。

### TYPE-MIGRATION-018

Type Migration implementationはdependency resolution、source patch、postcondition、stale-plan preflight、destructive authorization、multi-file commit integrationをshared core/application boundaryへ置かなければならない（MUST）。GUI、Tauri adapter、将来のCLI/Web adapterが独自にYAMLをparse/rewriteし、Type Migration semanticsを複製してはならない（MUST NOT）。

## 検証ルール

少なくとも次をfocused core/application regressionで確認する。

- Value Object conversion settingだけを変更し、underlying/data semanticsを変更しない。
- Enum/Flags member追加がexplicit numeric valueを要求し、current Enum/Flags validationへ従う。
- Enum/Flags renameがnumeric value/orderを保持し、Normal Enum、Flags sequence、nested/array occurrenceをsemanticに更新する。
- existing occurrenceがあるEnum/Flags member dropはfail closedし、未使用memberもdestructive authorizationなしではcommitしない。
- Flags `None = 0`をrename/dropできない。
- Custom field addがexisting occurrenceにexplicit initializerを要求し、すべてのmapping occurrenceへ同じvalidated valueを追加する。
- Custom field renameがMessagePack key/type/modifier/orderを保持し、nested occurrenceを含めてsemanticに更新する。
- Custom field dropがdeclaration/data occurrenceを一貫して削除し、authorizationなしではcommitしない。
- Plan / Diffはmutation前で、同一base snapshot/candidateに対応する。
- unrelated invalid sourceは安全に分類できる限りblockせず、unclassifiable sourceはfail closedする。
- external source/config/membership changeをstale preflightが拒否し、old Planで上書きしない。
- unaffected file bytesとaffected fileのunrelated source textを保持する。
- multi-file failureが`MIGRATION-010`のcomplete OLD rollbackとRecovery Requiredを区別し、Type Migration固有の別state machineを作らない。
- successがBuild / Publish / Git / generated artifact更新を起動しない。

## 互換性

Type Migrationはcurrent source schemaを安全に変換するauthoring operationであり、released-version compatibility systemを導入しない。

v1の各operationはsource migrationとして成功しても、generated C#を利用するexternal consumerのsource / API compatibilityを保証しない。`SetValueObjectConversions`はimplicit conversion operatorの有無、Enum/Flags member Add/Rename/Dropはgenerated enum member surface、Custom Type field Add/Rename/Dropはpropertyおよびpublic constructor surfaceを変更し得る。Type Migration Planの成功条件はcurrent canonical sourceとcurrent Approved Type System semanticsに対する変換整合性であり、released external consumerの互換性判定ではない。

v1でtype declaration rename、underlying変更、member numeric value変更、Custom Type field type/modifier/key変更を除外するのは、serialization/API compatibilityとarbitrary data conversion policyを本sliceへさらに拡張しないためである。この限定も、v1 operationそのものがexternal compatibilityを保証する意味ではない。

Enum/Flags numeric valueはcurrent schema member valueでありpersistent wire identityではないという`SCHEMA-ENUM-001`を変更しない。Custom Type field MessagePack `key`の意味も既存`SCHEMA-KEY-001`を変更しない。

## 例

以下はnon-normativeなoperation intent例である。wire formatを固定しない。

```text
RenameEnumMember(type = ItemRarity, member = Rare, newName = Epic)
AddCustomField(type = Reward, key = 2, name = note, type = string, modifier = Nullable, initializer = null)
```

## 未解決事項（Open Questions）

None identified for Type Migration v1. specific Rust API、wire schema、journal layout、diagnostic code spelling、patch helper decompositionは、上記observable contractを満たす限りimplementation detailとする。

## 非目標

- type declaration name rename。
- Value Object / Enum / Flags underlying変更。
- Enum / Flags member numeric value変更またはreorder。
- Custom Type field type / modifier / MessagePack key変更またはreorder。
- type category conversion。
- arbitrary migration scripting / SQL-like language。
- released-version compatibility system全体。
- Build / Publish / Gitとの自動結合。
