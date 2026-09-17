# Authoring Batch仕様

Status: Approved

Domain: Source Editing

この仕様は、Desktop制作v1における1 file内のscalar一括編集とclipboard codecを定義する。適用記録は[仕様変更0016](../spec-changes/0016-desktop-daily-editing.md)を参照する。通常のsource edit、record mutation、Type System、GUI interactionの既存契約はそれぞれのownerを維持する。

## 規範要件

### AUTHORING-BATCH-001

一括編集は一つのfileのbase snapshot、current local buffer revision、resolved schema/type revision、ordered target occurrence/fieldと入力を受け、shared applicationでcandidateを作らなければならない（MUST）。既存recordとAdded draftを識別し、PKだけ、表示row indexだけでtargetを解決してはならない（MUST NOT）。wire形は固定しない。

対象はtop-level fieldのRequired / Nullable Primitive、Value Object、normal Enumとする。Flags、Custom Type、Array、nested leafをrange対象にしてはならない（MUST NOT）。existing key、Pending delete、unsupported/unresolved fieldは編集対象外。Added draftのkeyは既存契約に従って入力可能とする。

### AUTHORING-BATCH-002

operationは「全targetのcandidateを適用」または「buffer不変」のどちらかでなければならない（MUST）。source定位不能、duplicate target、read-only混入、矩形不足、codec failureはoperation failureとし、黙ってskip/truncateしてはならない（MUST NOT）。domain-invalid valueのdiagnosticはoperation failureやSave gateにしてはならない（MUST NOT）。同一入力でbyte-identical candidateならno-opとする。

既存source-preserving patchとrecord mutation compositionを使い、既存local editsを失ってはならない（MUST NOT）。適用はbufferだけに行い、Save / Build / Migrationを暗黙実行してはならない（MUST NOT）。

### AUTHORING-BATCH-003

clipboard textはshared codecで矩形TSVとしてdecodeしなければならない（MUST）。以下をv1 codecとする。

- separatorはTAB。quoted field外のLFまたはCRLFをrow delimiterとする。bare CRはquoted field内でのみ許可する。
- field先頭の`"`はquoted field開始。内部の`""`はliteral `"`。終端quote後はseparator、row delimiter、EOFだけを許可する。
- unquoted fieldのquoteはmalformed。quoted field内のTAB / LF / CRLF / CRは文字としてそのまま保持する。
- 末尾のrow delimiter一個は終端とし、余分なempty rowを作らない。二個目以降はempty rowを表す。末尾TABは最後のempty fieldを作る。
- 空textは1 row × 1 empty field。row幅不一致、未閉quote、終端quote後の余分な文字は全体failure。header推測・trim・Unicode normalizationを行わない。

copyは全fieldをquoteし、内部quoteを二重化、列はTAB、行はLF、最後のrow delimiterなしでencodeしなければならない（MUST）。これによりempty string、embedded newline、末尾empty fieldを区別する。HTML clipboardやspreadsheet formulaを実行してはならない（MUST NOT）。

### AUTHORING-BATCH-004

decodeした各field textはshared resolved typeで次のauthoring inputに変換しなければならない（MUST）。TSV quotingはdelimiter escapingだけで、YAML quote指定ではない。

| 対象 | 入力の意味 |
| --- | --- |
| string / string-underlying Value Object / normal Enum | text全体をstringとして保持。`null`、`001`、`=...`もliteral。Enumの未知memberはdomain-invalid |
| 数値 / bool / numeric Value Object | YAML subsetのplain numeric/bool tokenとしてlosslessに表せるtextだけをそのcategoryとして保持。domain range / category不一致はdiagnostic。quoted YAML、collection・tag・alias・unsupported scalar、それ以外のtextは全体をliteral stringとして保持しdomain-invalidを示す |
| すべてのscalar対象でempty text | empty string。0、null、未入力placeholderに変換しない |

`null` textはstring系では文字列、数値/bool系でもliteral stringとして保持し、null化の暗黙syntaxにはしない（MUST NOT）。Nullableのnull化は明示`Set Null`で行い、Requiredを含むrangeへは適用しない。
数値/bool入力に前後whitespaceがある場合もtrimせずliteral stringとして保持する。例えばint列の`1.0`、`abc`、range外整数はcoerceせずdiagnostic対象とする。

copyはvalid non-null scalarのみを対象とし、string/Enumはliteral value、boolは`true`/`false`、integerはexact decimal、float/doubleはshared layerのround-trip可能なfinite decimal textとする（MUST）。float/doubleの整数相当値にも小数点または指数を含め、paste時にinteger categoryへ変わらない表現とする。null、invalid、complexを含むrangeはcopyを理由付きで停止し、曖昧なempty cellとして出力してはならない（MUST NOT）。read-only keyのcopyは許可する。cutはv1外。

## 受け入れ証拠

同PK別occurrence、Added key、existing key混入、stale revision、no-op、invalid値のall-or-none buffer適用を検証する。codecはquoted TAB/LF/CRLF/quote、末尾TAB/改行、空text、ragged row、`ulong::MAX`、intへ`1.0`、literal `null` / `=...`、null copy拒否を検証する。
