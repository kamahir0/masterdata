# 仕様変更 0042: record scalarのschema-directed解釈とTable draft

Status: Applied

本artifactはHuman承認時の設計・matrix・reviewを保持する履歴であり、現在のobservable behaviorは下記canonical specificationsが所有する。

## Affected Specifications

- `docs/specs/yaml-subset.md` — `YAML-SUBSET-009..014` のrecord value解釈、`YAML-SUBSET-016` のmapping key境界
- `docs/specs/type-system/primitives.md` — `TYPE-PRIMITIVE-003/004/006/007`。category判定のauthorityを変更し、range・finite・非coercionは維持
- `docs/specs/type-system/field-modifiers.md` — `TYPE-FIELD-001..005` のsource shape解釈
- `docs/specs/type-system/enums.md`、`value-objects.md`、`custom-types.md` — data leafの同一解釈経路
- `docs/gui/table-editor/spec.md` — `GUI-UNIFIED-003..005` のtype / Nullable / Array操作とdiagnostic
- `docs/specs/field-declaration-mutation.md` — `FIELD-DECL-002..005` の通常header操作への適用範囲
- `docs/specs/source-edit.md` — `SOURCE-EDIT-003..012/015` とschema draft Saveの境界
- `docs/specs/build-pipeline.md`、`docs/gui/data-editor/spec.md` — strict Build / GUI diagnosticの共有

## 根拠と分類（Source Evidence and Classification）

- **Human-selected Objective / Requirement**: YAMLはsyntax、MasterData schemaはrecord valueの意味を所有する。GUI / CLI / Build / Migrationはshared Coreの同じ解釈を使う。type / Nullable / Array変更はrecordを変換せず可逆的draftとし、解釈不能なら対象cellへdiagnosticを付ける。Saveability、semantic validity、buildabilityを分け、安全境界を維持する。
- **Human Proposal**: unquoted `null` をNull、quoted `"null"` をScalar textとする。quoteはtype annotationにしない。numeric grammarとcoercionは別途決める。これは確定済みの細部として扱わず、下記Agent Decisionとして具体化する。
- **現行Approved authority**: `YAML-SUBSET-009..014` はparser前にbool / integer / float / stringを分類し、`00123`等をsource errorとする。`TYPE-PRIMITIVE-003` はそのcategoryの再解釈を禁止。`GUI-UNIFIED-004/005` と `FIELD-DECL-002` はheader変更を全record検証済みの即時commitにする。`SOURCE-EDIT-004` はrecord Saveをdomain diagnosticだけで拒否しない。
- **Implementation evidence**: `document.rs` はrecordを `serde_yaml::Value` として保持し、lexeme検査後の`serde_yaml`分類を `type_system.rs`、`table.rs`、`data_authoring.rs`、`source_edit.rs`、Buildへ渡す。原文は `LoadedDocument.source` に別途保持し、値ごとのlexical provenanceはdomain値にない。`pipeline.rs` はvalidation不成立ならBuildPlanを作らず、.NET loweringはTypeSystemのnormalized valueを受け取る。

## A. Current semantic model

```text
source bytes
  → subset lexeme検査 / serde_yaml parse
  → serde_yaml::Value（Bool / Number / String / Null / Sequence / Mapping）
  → SchemaDocument / DataDocument.records
  → TypeSystem category照合、Table resolution、Validation diagnostics
  → GUI snapshot / CLI validate / BuildPlan → C#・normalized .NET request
```

BoolやNumberのcategoryはschema参照前に確定する。`true`はstring fieldでもBoolのままなのでinvalid。`00123`はstring fieldでもlexeme段階でinvalid。GUIのrecord bufferはSave candidateをsource-preservingに作り、domain diagnosticがあってもSaveできる。一方、type header操作は既存の全recordを新宣言で検証してから即時schema commitするため、`bool → int`が拒否される。

## B. Proposed semantic model / Agent Decisions

```text
source bytes
  → subset syntax・duplicate key・explicit value検査
  → structural source value: Scalar(decoded text, style, span/provenance)
                           | Null | Sequence | Mapping
  → table/type declarationからfield shapeを解決
  → shared schema-directed interpreter
  → typed value または位置付きdiagnostic
  → Save / Validate / Buildごとの判定
```

