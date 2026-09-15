# 仕様変更: Complex Value Authoring v1

Status: Applied

## Affected Specifications

- `docs/specs/source-edit.md` — `Status: Approved`
  - changed: `SOURCE-EDIT-003`, `SOURCE-EDIT-005`, `SOURCE-EDIT-006`, `SOURCE-EDIT-014`
  - added: `SOURCE-EDIT-015`, `SOURCE-EDIT-016`
- `docs/specs/source-record-mutation.md` — `Status: Approved`
  - changed: `SOURCE-RECORD-002`, `SOURCE-RECORD-003`, `SOURCE-RECORD-004`, `SOURCE-RECORD-014`
  - added: `SOURCE-RECORD-015`
- `docs/gui/data-editor/spec.md` — `Status: Approved`
  - changed: `GUI-DATA-STATE-001`, `GUI-DATA-EDIT-002`, `GUI-DATA-VAL-003`, `GUI-DATA-VAL-004`
  - added: `GUI-DATA-EDIT-003`
- `docs/gui/data-editor/record-mutation.md` — `Status: Approved`
  - changed: `GUI-DATA-ROW-001`, `GUI-DATA-ROW-002`, `GUI-DATA-ROW-003`, `GUI-DATA-ROW-005`, `GUI-DATA-ROW-012`
  - added: `GUI-DATA-ROW-013`

## 根拠と分類（Source Evidence and Classification）

- **Decision**: HumanはComplex Value Authoring v1のinitial strategyとしてRFC 0007 Option C、shared schema-driven value authoring modelをexisting editとAdd Rowで共用する方向を選択した。
- **Decision**: Humanはstructural complex editのsource preservationとしてP1 Fine-grained preservationを選択した。
- **Decision**: HumanはAdded record draftのunset value representationとしてD1 YAML `null` placeholderを選択した。
- **Decision**: 2026-09-16、Human maintainerはreview済みproposal全体を明示Approveした。
- **Requirement**: Approved Type Systemがfield valueとして許可するPrimitive、Value Object、Enum、Flags Enum、Custom TypeとRequired / Nullable / Array compositionを、raw YAML手編集へ戻らずData Editorからauthoringできる方向へ進める。
- **Constraint**: frontendはYAML parse、type lookup、Enum/Flags resolution、Custom Type shape reconstructionをdomain authorityとして再実装しない。
- **Constraint**: nested `long` / `ulong`を含むvalue transportはlosslessでなければならない。
- **Constraint**: file単位dirty / Save、validation non-blocking、exact source provenance、lost-update prevention、Conflict / Failure / Outcome Unknown、Build非連動を維持する。
- **Constraint**: existing recordのPrimary / Secondary Key構成field mutationは本changeへ含めない。
- **Constraint**: invalid existing sourceをeditor都合でcoerce、default補完、full reserializeしてはならない。

## 適用した差分（Applied Delta）

### 1. Shared resolved value authoring boundary

`SOURCE-EDIT-015`を追加した。

> Data Editor向けのshared application boundaryは、fieldのresolved base type category、Required / Nullable / Array shape、Custom Typeのnested field shape、Enum / Flagsのdeclared member set、およびcurrent source valueを、frontendがYAMLまたはtype declarationを再解釈せずeditorを構成できる形で提供しなければならない（MUST）。exact wire/API shapeは固定しない。frontendはこの情報からYAML domain semanticsを再構築してはならない（MUST NOT）。

current source valueがdomain-invalidでも、shared boundaryがsource valueをlosslessに保持できる場合はそのvalueを黙ってvalid valueへcoerceしてはならない（MUST NOT）。安全にtyped authoring stateへ投影できないsource shapeでは、fieldを近似編集可能として扱わず、original sourceを保持したままread-only reasonとshared diagnosticを提示できなければならない（MUST）。

`SOURCE-EDIT-016`を追加した。

> Complex value edit requestはnested scalarを含むvalue treeをlosslessに表現しなければならない（MUST）。特に任意のnested positionにある`long` / `ulong`は全64-bit rangeをroundingなしで往復できなければならず（MUST）、frontendのIEEE-754 `number`へ強制変換してはならない（MUST NOT）。Value ObjectはApproved underlying scalar representation、Enumはsymbolic member、Flagsはsymbolic member sequence、Custom Typeはdeclared field mapping、Nullable / ArrayはApproved Type Systemのshape semanticsを使用する。

### 2. Existing record edit scope

`GUI-DATA-STATE-001`を、existing recordのPrimary / Secondary Key構成fieldはread-onlyのまま、shared applicationが安全にauthoring可能と報告するnon-key Primitive / Value Object / Enum / Flags / Custom Type / Nullable / Arrayをeditableとする意味へ変更した。unsupported、unresolved、またはlosslessにtyped authoring stateへ投影できないfieldはfield単位でread-only reasonを示す。

