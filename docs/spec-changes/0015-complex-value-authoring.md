# 仕様変更: Complex Value Authoring v1

Status: Proposed

## Affected Specifications

- `docs/specs/source-edit.md` — `Status: Approved`
  - `SOURCE-EDIT-003`, `SOURCE-EDIT-005`, `SOURCE-EDIT-006`, `SOURCE-EDIT-014`
  - new candidate IDs: `SOURCE-EDIT-015`, `SOURCE-EDIT-016`
- `docs/specs/source-record-mutation.md` — `Status: Approved`
  - `SOURCE-RECORD-002`, `SOURCE-RECORD-003`, `SOURCE-RECORD-004`, `SOURCE-RECORD-014`
  - new candidate ID: `SOURCE-RECORD-015`
- `docs/gui/data-editor/spec.md` — `Status: Approved`
  - `GUI-DATA-STATE-001`, `GUI-DATA-EDIT-002`, `GUI-DATA-VAL-003`, `GUI-DATA-VAL-004`
  - new candidate ID: `GUI-DATA-EDIT-003`
- `docs/gui/data-editor/record-mutation.md` — `Status: Approved`
  - `GUI-DATA-ROW-001`, `GUI-DATA-ROW-002`, `GUI-DATA-ROW-003`, `GUI-DATA-ROW-005`, `GUI-DATA-ROW-012`
  - new candidate ID: `GUI-DATA-ROW-013`

## 根拠と分類（Source Evidence and Classification）

- **Decision**: HumanはComplex Value Authoring v1のinitial strategyとしてRFC 0007 Option C、shared schema-driven value authoring modelをexisting editとAdd Rowで共用する方向を選択した。
- **Decision**: Humanはstructural complex editのsource preservationとしてP1 Fine-grained preservationを選択した。
- **Decision**: HumanはAdded record draftのunset value representationとしてD1 YAML `null` placeholderを選択した。
- **Requirement**: Approved Type Systemがfield valueとして許可するPrimitive、Value Object、Enum、Flags Enum、Custom TypeとRequired / Nullable / Array compositionを、raw YAML手編集へ戻らずData Editorからauthoringできる方向へ進める。
- **Constraint**: frontendはYAML parse、type lookup、Enum/Flags resolution、Custom Type shape reconstructionをdomain authorityとして再実装しない。
- **Constraint**: nested `long` / `ulong`を含むvalue transportはlosslessでなければならない。
- **Constraint**: file単位dirty / Save、validation non-blocking、exact source provenance、lost-update prevention、Conflict / Failure / Outcome Unknown、Build非連動を維持する。
- **Constraint**: existing recordのPrimary / Secondary Key構成field mutationは本changeへ含めない。
- **Constraint**: invalid existing sourceをeditor都合でcoerce、default補完、full reserializeしてはならない。

## 提案する差分（Proposed Delta）

### 1. Shared resolved value authoring boundary

`SOURCE-EDIT-015`を追加する候補とする。

> Data Editor向けのshared application boundaryは、fieldのresolved base type category、Required / Nullable / Array shape、Custom Typeのnested field shape、Enum / Flagsのdeclared member set、およびcurrent source valueを、frontendがYAMLまたはtype declarationを再解釈せずeditorを構成できる形で提供しなければならない（MUST）。exact wire/API shapeは固定しない。frontendはこの情報からYAML domain semanticsを再構築してはならない（MUST NOT）。

current source valueがdomain-invalidでも、shared boundaryがsource valueをlosslessに保持できる場合はそのvalueを黙ってvalid valueへcoerceしてはならない（MUST NOT）。安全にtyped authoring stateへ投影できないsource shapeでは、fieldを近似編集可能として扱わず、original sourceを保持したままread-only reasonとshared diagnosticを提示できなければならない（MUST）。

`SOURCE-EDIT-016`を追加する候補とする。

> Complex value edit requestはnested scalarを含むvalue treeをlosslessに表現しなければならない（MUST）。特に任意のnested positionにある`long` / `ulong`は全64-bit rangeをroundingなしで往復できなければならず（MUST）、frontendのIEEE-754 `number`へ強制変換してはならない（MUST NOT）。Value ObjectはApproved underlying scalar representation、Enumはsymbolic member、Flagsはsymbolic member sequence、Custom Typeはdeclared field mapping、Nullable / ArrayはApproved Type Systemのshape semanticsを使用し、それらのdomain meaningをこの仕様変更で再定義しない。

### 2. Existing record edit scope

`GUI-DATA-STATE-001`を次の意味へ変更する候補とする。