このSourceValueは**record value subtree**に適用する。`$tags`等のrecord metadataはfield schema解釈へ混ぜず、既存の専用契約に従う。document envelope、field declaration、Enum/Flagsのmember numeric declaration、mapping keyは、それぞれ既存のschema syntax / integer / key契約を維持する。raw source bytesとdecoded scalar textをともに保持し、unmodified leafを再renderしない。parser実装の選択はこのobservable contractから独立し、`serde_yaml`が返したBool/Numberを原文やprovenanceなしに再利用してはならない。

**Agent Decision: scalar grammar** — unquoted `null`のみNull。`"null"` / `'null'` はScalar(`null`)。quoted/unquotedはdecoded textが同じなら同じfield意味を持つ。Required stringはNull以外のすべてのscalar textをそのまま受理する。boolはdecoded textが正確に`true`か`false`の場合だけ受理する。integerは `-?(?:0|[1-9][0-9]*)` とtarget range、float/doubleは現行 `YAML-SUBSET-012` のfraction/exponent grammarとfinite target valueを要求する。`1`をfloat/doubleへ数値coercionしない。quoted numeric/boolも同じgrammarで評価する。`00123`、`0xFF`、`+123`、`.5`、`NaN`等のYAML構文上のplain scalar textはrecord stringとして許容し、numeric fieldではdiagnosticとする。ただし`~`、暗黙の空value、unsupported tag/anchor/collection等、既存subsetがsyntaxとして禁止する形式は引き続きsource errorとする。外側のschema metadataやEnum member numeric declarationには従来のstrict grammarを適用する。

**Agent Decision: operation boundary** — value grammarの不一致、range外、field shape不一致、key/reference整合性違反はsemantic diagnosticであり、source textを消さない。source syntax/structural parseの破損、patch不能、exact identity conflict、I/O failure、Recovery Required / Outcome UnknownはSave safety failure。Build Selection後に必要なresolved modelがinvalidならBuildは失敗する。validation対象範囲とselectionの既存契約は変更しない。

## C. Scalar Interpretation Matrix

表はrecord fieldの**Required base type**を示す。`✓x`はtyped value `x`、`×`はその列のtype grammar/member/shape diagnostic。integer値は各列のrange内の場合を示し、range外は `×range`。`E?`は同名のdeclared Enum memberが存在する場合のみvalid。Flagsはsequence必須なのでscalar行はすべて`×shape`。`D`はactual Nullで、Requiredではすべて`×null`。quote付き行のsource textはYAML decode後のtextを括弧内に示す。

| record source | SourceValue | string | bool | int | uint | long | ulong | float | double | Enum | Flags |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `true` | Scalar(`true`) | ✓`"true"` | ✓true | × | × | × | × | × | × | × | ×shape |
| `false` | Scalar(`false`) | ✓`"false"` | ✓false | × | × | × | × | × | × | × | ×shape |
| `123` | Scalar(`123`) | ✓`"123"` | × | ✓123 | ✓123 | ✓123 | ✓123 | × | × | × | ×shape |
| `-123` | Scalar(`-123`) | ✓`"-123"` | × | ✓−123 | ×range | ✓−123 | ×range | × | × | × | ×shape |
| `00123` | Scalar(`00123`) | ✓`"00123"` | × | ×grammar | ×grammar | ×grammar | ×grammar | × | × | × | ×shape |
| `1.0` | Scalar(`1.0`) | ✓`"1.0"` | × | × | × | × | × | ✓1.0 | ✓1.0 | × | ×shape |
| `1e3` | Scalar(`1e3`) | ✓`"1e3"` | × | × | × | × | × | ✓1000 | ✓1000 | × | ×shape |
| `Potion` | Scalar(`Potion`) | ✓`"Potion"` | × | × | × | × | × | × | × | E?`Potion` | ×shape |
| `null` | Null | ×null | ×null | ×null | ×null | ×null | ×null | ×null | ×null | ×null | ×null |
| `"null"` | Scalar(`null`) | ✓`"null"` | × | × | × | × | × | × | × | × | ×shape |
| `"true"` | Scalar(`true`) | ✓`"true"` | ✓true | × | × | × | × | × | × | × | ×shape |
| `"123"` | Scalar(`123`) | ✓`"123"` | × | ✓123 | ✓123 | ✓123 | ✓123 | × | × | × | ×shape |
| `""` | Scalar(empty) | ✓`""` | × | × | × | × | × | × | × | × | ×shape |
| `剣` | Scalar(`剣`) | ✓`"剣"` | × | × | × | × | × | × | × | × | ×shape |
| `"hello: world"` | Scalar(`hello: world`) | ✓same text | × | × | × | × | × | × | × | × | ×shape |
| `hello: world` | YAML syntax error | — | — | — | — | — | — | — | — | — | — |

