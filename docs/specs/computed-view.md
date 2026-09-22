# Computed View仕様

Status: Implemented

Domain: Advanced Authoring

## 位置付け

Computed Viewは、保存済みcanonical Masterdata snapshotから、Tableの各recordについて純粋なscalar
expressionを評価するauthoring-onlyのread-only projectionである。これはTable field、MessagePack field、
generated C# property、Reference helper、MasterMemory binary value、またはUnity runtime expressionではない。
この仕様がComputed Viewのpersisted source、expression、解決、評価、Overview/query、migration boundaryを所有する。

Table / Key、Type System、Authoring Query、Schema Migration、Released Compatibilityの既存仕様は各自の意味を
所有し、この仕様はそれらをderived authoring projectionへlowerする。

## Persisted document

Computed Viewは既存schema/data/type documentとは別の、明示的な`kind: view` source documentとして保存する。
path、filename、directory、UI titleはidentityではない。

```yaml
kind: view
name: itemDisplay
table: item
columns:
  - name: displayName
    expression: 'name + " (" + category + ")"'
  - name: discountedPrice
    expression: 'price * 0.9'
```

`name`はproject-localなlowerCamel ASCII identifierであり、project内でuniqueでなければならない。`table`は
既存Tableのlogical identityである。`columns[].name`はview内でuniqueなlowerCamel ASCII identifierであり、
declaration orderはprojectionのcolumn orderだけを決める。view/column nameはいずれもgenerated C# identifierや
stable IDではない。view documentにunknown field、computed-column reference、aggregate、join、query、runtime
codeを許可しない。

`expression`は人間がreviewできるUTF-8 text scalarとして保存する。内部のtyped AST / resolved formは実装で
構築してよいが、通常のauthoringやmigrationで全source textをcanonical formatterへ置換してはならない。

## Expression grammar

v1 grammarは、次のoperatorだけを持つboundedなexpression languageである。whitespaceは任意で、field referenceは
target Tableのsource field symbolだけを指す。

```text
expression  := conditional
conditional := coalesce ('?' conditional ':' conditional)?
coalesce   := or ('??' or)*
or         := and ('||' and)*
and        := equality ('&&' equality)*
equality   := relation (('==' | '!=') relation)*
relation   := additive (('<' | '<=' | '>' | '>=') additive)*
additive   := multiplicative (('+' | '-') multiplicative)*
multiplicative := unary (('*' | '/' | '%') unary)*
unary      := ('!' | '+' | '-') unary | primary
primary    := literal | identifier | enum_literal | '(' expression ')'
literal    := null | true | false | integer | float | string
enum_literal := type_identifier '.' member_identifier
```

String literalはdouble quoteとJSON-compatible escapeを使用する。integerは符号を含まないdecimal literal、
float literalは小数点または指数を含むdecimal literalとし、numeric literalはcontext typeへ暗黙変換しない。
`null`はnull literalであり、identifierはASCII lowerCamel field nameである。`enum_literal`はresolved normal
Enumの型名とmember名を明示する唯一のdot syntaxであり、general member accessではない。function call、index、
assignment、loop、lambda、interpolation、regex、filesystem/network/environment accessは存在しない。

型検査は全branchとoperandを解決してから成功しなければならない。computed columnから別computed columnを
参照する構文はv1では禁止するため、dependency cycleは入力として作れない。未知fieldやmalformed grammarは
definition diagnosticであり、row evaluation failureではない。

## Type and value semantics

既存Type Systemのresolved field shapeを再利用する。Required / NullableのPrimitive、key-compatible scalar
Value Object、normal Enumを参照できる。Array、Flags、Custom Type、unresolved fieldはv1 operandとして
unsupportedである。expressionは独自の第二Type Systemを作らず、base semantic typeとnullable modifierを
resolved expressionへ保持する。

implicit conversionはない。二項operatorの両辺は、`null` literalを除き同じsemantic scalar typeでなければ
ならない。Value Objectはunderlying primitiveへ自動unwrapせず、Enumはnumeric valueへcastしない。stringの
compositionはstring `+` のみであり、numeric/bool/Enumをstringへ自動変換しない。