`GUI-DATA-EDIT-003`を追加し、Nullable、Array、Enum、Flags、Custom Typeのschema-aware controlをshared resolved value stateから構成し、raw YAML / JSON fragment editorを通常編集経路にしないcontractを追加した。

`GUI-DATA-VAL-003` / `GUI-DATA-VAL-004`はnested value pathへmapping可能なdiagnosticについてnested editor controlへのmarker / navigationを含むよう拡張した。

### 3. Added record draft scope

`SOURCE-RECORD-002`のRequired-Primitive-only gateを、全fieldがv1 supported resolved value shapeへ安全にresolveできることを条件とするcapability gateへ変更した。

`SOURCE-RECORD-004`は`SOURCE-EDIT-016`と同じlossless typed value representationをAdded record draftにも適用する。Added record draftのまだ入力されていないtyped valueはSave candidate上でYAML `null`として表現し、field entry自体を省略せず、first Enum member、numeric zero、empty string、empty Array、Custom Type default等を暗黙defaultとして発明しない。Nullableの`null`はvalid、Required / Arrayまたはnon-nullを要求するpositionの`null`はdomain-invalid diagnosticとし、そのvalidation errorだけを理由にSave candidate生成またはfile Saveを禁止しない。

Custom Type等のcompound draftを具体的なmapping valueとしてmaterializeした場合、declared field entryを保持し、未入力nested typed valueにも同じ`null` placeholder ruleを再帰適用する。Array value自体が未入力なら`null` placeholder、materialize後のempty sequence `[]`はvalid empty Arrayとして区別する。

`SOURCE-RECORD-015`を追加し、Added record draftとexisting record editがlifecycle差を除いて同じresolved value shape / authoring semanticsを共有することを定めた。

`GUI-DATA-ROW-001` / `GUI-DATA-ROW-005`はshared capability gateへ変更し、`GUI-DATA-ROW-002`の初回Save前key editabilityは維持した。`GUI-DATA-ROW-003`はcomplex draftのlossless typed inputとD1 `null` placeholderへ拡張した。`GUI-DATA-ROW-013`を追加し、existing editと同じshared complex editor semanticsをAdded recordにも適用した。

### 4. Fine-grained source-preserving candidate boundary

`SOURCE-EDIT-005` / `SOURCE-EDIT-006`のfull-file reserialization禁止、unrelated source preservation、unsafe locationでのfail-closedをcomplex value editへ拡張した。

Array element add/remove、Nullable state transition、Custom Type nested edit、Flags member add/remove等のstructural complex editでも、Save candidateは変更対象のlogical value pathとsyntax成立に必要な最小source rangeだけをpatchする。target value subtree内でも直接変更対象ではないsibling value、comment、quote/style、flow/block style、blank line、member/element source textをbroad subtree renderingで置換しない。

valid YAML維持に必要なseparator、indentation、collection marker等の周辺syntaxは必要範囲だけpatchへ含められるが、変更対象nodeと必要syntax rangeをbase snapshot上で安全かつ一意にlocalizeできない場合はfail closedし、target subtree全体またはfull fileのcanonical renderingへfallbackしない。

### 5. Existing safety composition

`SOURCE-EDIT-007`以降のfile commit safety、Source Record Mutationのlost-update / recovery、Data Editorのdirty / Save / Diff / Conflict / Build semanticsは維持した。Complex value authoringを理由にBuild / Publish / Git / schema Migration / Type Migrationを暗黙実行しない。

## 互換性（Compatibility）

YAML syntax、Table identity、MessagePack key、Type System、generated C#、MasterMemory binary formatは変更しない。変更はauthoring capabilityの拡張であり、保存後のvalue semanticsは既存Approved Type Systemに従う。

existing record key mutationは引き続き禁止するためPrimary / Secondary Key semanticsへの変更はない。

source text compatibilityはP1 Fine-grained preservationを採用し、complex structural editでも直接変更対象と必要syntax以外のsource bytesを保持する。安全にlocalizeできないsource shapeはbroad rewriteせずfail closedする。

Added record draftでは未入力typed valueをYAML `null`としてcandidateへ含める。Nullableではvalid、Required / Array等ではdomain-invalidとなるが、既存validation non-blocking contractによりvalidationだけを理由にSaveを禁止しない。このruleはdomain defaultを追加せず、Type Systemのvalidity semanticsを変更しない。

## 受け入れと実装への影響（Acceptance and Implementation Impact）

implementationでは少なくとも次のevidenceを要求する。

