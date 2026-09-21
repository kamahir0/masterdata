# 仕様変更: Reference v1

Status: Proposed

## Affected Specifications

- [Index / Reference](../specs/index-and-reference.md) `REF-001..007`: Option Bのcore semanticsをReference v1のcanonical ownerへ適用する。generated helper public API gateは本changeに残る。
- [Table / Primary Key / Secondary Key](../specs/table-and-keys.md): existing Primary/Secondary identity、Build Selection後のReference integrity順序を変更せず参照する。
- [Schema language](../specs/schema-language.md): approved Reference surfaceへのrouting/exampleを同期する。
- [Build Selection](../specs/build-selection.md): selected logical dataset上のReference validation順序を既存contractどおり使用する。
- [Table Editor](../gui/table-editor/spec.md): Reference declarationのguided authoringを追加する。
- C# generation: resolved ReferenceをMasterMemory query APIへlowerする。exact public helper contractは本changeで決定する。

## 根拠と分類（Source Evidence and Classification）

- Human Decision: 2026-09-21 JST、Web退役後の次product priorityとしてReference方向へ進み、Reference v1はOption B（Required ReferenceとNullable Referenceの両方）を採用する。Option AとOption CはRejected alternativeとする。
- Existing Draft: `REF-001`はsource fieldとtarget table/index、`REF-002`はunique/non-unique cardinality、`REF-003`はbuild validationとcaller-supplied `MemoryDatabase` helperを要求する方向を保持している。
- Approved Constraint: Secondary Key v1のlogical identityはordered current field-symbol sequenceであり、persistent nameを持たない（`INDEX-SECONDARY-002`）。generated `indexNo`とMessagePack `key`はsemantic identityではない。
- Approved Constraint: Build Selection後にPK/unique constraints、Reference integrity、canonical ordering、binary buildの順で評価する（`SCHEMA-TABLE-007` / Build Selection owner）。
- External evidence: MasterMemory v3はPrimary/Secondary Keyからtyped `FindBy...` queryを生成し、unique keyはsingle row、`NonUnique` keyは`RangeView<T>`を返す。Reference helperはこの既存APIを利用し、MasterMemory internalsを再実装しない。
- Agent Decision: Reference targetはbackend ordinalではなく `target.table + target.fields` でresolveする。target fieldsがPrimary Key sequenceと一致すればPrimary、そうでなければexactly matching Secondary Keyへresolveする。

## 提案する差分（Proposed Delta）

### 共通semantic core

ReferenceはTable schema上のschema-level declarationとする。field-level annotationへ限定するとcomposite keyを自然に表現できず、既存Primary/Secondaryのordered field sequence modelとも非対称になるため採用しない。

推奨surface:

```yaml
kind: schema
table: item
fields:
  - key: 0
    name: id
    type: ItemId
  - key: 1
    name: categoryId
    type: CategoryId
primaryKey:
  fields: [id]
references:
  - name: category
    fields: [categoryId]
    target:
      table: item-category
      fields: [id]
```

- `name`はTable内でuniqueなlanguage-independent Reference declaration nameとする。C# helper名そのものではなく、Referenceのdomain-facing presentation名であり、MessagePack identity、field identity、released compatibility identityにはしない。
- C# helperは`csharpName`省略時に`name`からdeterministically生成する。必要な場合だけoptional `csharpName`でgenerated C# method identifier全体をoverrideできる。C#固有presentationをReferenceのdomain semanticsやtarget identityへ使用してはならない。
- source `fields`はordered field-symbol sequence。target `fields`とcardinalityが一致しなければならない。
- targetはproject-local `table`と、そのTableのPrimary KeyまたはSecondary Keyと完全一致するordered `fields`で指定する。
- target `indexNo`、MessagePack `key`、generated C# name、file pathをtarget identityとして使用しない。
- source componentとtarget key componentは同じapproved key value semanticsで比較可能でなければならず、lossy conversionを挟まない。
- target Primary Keyまたはunique Secondary Keyはsingle Reference、`nonUnique: true` Secondary Keyはmulti Referenceへresolveする。
- selected source recordごとに、selected target logical datasetにmatching targetが存在しなければReference integrity errorとする。multi Referenceも0件をmissingとしてerrorにする。
- Reference validationはBuild Selection後、canonical ordering/binary build前に行う。
- Referenceはgenerated artifactやbinaryをauthorityにせずcanonical YAML + resolved Table modelをauthorityとする。

### Option A — Rejected alternative: Required-only Reference v1

- source `fields`はすべてRequired scalarかつkey-compatibleでなければならない。
- Nullable / Array source fieldはReference componentに使用しない。
- null-reference semanticsをv1へ導入しない。
- generated helperはsource row partial classへ `Get<Name>(MemoryDatabase database)` を生成する。
- unique targetはtarget rowを返し、non-unique targetはMasterMemory `RangeView<Target>`を返す。
- helperはDBをfield/propertyへ保持せず、毎回callerから受け取る。
- helper bodyはtarget Tableの既存generated `FindBy...` APIを呼ぶだけとし、独自index lookupを生成しない。
- helper名またはsignature collisionはvalidation/codegen errorとし、自動suffixで修復しない。

このOptionはnullableのpartial-null policyをv1 scopeから明示的に外し、composite/unique/non-uniqueとhelper APIを一つのcoherent sliceで完成させる。