- `+`, `-`, `*`, `/`, `%` は同一numeric primitive（int / uint / long / ulong / float / double）だけに適用する。
- integer arithmeticはdeclared width/rangeでcheckedとし、overflow、signed/unsignedの混在、division by zero、
  `MIN / -1` overflowをrow evaluation errorとする。divisionはintegerではzero方向へのtruncationとする。
- float/doubleはfinite resultだけをvalidとし、non-finite literal/result、division by zeroをrow evaluation
  errorとする。
- numeric comparisonは同一numeric primitive、string comparisonはordinal case-sensitive string同士、boolは
  equalityだけ、normal Enumは同じEnumのsymbolic valueのequalityだけを許可する。
- `&&` / `||` はnon-null bool同士、`!`はnon-null boolだけを許可する。JavaScript truthinessやSQL
  three-valued logicは使用しない。
- `==` / `!=` は同じsemantic type同士に適用し、nullとの比較だけはnull-awareである。`null == null`はtrue、
  nullとnon-nullの`==`はfalse（`!=`は逆）とし、orderingでnullを比較しない。
- `condition ? whenTrue : whenFalse` はnon-null bool conditionを要求する。両branchは同じbase semantic
  typeで、片方がnullなら結果はNullableとなる。
- `left ?? right` はleftがNullable、rightが同じbase semantic typeである場合に許可し、leftがnon-nullなら
  left、nullならrightを返す。resultはrightのnullabilityを反映する。

fieldがnullなら、null-aware operatorを除く算術・ordering・boolean operatorはそのrowでunavailableとなる。
nullを安全にfallbackするには`field ?? literal`を使用する。source fieldのtyped projectionがinvalidな場合は
computed cellもinvalid/unavailableとなり、0、空文字、falseへcoerceしない。

computed cellは少なくとも`valid(value)`、`null`、`invalid/unavailable(diagnostic)`を区別する。definitionが
validでもrecord valueに依存するoverflow、division by zero、invalid source valueはrow evaluation diagnostic
となり、他recordの評価を黙って置換しない。

## Resolution and evaluation

shared Rust coreは、次の順で一回のOverview/query snapshotを準備する。

```text
canonical source load
  -> Type System / Table resolution
  -> view target/dependency/type resolution
  -> base typed rows
  -> compiled expression evaluation per row
  -> Authoring Query search/filter/sort
  -> profile presentation / selected-only filtering
  -> read-only UI projection
```

view evaluationはBuild Profileのselected datasetを暗黙の入力にせず、保存済みsnapshotの全base rowsを対象に
する。Profileは既存Overviewのpresentation / selected-only compositionへ適用する。未保存Data Editor bufferは
評価へ混ぜない。source pathとrecord indexはprovenanceでありvalueやidentityの代替ではない。

同じsource snapshot、view definition、query、profileから、view column order、row order、cell result、diagnostic
orderが同じにならなければならない。filesystem traversalやHashMap iteration orderをobservable orderに使わない。
view definitionごとのtarget Tableは一意に解決されなければならず、unknown/ambiguous targetはview-specific
Unavailableとする。invalid viewが安全に分類できる限り、raw source Explorerと他TableのOverviewを破壊しない。

## Query composition

resolved computed scalarは既存Authoring Queryのscalar capabilityへ接続する。search、supported filter、sortの
operatorとcomparison semanticsをfrontendやview moduleが複製してはならない。computed output typeが既存query
capability外（bool search、float sort等）の場合はunsupported query diagnosticを返し、別のoperatorへ黙って
fallbackしない。computed columnsはOverviewでbase columnsの後に、view declaration order、view document identity
の安定順で表示する。Data Editorのeditable source cellには表示しない。

## Diagnostics

definition parse/type diagnosticsとrow evaluation diagnosticsを分離する。少なくとも次をstructuredに識別でき、
可能な限りsource span（line/column/byte range）、view path、view name、column name、record provenanceを返す。

