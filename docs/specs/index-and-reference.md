# IndexとReferenceのmodel（Index and Reference model）

Status: Draft

Table、Primary Key、Secondary Keyのcurrent Approved specificationは、[Table / Primary Key / Secondary Key仕様](table-and-keys.md)がcanonical ownerである。
このdocumentは、Approved Table/Key semanticsとReferenceのmaturityを混在させないため、Primary Key、Secondary Key、UniquenessのRequirement definitionを
所有しない。`INDEX-PRIMARY-001`、`INDEX-SECONDARY-001`、`INDEX-UNIQUE-001`は、既存IDを維持したままTable/Key仕様へ移動した。

Table/Key specificationは、explicitなMessagePack field `key`とgenerated `[Key(n)]`、Primary Key / Secondary Keyのfield-name sequence、
`nonUnique`、selection後のconstraint適用を扱う。ReferenceからPrimary/Secondary Keyへ向けるexact syntaxやtarget identityは、まだこのDraft
documentの未解決事項である。

### REF-001

MasterReferenceはsource fieldとtarget table/indexを指定しなければならない（MUST）。

### REF-002

unique targetは1件にresolveし、non-unique targetは複数件にresolveする。

### REF-003

Referenceはbuild中にvalidateしなければならず（MUST）、generated helperはmaster recordに
`MemoryDatabase`を保持せず、callerから受け取らなければならない（MUST）。

Referenceのexact YAML syntax、nullability、missing-reference severity、generated helperのnaming policyは未解決である。

Primary Key / Secondary KeyをReference targetとして指定する場合のfield-name sequence、logical identity、およびvalidation behaviorは、Table/Key仕様で
先取りせず、Referenceのrefinementで定義する。
