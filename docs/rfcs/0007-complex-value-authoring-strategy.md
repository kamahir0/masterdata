# RFC: Complex Value Authoring v1 strategy

Status: Accepted

## 背景（Context）

2026-09-15、HumanはType Editor v1 Objective完了後の次priorityとしてData Editorのcomplex field record inputを選択した。Approved Data Editorはexisting recordの通常editをRequired Primitive non-key fieldへ限定し、Added recordもRequired-Primitive-only Tableへ限定している。一方、Value Object / Enum / Flags / Custom Type / Nullable / Arrayのdomain semanticsはApproved Type Systemですでに定義済みである。

したがって課題は新しいtype semanticsを作ることではなく、Approved semanticsをData Editorのauthoring、source-preserving Save、dirty buffer、validationへどう接続するかである。

## 根拠と分類（Source Evidence and Classification）

| 分類 | 内容 |
| --- | --- |
| Decision | Humanは次priorityとしてcomplex field record inputを選択した。 |
| Decision | 2026-09-16、Humanはinitial strategyとしてOption Cを選択した。 |
| Requirement | raw YAML手編集へ戻らずschema-aware Data Editorからcomplex valueをauthoringできる方向へ進める。 |
| Constraint | YAMLはSource of Truthとし、frontendでYAML domain semanticsやtype resolutionを再実装しない。 |
| Constraint | file単位dirty / Save、validation non-blocking、source provenance、lost-update prevention、Conflict / Failure / Outcome Unknownを維持する。 |
| Constraint | existing recordのPrimary / Secondary Key構成field mutationは現Objectiveへ含めない。 |

## 課題（Problem）

Primitive scalar前提のeditor transportだけではFlags / Array sequence、Nullable transition、Custom Type mapping、nested Custom Type / Array / Nullable、nested `long` / `ulong` lossless transportを一貫して扱えない。existing editとAdded record draftで別modelを作ればdomain semanticsやUI behaviorが二重化しやすい。

## 目標（Goals）

- Approved Type Systemのfield value shapeをData Editorからauthoringする。
- type resolution、lossless value representation、candidate derivation、validationをshared core/application boundaryへ置く。
- existing record editとrecord additionでvalue semanticsを共用する。
- source-preserving Save、file単位dirty lifecycle、external conflict safetyを維持する。
- key/schema/type mutationやspreadsheet bulk editingへscopeを拡散させない。

## 非目標（Non-Goals）

- existing recordのPrimary / Secondary Key mutation。
- Table schema / type declaration mutation。
- general-purpose raw YAML / JSON fragment editor。
- range paste、fill handle、bulk edit、general Undo/Redo。
- `$tags` authoring、record reorder / duplicate。
- source file operation、Build / Publish / Git operation。
- released-version compatibility system。
- exact component library / placement / spacing。

## 選択肢（Options）

### Option A: existing-record complex editだけを先行

existing non-key fieldだけcomplex editorを追加し、complex TableのAdd Rowはdisabledのままとする。work packageは小さいが、最初のcomplex recordをGUIだけで作れない断点が残る。

### Option B: scalar-like categoryから段階導入

Value Object / Enum / Nullable scalar等から先行し、Array / Flags / Custom Type / nested compositionは後続へ残す。早く一部価値を出せるがsupport boundaryがtype composition依存になり、後からshared abstractionを再設計するriskがある。

### Option C: shared schema-driven value authoring modelをexisting editとAdd Rowで共用

shared core/applicationがresolved field shapeとcurrent valueをfrontendへ提供し、frontendはdescriptorに従ってschema-aware controlを構成する。frontendはYAML parse、type lookup、Enum/Flags resolution、Custom Type reconstructionを行わない。

v1対象はApproved Type Systemがfield valueとして許すPrimitive、Value Object、Enum、Flags Enum、Custom TypeとRequired / Nullable / Array compositionとする。existing recordではkey read-onlyを維持しnon-key fieldを対象とする。Added record draftでは初回key入力を含む全supported fieldへ同じvalue authoring modelを適用する。

value editはlossless typed tree / commandとしてshared boundaryを通り、nested `long` / `ulong`もroundingしない。candidate source derivationとsource provenanceはshared Rust側が所有し、frontendはSave candidate YAMLを生成しない。

## 採用した方向（Accepted Direction）

**Option C: shared schema-driven value authoring modelを採用する。**

Human maintainerは2026-09-16、直前に自己完結的に提示された推薦Option Cに対して「進める」と回答し、このinitial strategyを選択した。

この採用はRFCをAcceptedにするdesign decisionであり、implementation authorityではない。Approved canonical ownerへのsemantic deltaは[`docs/spec-changes/0015-complex-value-authoring.md`](../spec-changes/0015-complex-value-authoring.md)でreviewし、Human Approval後にcanonical specificationへ適用する。

## 採用後に必要なcanonical change

- `docs/specs/source-edit.md`: resolved complex value authoring、nested lossless representation、source-preserving patch boundary。
- `docs/specs/source-record-mutation.md`: Required-Primitive-only Add Record制限の拡張。
- `docs/gui/data-editor/spec.md`: existing non-key complex fieldのeditable scope、schema-driven editor、nested diagnostics。
- `docs/gui/data-editor/record-mutation.md`: complex Table Add Rowとexisting/new record共通value model。

## 互換性（Compatibility）

YAML syntax、Table identity、MessagePack key、Type System、generated C#、binary formatは変更しない。existing record key mutationも対象外のままである。

source text compatibilityはstructural complex edit時のtarget subtree内部preservation policyに依存する。Added recordのsource bytesは未入力draft valueのrepresentationにも依存する。これらはspec-change 0015のHuman decisionとして分離する。

## 未解決事項（Open Questions）

- structural complex editでtarget subtree内部のunchanged comment / style / bytesをfine-grainedに保持するか、target subtree replacementを許すか。
- Added record draftの未入力typed valueをYAML `null`として表現するか、sourceへ表現しないlocal-only `Unset`としてSave前に解消を要求するか。

exact editor presentationはdata safety / compatibilityを変えない範囲でRFC 0005のGUI refinement delegationに従う。

## 決定（Decision）

2026-09-16、Human maintainerは**Option C: shared schema-driven value authoring model**をComplex Value Authoring v1のdesign directionとして採用した。canonical behaviorはspec-change 0015のapproval / application後に初めてimplementation authorityとなる。