`Potion`をEnumとして受理するのはmember `Potion`が宣言された場合だけ。`true`等はEnum member name grammar (`^[A-Z][A-Za-z0-9]*$`) を満たさない。float/doubleの表示 `✓1000` は数値の意味であり、sourceの`1e3`を書き換える指示ではない。

| Wrapper / shape | 上記各source行への解釈 |
| --- | --- |
| Value Object (underlying = int/uint/long/ulong/string) | 各行をunderlying列へ委譲。validならnominal wrapper値、invalidなら同じleaf diagnostic。bool/float/double underlyingは現行どおりunsupported。 |
| `Nullable<T>` | `null`行だけ✓Null。syntax error行を除く他のScalar行は同じbase `T`列の結果。必須entryの欠落は別diagnostic。 |
| `Array<T>` | 上記すべてのScalar/Null行は×shape。`[]`は✓empty、sequenceでは各elementにbase `T`列を再帰適用し、invalid elementへindex付きdiagnostic。 |
| Flags Enum | 上記Scalar/Null行は×shape。sequenceの各elementは宣言memberへのcase-sensitive symbolic scalar解釈。unknown/重複/`None`の組合せ規則は既存Enum仕様に従う。 |
| Custom Type leaf | Custom Type全体はmapping必須でscalar/Null行は×shape。mapping内の各declared leafはそのfieldのbase列・modifierを再帰適用。unknown/missing memberは既存diagnostic。 |

**全cell共通のsource / operation結果**: `✓`でも`×`でも、上記のYAML syntaxが安全に表現される限り、その行を変更しないschema draftおよびschema Saveでrecord bytesはbyte-for-byte不変。record自身を編集するときだけ対象の最小rangeをpatchする。`×`はGUI/CLI Validateで同じCore由来の診断となり、Saveの拒否理由にはしない。Buildは既存のprofile-independent / selected-dataset validation範囲で`×`があれば失敗し、`✓`だけなら他の既存Build条件に従う。`hello: world`のunquoted構文エラー行はsource candidateにできずSave不可、Validate/Buildはparse failure。Nullable/Array/Value Object/Customへの委譲でもこの規則は変わらない。

## D. Compatibility analysis

- **Breaking source-language/validity change**: plain `true` / `false` / numberをstring fieldで受理し、quoted `"true"`をbool、quoted `"123"`をintegerとして受理する。`00123`等のplain numeric-looking textはrecord stringで新たに受理する。これまでinvalidだったprojectがvalid・buildableになり得る。CLI Validateのexit結果、GUI diagnostics、Buildの成否が変わる。
- **既存valid fixtureの意味**: `fixtures/showcase` と `fixtures/full` のbool、signed/unsigned各境界、float/double、symbolic Enum、Flags sequence、VO underlying、Custom nested leaf、Nullable null、Array leafは対応する宣言で同じ値に解釈される。`fixtures/minimal`も同様。fixtureが全利用者のsource corpusを代表する証拠ではない。
- **既存invalid test**: `crates/masterdata-core/tests/type_system.rs::primitive_scalar_categories_are_strict` は「bool→stringはinvalid」の期待を置換する必要がある。quoted variant、leading-zero lexemeテストはrecord stringとschema member numeric declarationを分けて更新する。source edit / migration / .NET lowering testsも対象になる。
- **維持する境界**: current valid sourceのrecord bytes、table identity、mapping keys、Enum member numeric declaration、generated C# public shape、MasterMemory binary format、Build Selection scope、key/reference capabilityは変えない。invalidからvalidへの拡張でもpersisted source format / CLI Validate結果のbreaking compatibilityに当たる。local fixture・Git履歴にrelease artifactを確認できないことは外部consumer不在の証明ではない。
- **移行**: 自動一括source変換をしない。以前不正なscalarが新規に有効化されるケースは、Validate結果で確認する。既存valid sourceを強制書換えしない。parser adapter変更時は現行valid corpusのsemantic equivalenceと、invalid→validの差分をfixture/testで固定する。

## E. Authoring state model

