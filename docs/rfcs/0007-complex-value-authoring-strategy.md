# RFC: Complex Value Authoring v1 strategy

Status: Proposed

## 背景（Context）

2026-09-15、HumanはType Editor v1 Objective完了後の次priorityとして、Data Editorのcomplex field record inputを選択した。

現在のApproved Data Editorでは、base snapshotに存在するexisting recordの通常cell editはRequired Primitiveの非key fieldに限定され、Nullable / Array / Enum / Flags Enum / Value Object / Custom Type等はread-onlyである。Added record draftについても、Source Record MutationとData Editor Record Mutationは全fieldがRequired PrimitiveのTableだけをinitial Add Row scopeとしている。

一方、complex valueのdomain semanticsは既にApproved Type Systemで定義されている。

- Value Objectはunderlying Primitiveと同じscalar representationを使う。
- Normal Enumはsymbolic member nameを使う。
- Flags Enumはsymbolic member nameのsequenceを使う。
- Custom Typeはschema field nameをmemberとするmappingを使う。
- field modifierはRequired / Nullable / Arrayを定義し、Arrayはbase typeのordered sequence、Nullableは`null`またはbase valueを表す。

したがって今回の中心課題は新しいdomain typeを設計することではなく、これらのApproved semanticsをData Editorのauthoring model、source-preserving Save、dirty buffer、validationへどう接続するかである。

## 根拠と分類（Source Evidence and Classification）

| 分類 | 内容 |
| --- | --- |
| Decision | Humanは次priorityとしてcomplex field record inputを選択した。 |
| Requirement | raw YAML手編集へ戻らず、schema-awareなData Editorからcomplex valueを入力できる方向へ進める。 |
| Constraint | YAMLはSource of Truthのままとし、frontendでYAML domain semanticsやtype resolutionを再実装しない。 |
| Constraint | file単位dirty / Save、validation non-blocking、source provenance、lost-update prevention、Conflict / Failure / Outcome Unknown等のApproved authoring safetyを維持する。 |
| Constraint | existing recordのPrimary / Secondary Key構成field mutationは現Objectiveへ含めない。 |
| Open Question | complex supportをexisting recordだけへ先行導入するか、type categoryごとに段階導入するか、existing editとAdded record draftを共通modelで扱うか。 |

## 課題（Problem）

Primitive scalarだけを前提にした現在のeditor transportを単純に広げるだけでは、complex value authoringを安全に閉じられない。

- FlagsとArrayはsequenceであり、Custom Typeはnested mappingである。
- Nullableでは`null`とnon-null valueのstate transitionがある。
- Custom Typeはnested Custom Type / Array / Nullableを含み得る。
- `long` / `ulong`はnested位置でもlosslessでなければならない。
- existing sourceのnested valueを編集する場合、full-file reserializationへ逃げずsource-preserving patchを維持する必要がある。
- Added record draftだけ別のvalue representationを持つと、existing editとnew-record authoringでdomain semanticsやUI behaviorが二重化しやすい。

frontendがschema textやYAML nodeを直接解釈してこれらを解決すると、Approved shared Rust semantic boundaryを破る。逆にbackendへraw YAML fragmentを渡すだけでは、schema-aware authoringというProduct Vision上の価値を十分に満たさない。

## 目標（Goals）

- Approved Type Systemのvalue shapeをData Editorからraw YAML手編集なしでauthoringできる方向を決める。
- complex valueのtype resolution、lossless representation、candidate derivation、validationをshared core/application boundaryへ置く。
- existing record editとrecord additionのscopeを明示し、同じvalue semanticsを重複実装しない。
- source-preserving Save、file単位dirty lifecycle、external conflict safetyを維持する。
- v1をbounded work packageに保ち、key/schema/type mutationやspreadsheet bulk editingへ拡散させない。

## 非目標（Non-Goals）

本RFC自体はproduct specificationでもimplementation authorityでもない。また次をこのRFCだけで承認しない。

- existing recordのPrimary / Secondary Key mutation。
- Table schema / type declaration mutation。
- raw YAML / JSON fragment editorをgeneral-purpose escape hatchとして導入すること。
- range paste、fill handle、bulk edit、general Undo/Redo。
- `$tags` authoring、record reorder / duplicate。
- source file operation、Build / Publish / Git operation。
- released-version compatibility system。
- exact widget library、popover/modal placement、spacing等のroutine interaction detail。

## 選択肢（Options）

### Option A: existing-record complex editだけを先行する

existing recordの非key fieldについてcomplex value editorを追加する。Added record draftは現在のRequired-Primitive-only制約を維持し、complex fieldを含むTableではAdd Rowをdisabledのままにする。

shared applicationはexisting valueのresolved shapeとedit commandを提供し、frontendはそのdescriptorからeditorを構成する。

利点:

- record additionのsource rendering / draft construction変更を同時に扱わずに済み、work packageが小さい。
- existing Data Editorの編集断点を先に減らせる。

欠点:

- empty Data documentや新規recordではcomplex valueをGUIから入力できず、raw YAMLへの断点が残る。
- existing value editorとAdded record draft inputの2段階導入になり、後続でrepresentation統合が必要になりやすい。
- Type / TableをGUIで作成できても、complex schemaを持つ最初のrecordをGUIだけで作れない。

### Option B: scalar-like categoryから段階導入する

Value Object、Normal Enum、Nullable scalar等、cell内のsingle-value controlへ落としやすいcategoryを先にsupportする。Array、Flags、Custom Typeおよびnested compositionはread-only / Add Row unsupportedのまま残し、後続sliceで追加する。

