# Source Record Mutation仕様

Status: Proposed

Domain: Source Editing

## 概要

本仕様は、既存Data documentにrecord occurrenceを追加・削除し、YAMLをSource of Truthのままfile単位で安全に保存するためのobservable contractを定義する。

既存record member valueの変更は[Source Record Edit](source-edit.md)、Table / record validityは[Table / Primary Key / Secondary Key](table-and-keys.md)、Primitive value domainは[Primitive Types](type-system/primitives.md)、host I/O boundaryは[Runtime hosts](runtime-hosts.md)が所有する。本仕様はそれらを再定義せず、record sequenceのstructural mutation、source-preserving insertion/removal、および既存value editとのcompositionを所有する。

Source Record Editの`SOURCE-EDIT-007`から`SOURCE-EDIT-014`が定義するfile単位commit、lost-update preflight、Conflict / Failure / Outcome Unknown、explicit Overwrite、Build非連動、およびpatch derivationとhost I/Oの分離は、本仕様のSaveにも適用する。

## 用語

- **Existing record occurrence**: base snapshotの選択Data document内に存在する1つのphysical record sequence item。Primary Key valueとは別のsource provenanceで識別する。
- **Added record draft**: base snapshotには存在せず、local buffer内だけに存在する未保存record。
- **Pending delete**: existing record occurrenceをSave candidateから除外するが、まだworkspace sourceを変更していないlocal buffer state。
- **Record mutation set**: 1つのsource data fileに対するexisting value edits、added record drafts、pending deletesを合わせたlocal mutation state。

## 規範要件

### SOURCE-RECORD-001

record追加・削除はexactなbase snapshotとselected source data fileに対して実行しなければならない（MUST）。existing recordの削除対象はbase snapshot内のrecord occurrenceとして識別しなければならず（MUST）、Primary Key valueだけ、logical Table内のrecord identityだけ、または他Data fileを含むmerged dataset上の位置だけで対象を選んではならない（MUST NOT）。

同一Primary Key valueを持つrecordが同一または別Data documentに共存しても、選択したsource occurrence以外を削除してはならない（MUST NOT）。

### SOURCE-RECORD-002

初期Add Record operationは、選択Data documentがexactly 1つのTable schemaへresolveし、そのschemaの全fieldがRequired Primitiveである場合だけsupportedでなければならない（MUST）。Nullable、Array、Enum、Flags Enum、Value Object、Custom Type、unknown typeを1つでも含むTableについて、初期Add Record operationをsupportedとして扱ってはならない（MUST NOT）。

この制限はexisting recordのDelete capabilityや、既存[Source Record Edit](source-edit.md)が許可するfield編集 capabilityを自動的に無効化してはならない（MUST NOT）。

### SOURCE-RECORD-003

Added record draftはschemaで宣言された全fieldをexactly 1回ずつ持たなければならない（MUST）。unknown field、missing field、duplicate fieldをrecord addition requestの構造として生成してはならない（MUST NOT）。candidate source上のrecord mapping member orderはschema declaration orderを使用しなければならない（MUST）。

Primary Key / Secondary Key構成fieldをaddition requestから省略してはならない（MUST NOT）。既存recordではread-onlyなkey fieldであっても、Added record draftでは初回Save前のrecord定義値として入力可能でなければならない（MUST）。

### SOURCE-RECORD-004

Added record draftの各Primitive inputはlosslessなtext representationとしてapplication boundaryを通らなければならない（MUST）。特に`long` / `ulong`は全64-bit rangeをroundingなしで保持しなければならず（MUST）、frontend等でIEEE-754 `number`へ強制変換してはならない（MUST NOT）。

Stringは入力textをString valueとして扱う。他Primitiveでは、入力が安全なYAML scalarとして解釈可能ならそのscalarを使用してよく（MAY）、解釈不能またはdomain-invalidな入力はYAML-validなString scalarとして保持してshared validationへ渡してよい（MAY）。record additionをdomain validation errorだけで拒否してはならない（MUST NOT）。