- core/application: Primitive / Value Object / Enum / Flags / Custom TypeとRequired / Nullable / Arrayのresolved descriptor、nested lossless value round-trip、invalid-source fail-closed。
- source patch: nested leaf edit、Nullable transition、Array add/remove/reorder、Flags member add/remove、Custom Type nested editがfine-grained patchであり、変更対象外sibling/comment/styleおよび別field / record / file bytesを変更しない。safe localization不能時にsubtree/full serializationへfallbackせず失敗する。
- record mutation: complex TableでAdd Rowがsupportedとなり、key fieldを含む全fieldがshared value modelでauthoringできる。未入力fieldはcandidateで`null`となり、Required / Array等のinvalidityはshared diagnosticへ現れるがSave validation gateにはならない。
- nested draft: materialized Custom Type内の未入力nested valueにも`null` placeholderが適用され、Arrayの未入力`null`とmaterialized empty `[]`を区別する。
- validation: nested diagnosticがcurrent bufferへ対応し、validation errorだけでSaveを禁止しない。
- Tauri adapter: typed tree / exact integer representationをlosslessにtransportし、frontend側でYAML semantic reconstructionを行わない。
- React workflow: Enum/Flags/Nullable/Array/Custom Typeの編集、nested focus/diagnostic、Added record初回入力、unset `null` representation、Save後のexisting key read-only transition。
- regression: existing Primitive authoring、dirty lifecycle、Conflict / Overwrite、Delete / Undo、Diff、Build非連動を維持する。

実装対象は主に`masterdata-core` / `masterdata-app`のauthoring snapshot・candidate derivation、Tauri command DTO、Data Editor React state/control、focused Rust/React testsとなる。.NET adapter、MasterMemory format、generated C# semanticsは変更対象としない。

## 未解決事項（Open Questions）

None identified. Exact DTO field name、React component placement、focus restorationの細部、spacing、deterministicな内部patch data structure等は、上記observable contractを変えない範囲でimplementation detailとする。

## レビュー（Review）

### Blocking Issues

None identified.

P1は`SOURCE-EDIT-005`のunrelated source preservationと`SOURCE-EDIT-006`のfail-closedをcomplex subtreeへ強める方向であり、broad serializer fallbackを許可しない既存contractと矛盾しない。D1はfield entryを省略せず`null`をcandidateへ置き、Required / Array等では`TYPE-FIELD-003`に従ってinvalid diagnosticとするため、Type System semanticsを変更しない。

### Non-blocking Issues

None identified.

### Questions

None identified.

### Approved as Proposed

**Yes.** Humanが選択したOption C + P1 + D1を、Approved Type System、Source Edit、Source Record Mutation、Data Editorの既存安全境界を弱めずtest可能なdeltaへ落とせている。

| Review axis | Verdict |
| --- | --- |
| Intent fidelity | Option C、P1、D1のHuman decisionをその強度のまま反映。 |
| Internal consistency | shared typed model、P1 source patch、D1 null placeholder、validation non-blockingが整合。 |
| Cross-spec consistency | `SOURCE-EDIT-005/006`、`SOURCE-RECORD-003/004/011`、Data Editor dirty/Save、Type System null/Array semanticsを維持。 |
| Terminology consistency | Existing `Base snapshot`、`Save candidate`、`Added record draft`、Type System用語を使用。 |
| Normative strength | Human decisionとApproved constraintに対応するMUST/MUST NOTだけを追加。 |
| Testability | nested patch preservation、fail-closed、null placeholder、complex Add Rowをobservable testへ分解可能。 |
| Backward compatibility | YAML/domain/binary semanticsは変更せずauthoring capabilityのみ拡張。source textはP1で既存 preservationを強化。 |
| Unresolved ambiguity | ApprovalをblockするOpen Questionなし。exact UI/DTO/internal patch structureは非semantic detailとして残す。 |
| Implementation leakage | crate/API/componentのexact structureを固定していない。 |
| Unrequested behavior | key mutation、schema/type mutation、raw YAML editor、bulk edit、Build/Publish/Git連動を追加していない。 |

Current implementation / testsはRequired-Primitive-only existing edit/Add Rowというprevious Approved sliceを実装しており、このchangeのcomplex capabilityを先行実装しているevidenceは確認していない。したがって未承認semanticsのretroactive approval問題は見つからない。

Implementation rationale review: implementation diffは本review scopeに存在しないためNot applicable。

## 承認記録（Approval Record）

2026-09-16、Human maintainerがproposal全体（Option C + P1 Fine-grained preservation + D1 YAML `null` placeholder）を明示Approveした。同じcanonical mergeで上記4仕様へdeltaを適用し、本artifactを`Applied`へ進めた。implementation statusは本artifactではなくcanonical specificationとDevelopment Stateが所有する。