- `E-VIEW-PARSE`, `E-VIEW-EXPRESSION-SYNTAX`
- `E-VIEW-TARGET-TABLE`, `E-VIEW-DUPLICATE-NAME`, `E-VIEW-DUPLICATE-COLUMN`
- `E-VIEW-UNKNOWN-FIELD`, `E-VIEW-UNSUPPORTED-OPERAND`, `E-VIEW-TYPE-MISMATCH`, `E-VIEW-DEPENDENCY-CYCLE`
- `E-VIEW-ROW-EVALUATION`, `E-VIEW-ARITHMETIC`, `E-VIEW-QUERY-UNAVAILABLE`
- `E-VIEW-SNAPSHOT-STALE`, `E-VIEW-SOURCE-CONFLICT`

Requirement IDとruntime Diagnostic Codeは混同しない。diagnostic orderはsource path、view name、column declaration
order、record source order、expression spanの順で安定化する。

## Source authoring and migration

View create/edit/removeはshared application/core operationを通じ、exact source identityとbase bytesを持つ
source-preserving Plan / Applyを使用する。Planはfilesystemを変更せず、Apply直前にsource membership/contentを
再確認し、stale sourceをConflictとして拒否する。comments、unrelated YAML、expression外のwhitespace/quoteは保持し、
Build、Publish、Git operationを暗黙に開始しない。frontendはYAML serializer、expression parser、dependency resolver、
patch engine、filesystem transactionを実装しない。

`RenameField`は、target Tableを明示的に参照するview expression内のfield tokenを安全に定位できる場合だけ、その
token spanをsource-preservingに置換し、再parse/type-check/postcondition検証する。expression全体のpretty-print、
quoted string内の同名文字列の置換、unlocatable/ambiguous patchはfail closedする。unrelated view sourceはbyte-for-byte
不変とする。`DropField`は依存viewが存在する場合、暗黙削除・null置換・replacement推測をせずfail closedする。
他のfield evolution（例えばAddField）がView column collisionまたは既存invalid definitionを生む場合も、sourceを
staleにせずdependency/precondition failureでfail closedする。Type Migration後もviewを再parse/type-checkし、既存
operationが安全にsymbolを追随できる場合以外はpostcondition failureとする。

## Build and compatibility boundary

source discovery、validation、authoring Overviewは`kind: view`を認識するが、Buildはview documentをMasterMemory
schema、generated C#、binary record、artifact receiptへlowerしない。viewの追加・編集・削除だけでruntime Table/API/
binary changeがあると推測してはならない。

Released Compatibilityはviewをauthoring-only subjectとして扱い、Generated APIとArtifact/Binaryはno runtime impact
（changeなし）とする。必要なsource authoring impactは既存axis ownerへroutingできるが、view definitionをgenerated
runtime contractへ昇格させない。view definitionのsemantic comparisonはview name/table/column/expressionの既存
source semanticsを使い、path/format/commentをidentityにしない。

## Verification

Rust focused testsはgrammar、span、type checking、strict no-coercion、null、invalid source、checked arithmetic、
determinism、multiple rows、query composition、diagnosticsをcoverする。application testsはview CRUD、source-preserving
patch、stale/lost-update、RenameField追随、DropField fail-closed、no implicit Build/Publishをcoverする。Overview/query
testsはbase+computed column、search/filter/sort、invalid cell、profile composition、saved snapshot boundaryをcoverする。
Build/codegen regressionsはview add/edit/removeでgenerated C# / MasterMemory schema / binary inputが変わらないことを
証明する。frontend testsはshared snapshotのpresentation/wiringだけを検証し、expression semanticsを再実装しない。

## 非目標

- generated C# computed property、MasterMemory binary field、Unity runtime evaluator
- embedded scripting、filesystem/network/environment access、side effect
- aggregate、group-by、arbitrary join、cross-project query、recursive Reference traversal
- persistent stable member ID、rename lineage、tombstone、released compatibility identity
- view resultのsource materialization、Data Editorでのcomputed cell編集
- Web / Browser / Native Host surface

## Open Questions

None for Computed View v1. exact Rust struct names、JSON field casing、UI componentはimplementation detailとする。
