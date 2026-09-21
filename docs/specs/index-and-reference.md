# IndexとReferenceのmodel（Index and Reference model）

Status: Approved

Table、Primary Key、Secondary Keyのcurrent Approved specificationは、[Table / Primary Key / Secondary Key仕様](table-and-keys.md)がcanonical ownerである。
このdocumentは、Approved Table/Key semanticsとReferenceの責務を分離するため、Primary Key、Secondary Key、UniquenessのRequirement definitionを
所有しない。`INDEX-PRIMARY-001`、`INDEX-SECONDARY-001`、`INDEX-UNIQUE-001`は、既存IDを維持したままTable/Key仕様へ移動した。

Table/Key specificationは、explicitなMessagePack field `key`とgenerated `[Key(n)]`、Primary Key / Secondary Keyのfield-name sequence、
`nonUnique`、selection後のconstraint適用を扱う。ReferenceからPrimary/Secondary Keyへ向けるrelationship semanticsはこのdocumentが所有する。

### REF-001

**Schema-level declaration**

Table schemaの`references` sequenceは、explicit `name`、ordered source `fields`、および`target.table` + ordered `target.fields`を持つ
schema-level declarationでなければならない（MUST）。field-level annotation、MessagePack key、file path、generated type nameをReference
identityへ使用してはならない（MUST NOT）。

### REF-002

**Target identity and cardinality**

`target.fields`はtarget TableのPrimary Key ordered sequence、または1つのSecondary Key ordered sequenceと完全一致してresolveしなければ
ならない（MUST）。Primary Keyとunique Secondary Keyはsingle、`nonUnique: true` Secondary Keyはmultiへresolveする。Secondary Keyの
declaration reorderによるbackend `indexNo`の変化はReference identityを変えてはならない（MUST NOT）。

### REF-003

**Selected dataset integrity**

ReferenceはBuild Selection後、PK/unique Secondary constraint後、canonical record ordering前にselected logical datasetへ対してvalidate
しなければならない（MUST）。selected source recordからselected target datasetへmatching rowが0件なら、non-null Referenceの
build-blocking validation errorとする。source recordがprofileでexcludeされた場合はReference integrity対象外、targetだけがexcludeされた
場合はmissing targetとする。

generated helperはmaster recordへ`MemoryDatabase`を保持せず、callerから受け取らなければならない（MUST）。helper bodyはtarget Tableの
既存MasterMemory generated queryへlowerし、RustやC#側で全Table scan、独自index、MasterMemory binary semanticsを再実装してはならない
（MUST NOT）。

### REF-004

**Required / Nullable source semantics**

Reference v1はRequired ReferenceとOptional/Nullable Referenceを扱う。Required Referenceの全source componentはRequired scalarで、既存
Type Systemのkey capabilityとcomparison capabilityを満たさなければならない（MUST）。Optional Referenceの全source componentはNullable
scalarでなければならず、Required/Nullableをcomposite内で混在させてはならない（MUST）。Array componentはrejectしなければならない
（MUST）。

Optional Referenceの全component nullはabsentでありvalid、target lookupを行わずmissing diagnosticを生成してはならない（MUST NOT）。
全component non-nullは通常lookupし、targetが0件ならerrorとする。一部componentだけnullのpartial-nullはinvalidであり、lookupを推測しては
ならない（MUST NOT）。

### REF-005

**Type compatibility**

source componentとtarget key componentは同じApproved key/value semantic typeで比較可能でなければならない（MUST）。Nullable modifierは照合
前に除くが、primitive width、nominal Value Object、Enum等のimplicit conversionをReference解決へ挿入してはならない（MUST NOT）。

### REF-006

**Declaration diagnostics**

Table内のduplicate Reference name、empty/duplicate/unknown source component、source modifier/capability違反、unknown/empty/duplicate/
unknown target component、target key sequence不一致、component count/type mismatchはstructured validation diagnosticとしてrejectする。
diagnosticは可能な範囲でsource schema、data file、record identity、Reference name、field/value pathへprovenanceを持つ。

### REF-007

**Non-unique semantics**

non-unique targetはmatching rowが1件以上ならvalidであり、複数matchをsingle rowへ縮退してはならない（MUST NOT）。0件は空relationではなく
missing Reference errorとする。

Reference declarationのexact lexical name grammar、generated helper public method name、およびOptional non-unique helperのpublic absence
return contractは、Reference v1のcore semantic owner外に残るHuman gateである。これらをimplementation convenienceで発明してはならない。

Primary Key / Secondary Key自体のcapability、query-name導出、unique constraint、backend loweringはTable/KeyおよびType System ownerを
再利用し、Reference resolverで複製してはならない。
