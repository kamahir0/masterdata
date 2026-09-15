# 仕様変更: Complex Value Authoring v1

Status: Draft

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
- **Requirement**: Approved Type Systemがfield valueとして許可するPrimitive、Value Object、Enum、Flags Enum、Custom TypeとRequired / Nullable / Array compositionを、raw YAML手編集へ戻らずData Editorからauthoringできる方向へ進める。
- **Constraint**: frontendはYAML parse、type lookup、Enum/Flags resolution、Custom Type shape reconstructionをdomain authorityとして再実装しない。
- **Constraint**: nested `long` / `ulong`を含むvalue transportはlosslessでなければならない。
- **Constraint**: file単位dirty / Save、validation non-blocking、exact source provenance、lost-update prevention、Conflict / Failure / Outcome Unknown、Build非連動を維持する。
- **Constraint**: existing recordのPrimary / Secondary Key構成field mutationは本changeへ含めない。
- **Constraint**: invalid existing sourceをeditor都合でcoerce、default補完、full reserializeしてはならない。
- **Open Question**: structural complex edit時にtarget value subtree内部のunchanged source textをどこまでbyte-preserveするか。
- **Open Question**: Added record draftでまだ入力されていないtyped valueをSave candidateへどう表現するか。

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

`SOURCE-RECORD-015`を追加する候補とする。

> Added record draftとexisting record editは、base snapshot presenceやkey editability等のlifecycle差を除き、同じresolved value shapeとvalue authoring semanticsを使用しなければならない（MUST）。record addition専用にfrontend-owned YAML rendering、Enum lookup、Custom Type reconstructionを持ってはならない（MUST NOT）。

`GUI-DATA-ROW-001` / `GUI-DATA-ROW-005`は、Required-Primitive-only gateを上記shared capability gateへ置き換える候補とする。`GUI-DATA-ROW-002`の「Added record draftではkey fieldも初回Save前にeditable」という既存例外は維持し、same typed value authoring modelをkey fieldにも使用する。

`GUI-DATA-ROW-013`を追加する候補とする。

> Added record draftのcomplex field editorはexisting recordと同じshared resolved value authoring modelから構成しなければならない（MUST）。Added record固有のdraft stateはsourceへまだ存在しないことと初回key入力を表現するためだけに用い、別のdomain type semanticsを導入してはならない（MUST NOT）。

### 4. Source-preserving candidate boundary

`SOURCE-EDIT-005` / `SOURCE-EDIT-006`のfull-file reserialization禁止、unrelated source preservation、unsafe locationでのfail-closedはcomplex value editにも維持する。

ただし、Array element add/remove、Nullable state transition、Custom Type nested structure editなどの**target value subtree内部**でどこまでunchanged bytesを必須保持するかはHuman decisionが必要であり、このDraftでは未確定とする。候補はOpen Questionsに記載する。

### 5. Existing safety composition

`SOURCE-EDIT-007`から`SOURCE-EDIT-014`、`SOURCE-RECORD-012`から`SOURCE-RECORD-014`、Data Editorのdirty / Save / Diff / Conflict / Build semanticsは変更しない。Complex value authoringを理由にBuild / Publish / Git / schema Migration / Type Migrationを暗黙実行してはならない。

## 互換性（Compatibility）

YAML syntax、Table identity、MessagePack key、Type System、generated C#、MasterMemory binary formatは変更しない。変更はauthoring capabilityの拡張であり、保存後のvalue semanticsは既存Approved Type Systemに従う。

existing record key mutationは引き続き禁止するためPrimary / Secondary Key semanticsへの変更はない。source text compatibilityについてはtarget subtree内部のpreservation policyが未決定であり、Human decisionまでcompatibility contractは完成しない。

Added record draftのunset value representationは、ユーザーが未入力のままSaveした場合のsource bytesとdiagnostic outcomeへ影響するため、Human decisionまで固定しない。

## 受け入れと実装への影響（Acceptance and Implementation Impact）

承認後は少なくとも次のevidenceを要求する。