### SOURCE-RECORD-005

Add Recordはselected source data fileの`records` sequence末尾へAdded record draftを追加するSave candidateを生成しなければならない（MUST）。source record orderをdomain / binary semanticsへ昇格させてはならない（MUST NOT）が、source-preserving authoring上のdeterministic presentationとしてappend位置を固定する。

GUI Source Creationが生成するsemantic empty records representation（少なくとも`records: []`）から、最初のblock-sequence recordを安全に生成できなければならない（MUST）。

### SOURCE-RECORD-006

Delete Recordはselected existing record occurrenceだけをSave candidateから除外しなければならない（MUST）。Delete開始時点でworkspace sourceへ即時mutationを行ってはならず（MUST NOT）、file Saveまでlocal Pending deleteとして保持できなければならない（MUST）。

Added record draftをSave前にDeleteした場合は、そのadditionをcancelしたものとして扱わなければならず（MUST）、base snapshotに存在しないrecordのdelete patchを生成してはならない（MUST NOT）。

### SOURCE-RECORD-007

同一fileのexisting value edits、Added record drafts、Pending deletesは1つのSave candidateへcompositionできなければならない（MUST）。Pending delete対象のexisting recordにvalue editが存在する場合、final candidateではrecord deletionが優先しなければならない（MUST）。Pending deleteをUndoした場合は、Delete前に存在したlocal value edit stateを復元できる情報を失ってはならない（MUST NOT）。

Added record draftの入力変更はそのdraft mappingへ反映し、初回Save後は通常のexisting record snapshotとして扱わなければならない（MUST）。

### SOURCE-RECORD-008

Record mutation candidateはsource-preservingかつdeterministicでなければならない（MUST）。同じexact base snapshotと同じordered Record mutation setからは同じcandidate source bytesを生成しなければならない（MUST）。

対象構造変更と無関係なcomments、blank lines、line ending / newline style、quote style、indentation、mapping member order、非対象record order、およびその他のsource textを変更または削除してはならない（MUST NOT）。semantic AST全体を通常serializerで全面再出力する方式を通常mutation pathとして使用してはならない（MUST NOT）。

### SOURCE-RECORD-009

Delete patchはtarget record sequence itemのsource rangeだけを除去しなければならない（MUST）。target mappingの内部に属するsource textはrecordとともに除去してよい（MAY）が、record itemの外側にあるstandalone commentまたはseparator blank lineを「record ownership」と推測して暗黙に削除してはならない（MUST NOT）。

Add patchは既存record、comment、blank lineのbytesを再serializeしてはならず（MUST NOT）、既存`records` regionへ必要なnew record bytesだけを挿入しなければならない（MUST）。

### SOURCE-RECORD-010

base snapshotから`records` region、record sequence boundary、またはdelete対象occurrenceを安全かつ一意に再特定できない場合、candidate生成を失敗させなければならない（MUST）。full-file reserialization、Primary Key search、近似text search、別recordへのfallbackで成功扱いしてはならない（MUST NOT）。このfailureではworkspace sourceを変更してはならない（MUST NOT）。

初期implementationが安全に扱えないYAML sequence styleを無理にrewriteしてはならず（MUST NOT）、unsupported source shapeとして明示的に失敗してよい（MAY）。

### SOURCE-RECORD-011

Record mutation後のvalidationは現在のSave candidateをshared Table / Type semanticsで評価しなければならない（MUST）。Added recordのinvalid value、Primary Key / Secondary Key constraint violation、またはDeleteによって変化したproject validationをdiagnosticとして返してよい（MAY）が、validation errorだけを理由にfile Saveを禁止してはならない（MUST NOT）。

Save successはproject validation successを意味しない。

### SOURCE-RECORD-012

