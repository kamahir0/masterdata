# 仕様変更: Reference v1

Status: Draft

## Affected Specifications

- [Index / Reference](../specs/index-and-reference.md) `REF-001..003`: DraftをReference v1のcanonical ownerへrefineする。
- [Table / Primary Key / Secondary Key](../specs/table-and-keys.md): existing Primary/Secondary identity、Build Selection後のReference integrity順序を変更せず参照する。
- [Schema language](../specs/schema-language.md): approved Reference surfaceへのrouting/exampleを同期する。
- [Build Selection](../specs/build-selection.md): selected logical dataset上のReference validation順序を既存contractどおり使用する。
- [Table Editor](../gui/table-editor/spec.md): Reference declarationのguided authoringを追加する。
- C# generation: resolved ReferenceをMasterMemory query APIへlowerする。exact public helper contractは本changeで決定する。

## 根拠と分類（Source Evidence and Classification）

- Human Decision: 2026-09-21 JST、Web退役後の次product priorityとしてReference方向へ進む。
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

- `name`はTable内でuniqueなReference declaration nameとし、generated helper namingのsourceになる。MessagePack identity、field identity、released compatibility identityにはしない。
- source `fields`はordered field-symbol sequence。target `fields`とcardinalityが一致しなければならない。
- targetはproject-local `table`と、そのTableのPrimary KeyまたはSecondary Keyと完全一致するordered `fields`で指定する。
- target `indexNo`、MessagePack `key`、generated C# name、file pathをtarget identityとして使用しない。
- source componentとtarget key componentは同じapproved key value semanticsで比較可能でなければならず、lossy conversionを挟まない。
- target Primary Keyまたはunique Secondary Keyはsingle Reference、`nonUnique: true` Secondary Keyはmulti Referenceへresolveする。
- selected source recordごとに、selected target logical datasetにmatching targetが存在しなければReference integrity errorとする。multi Referenceも0件をmissingとしてerrorにする。
- Reference validationはBuild Selection後、canonical ordering/binary build前に行う。
- Referenceはgenerated artifactやbinaryをauthorityにせずcanonical YAML + resolved Table modelをauthorityとする。

### Option A — 推奨: Required-only Reference v1

- source `fields`はすべてRequired scalarかつkey-compatibleでなければならない。
- Nullable / Array source fieldはReference componentに使用しない。
- null-reference semanticsをv1へ導入しない。
- generated helperはsource row partial classへ `Get<Name>(MemoryDatabase database)` を生成する。
- unique targetはtarget rowを返し、non-unique targetはMasterMemory `RangeView<Target>`を返す。
- helperはDBをfield/propertyへ保持せず、毎回callerから受け取る。
- helper bodyはtarget Tableの既存generated `FindBy...` APIを呼ぶだけとし、独自index lookupを生成しない。
- helper名またはsignature collisionはvalidation/codegen errorとし、自動suffixで修復しない。

このOptionはnullableのpartial-null policyをv1 scopeから明示的に外し、composite/unique/non-uniqueとhelper APIを一つのcoherent sliceで完成させる。

### Option B — Nullable Referenceをv1へ含める

Option Aに加え、source componentsを全てNullableにしたReferenceを許可する。

- 全component null: Referenceなし。
- 全component non-null: 通常lookup。
- compositeで一部だけnull: validation error。
- unique optional helperはnullable targetを返す。
- non-unique optional helperのabsence representationをexactly定義する必要がある。

利便性は高いが、validation、projection、GUI、C# signature、optional multi-reference semanticsがv1で増える。

### Option C — Integrity-only first

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

Human decision required:

1. Option A（Required-only + helperまで完成）、Option B（Nullableもv1）、Option C（integrity-only）のどれをReference v1とするか。
2. Option A/Bの場合、generated helper namingを推奨の`Get<Name>`で確定してよいか。別のpublic namingを採る場合はこの時点で決める。

target identityにbackend `indexNo`やSecondary Key nameを導入する案は、既存Approved identity contractと矛盾するためchoiceとして扱わない。

## レビュー（Review）

Pending Human decision. target identity、selection order、MasterMemory delegationは既存authorityから一意。persisted Reference declarationとgenerated public API / nullable scopeはmaterial product choiceであり、Human gateに該当する。

## 承認記録（Approval Record）

Pending.