- base snapshotに存在するexisting recordでは、Primary / Secondary Key構成fieldを引き続きread-onlyとする（MUST）。
- non-key fieldは、shared applicationがv1 supported resolved value shapeとして安全にauthoring可能と報告する場合、Primitiveに限定せずValue Object / Enum / Flags / Custom Type / Nullable / Arrayを含めeditableとして扱わなければならない（MUST）。
- unsupported、unresolved、またはsource shapeをlosslessにauthoring stateへ投影できないfieldは、Table全体を隠さずfield単位でread-only reasonを示さなければならない（MUST）。

`GUI-DATA-EDIT-003`を追加する候補とする。

> Complex value editorはshared resolved value authoring stateから構成しなければならない（MUST）。Nullableのnull/non-null transition、Array element add/remove/order、Enum single-member selection、Flags member set、Custom Type nested field editをschema-aware controlとして扱い、general-purpose raw YAML / JSON fragment editorを通常編集経路として使用してはならない（MUST NOT）。exact inline/popover/drawer componentはimplementation detailとする。

`GUI-DATA-VAL-003` / `GUI-DATA-VAL-004`は、shared diagnosticがnested value pathへ対応づけ可能な場合、top-level cellだけでなく対応するnested editor controlへmarker / navigationを提供できるようapplicabilityを拡張する。cellへ一意に対応できないdiagnosticを失ってはならない既存ruleは維持する。

### 3. Added record draft scope

`SOURCE-RECORD-002`を、全fieldがRequired Primitiveの場合だけsupportedとする制限から、**全fieldがv1 supported resolved value shapeへ安全にresolveできる場合にAdd Recordをsupportedとする**ruleへ変更する候補とする。unknown / unsupported field shapeが1つでもある場合はAdd Recordをsupportedとして扱ってはならない（MUST NOT）。

`SOURCE-RECORD-004`はPrimitive-only text inputから、`SOURCE-EDIT-016`と同じlossless typed value representationをAdded record draftにも適用する意味へ変更する候補とする。domain validation errorだけを理由にdraft保持またはfile Saveを禁止してはならない既存ruleは維持する。

さらに、Added record draftのまだ入力されていないtyped valueはSave candidate上でYAML `null`として表現しなければならない（MUST）。field entry自体を省略してはならず（MUST NOT）、first Enum member、numeric zero、empty string、empty Array、Custom Type default等のdomain valueを暗黙defaultとして発明してはならない（MUST NOT）。Nullable fieldの`null`はApproved Type Systemに従ってvalidであり、Required / Arrayまたはnon-nullを要求するvalue positionの`null`はdomain-invalidとしてshared validation diagnosticへ渡さなければならない（MUST）。そのvalidation errorだけを理由にSave candidate生成またはfile Saveを禁止してはならない（MUST NOT）。

Custom Type等のcompound draftを利用者が具体的なmapping valueとしてmaterializeした場合、そのmappingはdeclared field entryを保持し、まだ入力されていないnested typed valueにも同じ`null` placeholder ruleを再帰的に適用しなければならない（MUST）。Array value自体が未入力なら`null` placeholderであり、利用者がArray valueをmaterializeした後のempty sequence `[]`はApproved Type Systemが定めるvalid empty Arrayとして扱う。

`SOURCE-RECORD-015`を追加する候補とする。

> Added record draftとexisting record editは、base snapshot presenceやkey editability等のlifecycle差を除き、同じresolved value shapeとvalue authoring semanticsを使用しなければならない（MUST）。record addition専用にfrontend-owned YAML rendering、Enum lookup、Custom Type reconstructionを持ってはならない（MUST NOT）。

`GUI-DATA-ROW-001` / `GUI-DATA-ROW-005`は、Required-Primitive-only gateを上記shared capability gateへ置き換える候補とする。`GUI-DATA-ROW-002`の「Added record draftではkey fieldも初回Save前にeditable」という既存例外は維持し、same typed value authoring modelをkey fieldにも使用する。

`GUI-DATA-ROW-003`のlossless inputとvalidation non-blocking ruleはcomplex draftにも適用する。初期未入力stateが`null` placeholderを持つことを理由にcell edit、draft保持、file Saveを禁止してはならない（MUST NOT）。

`GUI-DATA-ROW-013`を追加する候補とする。

> Added record draftのcomplex field editorはexisting recordと同じshared resolved value authoring modelから構成しなければならない（MUST）。Added record固有のdraft stateはsourceへまだ存在しないことと初回key入力を表現するためだけに用い、別のdomain type semanticsを導入してはならない（MUST NOT）。

### 4. Fine-grained source-preserving candidate boundary

`SOURCE-EDIT-005` / `SOURCE-EDIT-006`のfull-file reserialization禁止、unrelated source preservation、unsafe locationでのfail-closedをcomplex value editへ拡張する。