Record mutation Saveは[Source Record Edit](source-edit.md)の`SOURCE-EDIT-007`から`SOURCE-EDIT-014`に従わなければならない（MUST）。したがってcommit unitは1 source data fileであり、mutation開始直前のexact-content preflight、explicit Overwrite recheck、`Success / Conflict / Failure / Outcome Unknown`の区別、Outcome Unknown後のactual source再取得、およびpartial/truncated contentをsuccess扱いしないことが必要である。

Record addition / deletionを理由に別dirty fileを暗黙Saveしてはならない（MUST NOT）。

### SOURCE-RECORD-013

Record mutationはBuild、Publish、Git stage / commit / push、schema Migration、generated C#更新、binary更新を暗黙に開始してはならない（MUST NOT）。Buildは保存済みworkspace sourceを入力とする別operationのままでなければならない（MUST）。

### SOURCE-RECORD-014

record sequence location resolution、candidate derivation、Primitive input conversion、existing value editとのcomposition、validationとのcomposition等のshared semanticsをTauri frontend、Browser Host、Native Host adapterごとに再実装してはならない（MUST NOT）。source patch derivationはhost commit I/Oから分離できなければならない（MUST）。

Native filesystem write等のhost-specific mutationは[Runtime hosts](runtime-hosts.md)のcapability boundaryへ委譲する。

## 検証ルール

少なくとも次をfocused unit / integration / GUI workflow evidenceで検証する。

- `records: []`のData documentへ最初のrecordを追加でき、schema declaration orderでfieldがrenderされる。
- 既存block sequenceの末尾へAddしても既存record、comments、blank lines、line endings、quote / indentationが保持される。
- `long` / `ulong` boundary valueがaddition boundaryでroundingされない。
- domain-invalidなnew record valueでもvalidationだけを理由にSave拒否されない。
- 同一PK valueのrecordが複数存在してもselected occurrenceだけをDeleteする。
- Deleteがtarget record外のstandalone comment / blank lineを削除しない。
- edited existing recordをDeleteし、Undoするとlocal editが復元される。
- Added record draftをDeleteするとadditionがcancelされ、他mutationがなければbyte-identical candidateへ戻る。
- Add / Delete / existing value editが同一fileの1 candidateへcompositionされる。
- unsafe / unsupported source shapeではfull serializationへfallbackせずmutation前に失敗する。
- base snapshot後のexternal editを通常SaveがConflictとして拒否し、explicit Overwriteでもexternal identityを再確認する。
- Success / Conflict / Failure / Outcome Unknownが混同されず、Outcome Unknown後にblind retryしない。
- SaveだけでBuild / Publish / Git operationを開始しない。

fixtureを使用する場合、既存fixture sourceを通常GUI/CLI executionで直接書き換えずtemporary workspace copyを使用する。

## 互換性

既存YAML syntax、Table identity、field semantics、MessagePack key、generated C#、binary formatを変更しない。record addition / deletionはexisting Data documentの`records` sequenceへ新しいauthoring operationを追加するだけであり、source record orderを新しいdomain identityへ昇格させない。

既存[Source Record Edit](source-edit.md)のRequirement IDとmeaningは変更しない。本仕様は`SOURCE-EDIT-002`が禁止する「既存member edit operationへの暗黙なadd/delete混入」を維持したまま、別operationとしてstructural mutationを導入する。

wire/API serialized shape、internal patch structure、exact diagnostic codeは固定しない。

## Open Questions

None identified for the initial Required-Primitive-only Add / source-occurrence Delete slice.

## 非目標

- Nullable / Array / Enum / Flags / Value Object / Custom Typeを含むTableへのrecord addition。
- `$tags`の追加・編集。
- record duplicate、move / reorder、bulk add / bulk delete。
- schema / field / key / type mutation。
- source file rename / move / folder operation。
- multi-file atomic transaction。
- general-purpose YAML formatterまたはfull document serializerによるSave。
- Build / Publish / Git operation。
- spreadsheet range edit、一括paste、fill handle、general Undo/Redo。
- Programmable View / Computed / Annotation columnの保存format。