利点:

- 最短で一部のcomplex typeをeditableにできる。
- nested structural editorとsource patch設計を後回しにできる。

欠点:

- 「complex fieldを編集できる/できない」の境界がtype compositionに依存し、利用者にとって予測しづらい。
- category追加ごとにsnapshot capability、frontend control、draft construction、testsを繰り返し拡張する可能性が高い。
- Custom TypeやArrayを使う実用schemaではraw YAMLへの断点が残る。
- shared authoring abstractionを後から再設計するriskがある。

### Option C: shared schema-driven value authoring modelを導入し、existing editとAdd Rowで共用する

shared core/applicationが、resolved field shapeをfrontend向けのtyped value descriptorとして公開し、同じmodelでexisting valueとAdded record draftを表現する。frontendはdescriptorに従ってschema-aware controlをrenderし、YAML parse、type lookup、Enum/Flags resolution、Custom Type shape reconstructionを行わない。

v1の対象は、Approved Type Systemがfield valueとして許すPrimitive、Value Object、Enum、Flags Enum、Custom Typeと、それらに対するRequired / Nullable / Array compositionとする。existing recordでは現在のkey read-only ruleを維持し、non-key fieldだけを編集対象とする。Added record draftでは既存Record Mutation contractに従い、record定義に必要なkey fieldを含む全supported fieldへ同じvalue authoring modelを適用する。

value edit requestはlosslessなtyped tree / commandとしてshared boundaryを通り、nested `long` / `ulong`もtextまたはexact integer representationを維持する。candidate source derivationとsource provenanceはshared Rust sideが所有する。frontendはSave candidateのYAMLを生成しない。

source preservationでは、変更対象valueのlogical pathとsource provenanceから必要な範囲だけpatchする。unchanged sibling field、record、file-level comment / formattingを再serializeしない。Array要素追加・削除、Nullable state transition等でstructural replacementが必要な場合も、full-file serializerへfallbackせず、対象value subtreeより外側を変更しない方針をcanonical specificationで具体化する。

利点:

- existing editとnew-record authoringが1つのvalue modelを共有し、domain semanticsの二重化を避けられる。
- GUIでschema/typeを作った後、complex schemaの最初のrecordまでraw YAMLなしでauthoringできる。
- Desktop以外のfuture authoring adapterでもshared representationを再利用できる。
- nested Custom Type / Array / Nullableを後付け特例ではなくcompositionとして扱える。

欠点:

- Option A/Bよりcore/applicationとfrontendの初期変更量が大きい。
- nested source provenanceとstructural patchのcontractを先に明確化する必要がある。
- editor component recursion、focus、diagnostic path mapping等のtest surfaceが増える。

## 推薦（Recommendation）

**Option C: shared schema-driven value authoring model**を推薦する。

理由は、今回のObjectiveを「一部typeをeditableにする」ではなく「Type Systemで定義した値をData Editorから実際にauthoringできる」縦切りとして閉じられるためである。Type Editorまでshared semantic boundaryで統一した直後にfrontend-owned type semanticsやcategory別の一時modelを増やすより、最初からexisting/new recordで共通のvalue authoring boundaryを置く方が長期方向と整合する。

ただしOption Cの採用はRFCを`Accepted`にするだけであり、implementation authorityにはならない。採用後にApproved canonical ownerへのdeltaを`docs/spec-changes/`でreviewし、Human Approvalを受ける必要がある。

## 採用後に必要なcanonical change

Option Cを採用する場合、少なくとも次をchange artifactでreviewする。

- `docs/specs/source-edit.md`
  - existing complex value editのlossless typed representation、nested source provenance、source-preserving subtree patch boundary。
- `docs/specs/source-record-mutation.md`
  - Required-Primitive-only Add Record制限を、supported resolved field value shapeへ拡張するdelta。
- `docs/gui/data-editor/spec.md`
  - existing non-key complex fieldのeditable scope、schema-driven editor、nested diagnostics / focus behavior。
- `docs/gui/data-editor/record-mutation.md`
  - complex TableでのAdd Row、Added record draftのtyped input、existing/new record共通value model。

既存Requirement IDを維持できるapplicability refinementか、新Requirementが必要なsemantic additionかはchange artifact refinement時に個別に決定する。

## 互換性（Compatibility）

本RFCはYAML syntax、Table identity、MessagePack key、Type System、generated C#、binary formatを変更しない。authoring capabilityの拡張であり、保存後のsource value semanticsは既存Approved Type Systemに従う。

source text compatibilityについては、complex subtreeの編集でどこまでoriginal comments / styleを保持するかがobservable authoring behaviorになる。Option Cでは少なくとも対象value subtree外のsourceを変更しないことを方向性とし、subtree内部の最小patch contractはcanonical change reviewで固定する。

existing record key mutationは対象外のままなので、Primary / Secondary Key semanticsを変更しない。

## 未解決事項（Open Questions）

- HumanはOption A / B / Cのどのinitial strategyを採用するか。
- Option C採用時、complex subtree内部のcomment / style preservationをleaf-levelで必須にする範囲と、structural operation時にtarget subtree canonical renderingを許す範囲をcanonical changeでどう定義するか。
- exact editor presentation（inline、popover、drawer等）は、data safety / accessibilityを変えない範囲でGUI refinement detailとしてどこまで委任するか。

## 決定（Decision）

未決定。Human maintainerの選択を待つ。