Array element add/remove、Nullable state transition、Custom Type nested edit、Flags member add/remove等のstructural complex editでも、Save candidateは変更対象のlogical value pathとそのsyntaxを成立させるために必要な最小source rangeだけをpatchしなければならない（MUST）。target value subtree内であっても、直接変更対象ではないsibling value、comment、quote/style、flow/block style、blank line、member/element source textをbroad subtree renderingによって置き換えてはならない（MUST NOT）。

valid YAMLを維持するためにseparator、indentation、collection marker等の周辺syntaxを挿入・除去する必要がある場合、その必要範囲だけをpatchへ含めてよい（MAY）。ただし変更対象nodeと必要syntaxのsource rangeをbase snapshot上で安全かつ一意にlocalizeできない場合、candidate derivationは`SOURCE-EDIT-006`に従ってfail closedしなければならず（MUST）、target subtree全体またはfull fileのcanonical renderingへfallbackして成功扱いしてはならない（MUST NOT）。

structural operationにより削除されるnode自身の内部source textはnodeとともに除去してよい（MAY）が、そのnodeの外側にあるstandalone comment等をownership推測だけで削除してはならない（MUST NOT）。同じbase snapshotとordered edit setから同じcandidate bytesを生成する既存determinism ruleは維持する。

### 5. Existing safety composition

`SOURCE-EDIT-007`から`SOURCE-EDIT-014`、`SOURCE-RECORD-012`から`SOURCE-RECORD-014`、Data Editorのdirty / Save / Diff / Conflict / Build semanticsは変更しない。Complex value authoringを理由にBuild / Publish / Git / schema Migration / Type Migrationを暗黙実行してはならない。

## 互換性（Compatibility）

YAML syntax、Table identity、MessagePack key、Type System、generated C#、MasterMemory binary formatは変更しない。変更はauthoring capabilityの拡張であり、保存後のvalue semanticsは既存Approved Type Systemに従う。

existing record key mutationは引き続き禁止するためPrimary / Secondary Key semanticsへの変更はない。

source text compatibilityはP1 Fine-grained preservationを採用し、complex structural editでも直接変更対象と必要syntax以外のsource bytesを保持する。安全にlocalizeできないsource shapeはbroad rewriteせずfail closedする。

Added record draftでは未入力typed valueをYAML `null`としてcandidateへ含める。Nullableではvalid、Required / Array等ではdomain-invalidとなるが、既存validation non-blocking contractによりvalidationだけを理由にSaveを禁止しない。このruleはdomain defaultを追加せず、Type Systemのvalidity semanticsを変更しない。

## 受け入れと実装への影響（Acceptance and Implementation Impact）

承認後は少なくとも次のevidenceを要求する。

- core/application: Primitive / Value Object / Enum / Flags / Custom TypeとRequired / Nullable / Arrayのresolved descriptor、nested lossless value round-trip、invalid-source fail-closed。
- source patch: nested leaf edit、Nullable transition、Array add/remove/reorder、Flags member add/remove、Custom Type nested editがfine-grained patchであり、変更対象外sibling/comment/styleおよび別field / record / file bytesを変更しない。safe localization不能時にsubtree/full serializationへfallbackせず失敗する。
- record mutation: complex TableでAdd Rowがsupportedとなり、key fieldを含む全fieldがshared value modelでauthoringできる。未入力fieldはcandidateで`null`となり、Required / Array等のinvalidityはshared diagnosticへ現れるがSave validation gateにはならない。
- nested draft: materialized Custom Type内の未入力nested valueにも`null` placeholderが適用され、Arrayの未入力`null`とmaterialized empty `[]`を区別する。
- validation: nested diagnosticがcurrent bufferへ対応し、validation errorだけでSaveを禁止しない。
- Tauri adapter: typed tree / exact integer representationをlosslessにtransportし、frontend側でYAML semantic reconstructionを行わない。
- React workflow: Enum/Flags/Nullable/Array/Custom Typeの編集、nested focus/diagnostic、Added record初回入力、unset `null` representation、Save後のexisting key read-only transition。
- regression: existing Primitive authoring、dirty lifecycle、Conflict / Overwrite、Delete / Undo、Diff、Build非連動を維持する。

実装対象は主に`masterdata-core` / `masterdata-app`のauthoring snapshot・candidate derivation、Tauri command DTO、Data Editor React state/control、focused Rust/React testsとなる見込みである。.NET adapter、MasterMemory format、generated C# semanticsは変更対象としない。

## 未解決事項（Open Questions）

None identified for this proposal. Exact DTO field name、React component placement、focus restorationの細部、spacing、deterministicな内部patch data structure等は、上記observable contractを変えない範囲でimplementation detailとする。

## レビュー（Review）

Independent `review-spec` pending after P1 / D1 incorporation.

## 承認記録（Approval Record）

未承認。Human maintainerによるproposal全体の明示Approvalが必要である。