- core/application: Primitive / Value Object / Enum / Flags / Custom TypeとRequired / Nullable / Arrayのresolved descriptor、nested lossless value round-trip、invalid-source fail-closed。
- source patch: nested leaf edit、Nullable transition、Array add/remove/reorder、Custom Type nested editで決定済みpreservation contractを満たし、別field / record / file bytesを変更しない。
- record mutation: complex TableでAdd Rowがsupportedとなり、key fieldを含む全fieldがshared value modelでauthoringできる。
- validation: nested diagnosticがcurrent bufferへ対応し、validation errorだけでSaveを禁止しない。
- Tauri adapter: typed tree / exact integer representationをlosslessにtransportし、frontend側でYAML semantic reconstructionを行わない。
- React workflow: Enum/Flags/Nullable/Array/Custom Typeの編集、nested focus/diagnostic、Added record初回入力、Save後のexisting key read-only transition。
- regression: existing Primitive authoring、dirty lifecycle、Conflict / Overwrite、Delete / Undo、Diff、Build非連動を維持する。

実装対象は主に`masterdata-core` / `masterdata-app`のauthoring snapshot・candidate derivation、Tauri command DTO、Data Editor React state/control、focused Rust/React testsとなる見込みである。.NET adapter、MasterMemory format、generated C# semanticsは変更対象としない。

## 未解決事項（Open Questions）

### Q1: structural editのsource preservation

- **P1 Fine-grained preservation（推薦）**: target subtree内でもunchanged source bytesを可能な限り保持し、Array item add/remove、mapping member value edit等は必要なsource rangeだけをpatchする。安全にlocalizeできない場合はbroad subtree rewriteへfallbackせずfail closedする。Git diffと既存source-preserving方針に最も強く整合するが、implementationとsource-range regressionが大きい。
- **P2 Target-subtree replacement**: structural operationではedited value subtree全体をcanonical renderingへ置換してよい。ただしsubtree外のsource bytesは保持する。implementationは単純になるが、subtree内部のcomment / quote / flow-vs-block style等が変更され得てdiff churnが増える。

### Q2: Added record draftのunset value representation

- **D1 YAML `null` placeholder（推薦）**: 未入力typed valueはSave candidate上で`null`として表現する。Nullableではvalid null、Required / Array / non-null complex valueではdomain-invalidだがYAML-validであり、shared validationがdiagnosticを返す。validation non-blockingと両立し、first Enum memberやnumeric zero等のdomain defaultを発明しない。
- **D2 Local-only `Unset`**: 未入力stateはsourceへ表現せず、すべてのUnsetが具体的なYAML nodeへ変換されるまでSave candidate生成をsource-safety理由でblockする。nullとの区別は明確だが、既存Added recordの「domain-invalidでもSave可能」というauthoring体験を狭める。

exact editor component placement、focus restorationの細部、spacing等は既存RFC 0005のGUI refinement delegationに従い、data safety / compatibilityを変えない範囲でimplementation detailとする。

## レビュー（Review）

### Blocking Issues

1. **Q1未決定**: P1/P2でsource diffとcomment/style preservation contractが変わるため、Human choiceなしにcanonical wordingを確定できない。
2. **Q2未決定**: D1/D2で未入力draftのSave可否とsource bytesが変わるため、Human choiceなしにcanonical wordingを確定できない。

### Non-blocking Issues

None identified.

### Questions

- Q1はP1 Fine-grained preservationを採用するか、P2 Target-subtree replacementを採用するか。
- Q2はD1 YAML `null` placeholderを採用するか、D2 Local-only `Unset`を採用するか。

### Approved as Proposed

No. Option Cのintent、Approved Type System、shared Rust boundary、key read-only、validation non-blockingとの整合性にはBlockingを見つけていないが、Q1/Q2はobservable source behaviorを変えるためHuman decisionが必要である。両decision反映後に`Status: Proposed`へ進め、改めてreview-specを通す。

| Review axis | Verdict |
| --- | --- |
| Intent fidelity | Option CとCurrent Objectiveに整合。Q1/Q2は未承認として保持。 |
| Internal consistency | Q1/Q2以外は整合。 |
| Cross-spec consistency | Type System / Source Edit / Record Mutation / Data Editor boundaryを維持。 |
| Terminology consistency | Existing terminologyを使用。 |
| Normative strength | Human-selected directionとApproved constraintの範囲に限定。 |
| Testability | Q1/Q2確定後はfocused acceptanceへ分解可能。 |
| Backward compatibility | Domain/binary changeなし。source-text policyだけQ1で未確定。 |
| Unresolved ambiguity | Q1/Q2がBlocking。 |
| Implementation leakage | exact DTO / component / algorithmは固定していない。 |
| Unrequested behavior | Key mutation、raw YAML editor、bulk edit等を追加していない。 |

## 承認記録（Approval Record）

未承認。Human maintainerのQ1/Q2 decision後にreview可能な`Proposed`へ更新する。