Table sessionは、persisted source snapshotのexact identityをfileごとに持ち、schema draft（type / Nullable / Array）と各record source draftを**別fileのdirty state**として保持する。両draftをoverlayしたin-memory candidateでshared interpreterを実行し、同じTableのinline / 全分離Data sourceについてfield/path/source occurrenceへ結び付くdiagnosticを返す。現在のgrid外のdiagnosticもProblemsから失わない。schemaを `bool→int→bool` と戻した時はsource valueを触らず診断が消える。header変更は即時disk mutationでもMigration承認dialogでもない。Undo/Redoは少なくとも未保存のdeclaration変更を1操作単位で戻し、Save成功後の履歴は新baseとの混同を避けて再基準化する。

semantic-invalidな**既存scalar**をsourceとして安全に保持・位置特定できる場合、typed projection失敗だけでcellを修復不能なread-onlyにしない。GUIは元のdecoded textとsource provenanceを保持した編集状態をshared boundaryから受け取り、利用者が置換してdiagnosticを解消できる。構造が不明、またはsafe patchできないcomplex valueは既存`SOURCE-EDIT-015`のread-only reasonを維持する。この境界はvalueをvalidへ黙ってcoerceすることを許さない。

type / Nullable / Arrayは**reinterpretation**。rename/add/dropは**structural source mutation**、dropは加えてdestructive。MessagePack key、Primary/Secondary Key、Referenceは**identity/dependency mutation**。後者は現行Plan/dirty/authorization契約を維持し、このsliceでdraftへ一括移行しない。type変更でkey/reference capabilityが壊れてもheader draftは保持し、その依存宣言にsemantic diagnosticを出す。GUIはsource textをparseせずshared snapshotを描画する。

## F. Persistence model

schema Saveはschema draftを**schema source fileのみ**にsource-preserving patchとして適用する。対象schema fileのbase exact identityとcurrent identityを比較し、staleならConflict。record fileは一切変換・書換えしない。record Saveは既存file-local safety contractを維持し、record draftを対象fileのみへpatchする。schemaとrecordの両方がdirtyでも各Saveを独立に行い、一方のSaveで他方を暗黙保存しない。Save Allは各fileの明示的な上位workflowであり、途中失敗を全成功と表示しない。

Save candidateはsubset syntax、構造、変更対象の再特定、expected patch postcondition、exact identity、write safetyを満たす必要がある。type mismatchやkey/reference整合性などのdomain diagnosticはSave gateにしない。schema Save後、他fileのdraftを保持したまま新baseでdiagnosticを再計算する。record fileへの外部変更があれば、そのfileのdraftはConflictとして保持し、黙ってrebase/overwriteしない。write失敗・Outcome Unknown・Recovery Requiredは既存contractに従い、Successと区別する。structural/destructive operationのmulti-file Planは既存のrollback/recovery境界に従う。

## New / Changed Requirements（Proposed Delta）

1. YAML subsetのrecord value節からprimitive categoryの先行決定を除き、Scalar text / Null / Sequence / Mappingとquote・syntax契約を定義する。mapping key、schema metadata、type declaration numeric memberは独立に既存restrictionを保持する。canonical適用時はmaterially replacedな`YAML-SUBSET-009..014`のscopeと履歴を明示し、record値の新契約へ新IDを割り当てる。
2. `TYPE-PRIMITIVE-003`はrecord source leafをfield schemaに基づいて一度だけ解釈し、typed resultか位置付きdiagnosticを全consumerへ提供するよう改める。`TYPE-PRIMITIVE-004/006/007`のrange・empty・finite規則は維持する。normalized .NET requestはvalid resolved modelからのみ生成する。
3. `GUI-UNIFIED-004/005`の通常type / Nullable / Array header editを可逆的draftにし、record sourceの自動変換とsemantic invalidによるedit拒否を禁止する。`SOURCE-EDIT-015`のinvalid scalarはsource-preservingな修復編集を可能にする。Save / Validate / Buildの判定境界を上記E/Fにする。`FIELD-DECL-002..005`の明示Plan operation自体は保持し、そのstrictなall-record preconditionを通常のdeclaration reinterpretationへ適用しない。structural/identity mutationの既存安全性を縮小しない。

## 受け入れと実装への影響（Acceptance and Implementation Impact）

