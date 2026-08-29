# Indexとreferenceのmodel（Index and reference model）

Status: Draft

Tableは、MasterMemoryが実用的にサポートできる範囲で、primary key、composite primary key、
複数のsecondary index、unique/non-unique secondary index、composite secondary indexをサポート
する予定である。Field identityとindex numberは別のconceptである。

予定しているRequirement ID:

### INDEX-PRIMARY-001

Tableは、composite keyを含む1つのprimary keyを宣言してもよい（MAY）。

### INDEX-SECONDARY-001

Tableは複数のsecondary indexを宣言してもよい（MAY）。

### INDEX-UNIQUE-001

Uniquenessは明示的なpropertyであり、field nameからの推論ではない。

### REF-001

MasterReferenceはsource fieldとtarget table/indexを指定しなければならない（MUST）。

### REF-002

unique targetは1件にresolveし、non-unique targetは複数件にresolveする。

### REF-003

Referenceはbuild中にvalidateしなければならず（MUST）、generated helperはmaster recordに
`MemoryDatabase`を保持せず、callerから受け取らなければならない（MUST）。

Open Questions: exact YAML syntax、nullability、missing-reference severity、generated helperの
naming policy。