### Option B — Applied Human decision: Nullable Referenceをv1へ含める

Option Aに加え、source componentsを全てNullableにしたReferenceを許可する。Reference v1のcanonical semanticsは次のとおりとする。

- 全component null: Referenceなし。
- 全component non-null: 通常lookup。
- compositeで一部だけnull: validation error。
- Required compositeは全componentがRequired scalar、Optional compositeは全componentがNullable scalarでなければならない。Required/Nullableの混在はrejectする。
- Array fieldはReference source componentとしてrejectする。
- nullable modifierを除いたbase semantic typeをtarget key componentと照合し、implicit conversionを行わない。
- Optional Referenceの全null値はselected target dataset lookupを行わず、missing diagnosticを生成しない。
- non-null値のtargetが0件なら、unique/non-uniqueを問わずbuild-blocking missing-reference diagnosticとする。
- non-unique targetの複数matchは正常であり、0件だけをmissingとして扱う。
- targetはPrimary Key、unique Secondary Key、またはnon-unique Secondary Keyのordered field sequenceへexactly resolveする。
- generated helperのunique/non-unique query loweringはMasterMemoryのgenerated queryへ委譲する。ただし公開helper名とOptional non-uniqueのabsence representationは別Human gateとして未確定のままとする。

利便性は高いが、validation、projection、GUI、C# signature、optional multi-reference semanticsがv1で増える。

### Option C — Rejected alternative: Integrity-only first

source declarationとbuild validationだけ先行し、generated helperを後続へ送る。

implementation sliceは小さいが、既存`REF-003`のcaller-supplied helper方向と利用者価値を分断するため非推奨。

## 互換性（Compatibility）

新しい`references` propertyは既存schemaへ加えるadditive surfaceで、Referenceを使用しない既存projectのYAML / generated C# / binary behaviorを変更しない。

Referenceを追加したschemaではgenerated public helper APIが増える。Reference `name`やtarget field sequence変更はgenerated API / semantic relationshipへ影響し得るが、released-version compatibility classificationは本Objective外とし、既存projectへstable identity policyを暗黙追加しない。

Secondary Keyのdeclaration reorderでbackend `indexNo`が変わってもReference targetはordered field sequenceでresolveするためsemantic targetは変わらない。

## 受け入れと実装への影響（Acceptance and Implementation Impact）

Human decision後、少なくとも以下をevidence化する。

- schema parse: single/composite Reference declaration。
- target resolution: Primary Key、unique Secondary、non-unique Secondary。
- invalid target: unknown table、unknown/duplicate source field、target fieldsがindexに一致しない、component count/type mismatch。
- Build Selection: source/target両方がselected datasetに基づき、excluded targetだけではmissingになる。
- integrity: unique target exactly one、non-unique target one-or-more。missingはbuild-blocking。
- generated helper: `MemoryDatabase`を引数に取り、unique/non-uniqueで正しいMasterMemory query/return typeを生成し、generated C#がcompileする。
- Table Editor: Reference add/edit/removeがshared semanticsへ接続され、frontend独自target resolutionを持たない。
- existing project / Build / migration / Publish regressionsなし。

## 未解決事項（Open Questions）

None for Reference v1 public helper contract.

Human decision: Referenceの`name`はlanguage-independent domain nameとして保持する。C# helperは`csharpName`省略時に`Get` + `name`の先頭ASCII lowercase letterのみuppercaseしたidentifierを生成し、`csharpName`指定時はその値をgenerated C# method identifier全体としてexactly使用する。field名やtarget Table名からhelper名を推測してはならない。

Optional + non-unique helperは`RangeView<Target>`を返し、Reference absent（全source component null）の場合は`RangeView<Target>.Empty`を返す。non-null Referenceの0件matchは引き続きbuild-blocking missing Referenceであり、empty resultで正常化しない。

target identityにbackend `indexNo`やSecondary Key nameを導入する案は、既存Approved identity contractと矛盾するためchoiceとして扱わない。

## レビュー（Review）

review-spec pass: core declaration、target identity、selection order、nullable semantics、MasterMemory delegation、およびHumanが確定したdomain name / C# presentation separationとOptional non-unique absence contractは既存authorityに整合する。Reference v1について未解決Human gateはない。

## 承認記録（Approval Record）

Option B Human decision: 2026-09-21 JST。Option A / Option C: Rejected alternative。
Helper API Human decision: 2026-09-21 JST。Reference `name`をlanguage-independent domain nameとして保持し、C#はdefault `Get<Name>` + optional exact `csharpName` overrideとする。Optional non-unique absenceは`RangeView<T>.Empty`とする。
Core semantic application: Option Bのcore semantic sliceはApproved [Index / Reference](../specs/index-and-reference.md)へ適用済み。helper public API decisionも確定したため、implementation / verification完了後にこのchange全体をAppliedへ遷移する。

## Approval eligibility

Autonomous approval eligible: Yes after the recorded Human decisions; no unresolved Human gate remains.

Review verdict: Pass。core AST、target identity、cardinality、nullable/value/integrity、Build Selection ordering、migration fail-closed、source-preserving Desktop authoring、domain/codegen naming separation、Optional non-unique absence contractはHuman decisionsと既存Approved authorityへ整合する。