- Golden Cases: `bool→int`はdraft成功＋2セル診断、`int→bool`で診断消失、`bool→string`は`true`を文字列として解釈、`int→string`は`123`を文字列として解釈、`string→int`で`abc`に診断、`null`と`"null"`区別、GUI/CLI同一診断由来、semantic-invalid Save成功・Build失敗。
- Core parser adapter / document record representation / type resolution / table resolution / authoring projection / source edit / Migration preflight / .NET normalizationの順で同一解釈経路を作る。frontendにType SystemやYAML解釈を複製しない。literal sourceとtyped resultを区別し、64-bit値・quote・comment・source spanを失わない。
- Focused regression: matrix各row×primitive、wrapper/recursive leaf、key/reference診断、inline/separate source、exact bytes、undo/redo、stale/dirty/rollback、CLI Validate/Build parity。現行valid fixtureのsemantic resultとbinary結果を比較し、Desktopでkeyboard/pointerのheader編集、診断移動、Saveを確認する。

## 未解決事項（Open Questions）

- **解決済み**: source-language・CLI validityのbreaking changeは2026-09-27のHuman decisionで採用された。
- parser libraryを維持してsource AST adapterを追加するか、別parserへ移るかは実装検証で決める。どちらも同じobservable matrixとsource safetyを満たす必要がある。parser migrationそのものはRFC 0002の独立decision。
- 不正なYAML syntaxをエディタでraw textとして一時保持する機能、schema以外の全operationのdraft統合、visual polishは今回の承認範囲外。

## Potential ADRs

承認された場合、record SourceValueとtyped/resolved valueを分離するcross-cutting architecture WHYを新ADRへ記録する。parser libraryの採否や実装型名をADRに固定しない。

## Approval Eligibility

- Autonomous approval eligible: **No**（Human approvalを取得済み）
- Human gate: `docs/execution-workflow.md#human-gate` の **Breaking compatibility**（2026-09-27のHuman decisionで充足）。
- 推奨: 本提案のschema-directed record scalar解釈を採用する。現行valid sourceは保持し、invalid→validの拡張を明示したうえでshared Coreへ実装する。

## レビュー（Review）

refinement後のfresh passで、依頼文、現行Approved specs、Core経路、fixture/testを再照合した。

- **Blocking Issues**: None identified。matrixはrequired source/type組合せ、shape wrapper、source/Save/Build判定を覆う。
- **Non-blocking Issues**: parser adapterがquote/style/spanをlosslessに抽出できることは実装時にfixtureで証明する。float/doubleの丸め・finite判定は現行Type Systemのtarget value規則を維持し、parser固有の数値分類をauthorityにしない。
- **Questions**: breaking compatibility採否のみHuman decision needed。採用後の`~`維持、非coercion、file-local Saveは本proposalで決定済み。
- **Approved as Proposed**: Yes（semantic review verdictのみ。`Status: Proposed`のまま）。
- **Autonomous approval eligibility**: Eligible: No。Human gate: Breaking compatibility。Rationale: source languageのaccepted set、CLI ValidateとBuildの成功/失敗を変更する。
- **Review dimensions**: Intent fidelityとnormative strengthはHuman RequirementとAgent Decisionを分離。internal/cross-spec consistencyは旧`FIELD-DECL`明示Planと新通常draftを分離し、`SOURCE-EDIT-015`のread-only例外を明記。testabilityとfailure semanticsはmatrix・Golden Cases・exact identity / Outcome Unknownで確認可能。backward compatibilityはvalid fixture保持とinvalid→valid拡張を区別。terminology / implementation leakageはSourceValueをconceptual modelとしwire形・parser libraryを固定しない。documentation ownershipはapproval前artifact、approval後canonical spec / ADRへroute。unrequested UI polishと全schema operationのdraft化は除外。

## 承認記録（Approval Record）

Approval mode: Human。2026-09-27、Humanは提案0042全体（Scalar Interpretation Matrix、authoring state、persistence model、Agent Decisionsを含む）を明示的に採用した。semantic reviewはBlockingなし、Approved as Proposed: Yes。canonical applicationは本変更のGit履歴で追跡する。`docs/specs/yaml-subset.md`、`type-system/primitives.md`、`type-system/field-modifiers.md`、`type-system/enums.md`、`field-declaration-mutation.md`、`source-edit.md`、`build-pipeline.md`、`docs/gui/table-editor/spec.md`、`docs/gui/data-editor/spec.md`へ適用し、architecture WHYはADR 0007へ記録した。
