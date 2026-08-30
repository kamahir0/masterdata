# Masterdata YAML subset仕様（Masterdata YAML subset）

Status: Proposed

Domain: Schema Language

## 概要

本proposalは、Masterdataが受理するYAMLの構造・collection・scalar subsetを、特定のYAML parser/libraryの挙動から独立して定義する。
parser libraryはこのproduct contractを暗黙に決定してはならず（MUST NOT）、implementationはsubset boundaryで明示的に検証する。

このdocumentは、source documentの単位、mapping/collection、comment、scalar classification、およびunsupported YAML constructの
canonical ownerである。document envelopeとschema declarationは[Schema言語仕様](schema-language.md)が、Primitive Typeのtarget
value domainは[Primitive Types仕様](type-system/primitives.md)が所有する。parser libraryの選択は
[YAML parser/library RFC](../rfcs/0002-yaml-parser-library.md)で別途扱い、このproposalからmigrationを導出しない。

## 用語

`Masterdata YAML subset` は、Masterdata productが受理するYAML syntaxとscalar classificationの範囲である。`source file` は、
ちょうど1つのMasterdata YAML documentを含むfileである。`mapping key` はYAML mappingのmember name、`scalar category` はboolean、
null、integer、floating-point、またはstringとしてsubsetが分類するleaf valueを指す。

## 規範要件

### YAML-SUBSET-001

MasterdataのYAML subset semanticsは、選択されたYAML parser/libraryのimplicit typing、default construct support、またはerror
behaviorだけから決めてはならない（MUST NOT）。parser implementationは、このdocumentが定めるproduct subsetへ入力を適合させ、
unsupported constructを受理してはならない（MUST NOT）。

### YAML-SUBSET-002

1つのsource fileは、正確に1つのMasterdata YAML documentに対応しなければならない（MUST）。explicit document start marker `---`、
explicit document end marker `...`、複数document、`%YAML`、`%TAG`、およびその他のYAML directiveはサポートしてはならない
（MUST NOT）。

### YAML-SUBSET-003

すべてのmappingにおいてduplicate mapping keyを禁止しなければならない（MUST）。duplicate keyはstructural parse/source errorで
あり、first-winsまたはlast-winsとして解釈してはならない（MUST NOT）。

### YAML-SUBSET-004

Masterdata-owned structureに対応するmapping内のunknown semantic memberはinvalidでなければならず（MUST）、silent ignoreしては
ならない（MUST NOT）。structure-specificなmember shapeとfailureのcanonical ownerは各domain specificationに置く。例えば、
Custom Type data mappingのunknown memberは[Custom Type仕様](type-system/custom-types.md)の `SCHEMA-CUSTOM-007` が所有する。
このrequirementは、GUI saveがunknown field、comment、formattingを保持することを意味しない。

### YAML-SUBSET-005

anchor `&name`、alias `*name`、およびmerge key `<<` はサポートしてはならない（MUST NOT）。merge keyはaliasを別途サポートする
かどうかにかかわらずunsupportedである。

### YAML-SUBSET-006

explicit YAML tagはすべてサポートしてはならない（MUST NOT）。`!!str`、`!!int`、`!!timestamp`、`!ItemId`などのstandardまたは
custom tagを含む。

### YAML-SUBSET-007

block mappingはサポートしなければならない（MUST）。flow mappingはサポートしてはならない（MUST NOT）。block sequenceとflow
sequenceはサポートしなければならない（MUST）。

### YAML-SUBSET-008

YAML commentはsource内で許可しなければならない（MUST）。commentはMasterdata semanticを持たず、domain data、validation
semantics、binary semanticsを変更してはならない（MUST NOT）。GUI save operationがcomment、formatting、またはquote styleを
exactに保持するかどうかは、このrequirementでは定義しない。

### YAML-SUBSET-009

boolean literalは、unquotedな `true` または `false` だけでなければならない（MUST）。`yes`、`no`、`on`、`off`をbooleanとして
分類してはならない（MUST NOT）。

### YAML-SUBSET-010

null literalはunquotedな `null` だけでなければならない（MUST）。`~`をnull shorthandとしてサポートしてはならない（MUST NOT）。
quotedな `"null"` はstring scalarとして扱わなければならない（MUST）。

### YAML-SUBSET-011

unquoted integer scalarはordinary base-10 syntaxだけを使用しなければならない（MUST）。lexical grammarは次である。

```text
-?(?:0|[1-9][0-9]*)
```

`0`、`123`、`-123`はsupportedである。hexadecimal（`0xFF`）、octal（`0o755`）、binary（`0b1010`）、numeric separator
（`1_000`）、explicit leading `+`、および`0`以外のleading-zero formはサポートしてはならない（MUST NOT）。したがって、
`00`、`00123`、`-00123`、`+123`はinvalidである。signed/unsignedのlegalityとtarget typeのrangeはPrimitive Types仕様の
semantic validationが所有する。

### YAML-SUBSET-012

unquoted floating-point scalarは、decimal fractionまたはdecimal exponentによってfloating-point syntaxであることが明示され
なければならない（MUST）。subsetのlexical grammarは次である。

```text
-?(?:[0-9]+\.[0-9]+(?:[eE][+-]?[0-9]+)?|[0-9]+[eE][+-]?[0-9]+)
```

`1.0`、`-0.5`、`1e3`、`1E3`、`1e+3`、`1e-3`、`1.5e-2`はsupportedである。`.5`、`1.`、`+1.5`、`1_000.0`、`NaN`、
`Infinity`、`+Infinity`、`-Infinity`はサポートしてはならない（MUST NOT）。leading `+`は許可せず、exponent内部の`+`または`-`は許可する。
既存Primitive Types仕様のfinite-only ruleは引き続き適用される。

### YAML-SUBSET-013

scalar category間でimplicit numeric coercionを行ってはならない（MUST NOT）。integer scalar `1`はnumeric conversionによって
`float`または`double` fieldを満たしてはならず、floating-point scalar `1.0`はinteger fieldを満たしてはならない。target typeの
strict validationは[Primitive Types仕様](type-system/primitives.md)に従う。

### YAML-SUBSET-014

single-quoted stringとdouble-quoted stringをサポートしなければならない（MUST）。quote styleはdomainまたはbinary semanticsを
変更してはならない（MUST NOT）。defined scalar categoryに一致しない通常のplain scalarはstringとして扱わなければならない
（MUST）。例えば、`Potion`、`consumable`、`region-jp`、およびboolean literalではない`yes`、`no`、`on`、`off`はordinary
string scalarである。不正なnumeric-looking formやunsupportedなnull/numeric token（`00`、`00123`、`-00123`、`+123`、
`0xFF`、`0o755`、`0b1010`、`.5`、`1.`、`+1.5`、`1_000.0`、`NaN`、`Infinity`、`+Infinity`、`-Infinity`、`~`）を通常のstringへ
fallbackさせてはならない（MUST NOT）。これらは`YAML-SUBSET-010`、`YAML-SUBSET-011`、`YAML-SUBSET-012`に従いinvalidまたは
unsupportedとする。

timestamp-looking plain scalarは、このrequirementのstring fallbackから除外する。unquoted timestamp-looking plain scalarの意味は
`Open Questions`で扱う。

### YAML-SUBSET-015

literal block scalar `|`をサポートしなければならず（MUST）、そのdecoded stringではliteral block内のapplicable newlineを保持
しなければならない（MUST）。folded block scalar `>`はサポートしてはならない（MUST NOT）。このdocumentは、custom folding behaviorを
追加してはならない（MUST NOT）。

## 検証ルール

source fileごとにdocument数、directive、document marker、duplicate mapping key、anchor/alias/merge、explicit tag、collection
shape、comment、scalar categoryを検証する。scalarがtarget Primitive Typeへ渡される場合、scalar categoryとrepresentable valueは
[Primitive Types仕様](type-system/primitives.md)のstrict validationへ渡され、implicit coercionを行わない。

`kind`、`table`、`records`、schema fields、type declarationなどMasterdata-owned memberの具体的なrequired/unknown ruleは、
[Schema言語仕様](schema-language.md)、[Custom Type仕様](type-system/custom-types.md)、その他のcanonical ownerへ委譲する。

## 互換性

このproposalはsource YAMLの解釈可能範囲を定義する。duplicate key、unsupported construct、scalar classificationの変更は、同じ
source textのparse結果、diagnostic、domain data、binary outputを変更し得るため、将来のstatus promotionまたはparser migrationには
compatibility corpusと明示的なchange reviewが必要である。quote styleとcommentはdomain/binary semanticsを持たない。

parser libraryの変更はこのsubset contractを変更せず、選択されたlibraryがsubsetを満たすようadapterまたはvalidation boundaryを
提供しなければならない。`serde_yaml`から別libraryへのmigration、round-trip editorのexact preservation、released schema migrationは
このproposalでは行わない。ApprovedのPrimitive Types仕様とのparser-boundaryの接続は、別途[仕様変更proposal 0002](../spec-changes/0002-yaml-subset.md)
で追跡し、このproposal自体がApproved semanticsを直接変更することはない。

## 受け入れ証拠

| Requirement | Success observation | Failure observation |
| --- | --- | --- |
| `YAML-SUBSET-001` | 異なるparser candidateでもproduct subsetのclassificationとreject ruleが同じである。 | parserのdefault implicit typingだけでproduct behaviorが決まる。 |
| `YAML-SUBSET-002` | 1 file 1 documentが受理される。 | `---`、`...`、directive、複数documentが受理される。 |
| `YAML-SUBSET-003` | unique key mappingが受理される。 | duplicate keyがfirst-wins/last-winsで受理される。 |
| `YAML-SUBSET-004` | unknown semantic memberがerrorになる。 | unknown memberがsilent ignoreされる、またはGUI preservationをsemantic acceptanceとみなす。 |
| `YAML-SUBSET-005` | 通常のmapping/sequenceが受理される。 | anchor、alias、`<<` mergeが受理される。 |
| `YAML-SUBSET-006` | explicit tagなしのscalarが受理される。 | `!!str`、`!!int`、`!!timestamp`、custom tagが受理される。 |
| `YAML-SUBSET-007` | block mapping、block sequence、flow sequenceが受理される。 | flow mapping `{ itemId: 1001 }`が受理される。 |
| `YAML-SUBSET-008` | full-line/inline commentを含む入力のdomain/binary resultがcommentなしと一致する。 | commentがdomain valueやbinary semanticsを変更する。 |
| `YAML-SUBSET-009` | `true`/`false`だけがbooleanになり、`yes`/`no`/`on`/`off`はbooleanにならない。 | YAML libraryの広いboolean resolutionが採用される。 |
| `YAML-SUBSET-010` | `null`だけがnullになり、`~`がrejectされ、quoted `"null"`がstringになる。 | `~`がnull shorthandとして受理される。 |
| `YAML-SUBSET-011` | `0`、`123`、`-123`がinteger scalarになる。 | hex、octal、binary、separator、leading `+`、leading zero formが受理される。 |
| `YAML-SUBSET-012` | fraction/exponent formがfloating scalarになり、finite-only ruleが適用される。 | `.5`、`1.`、`+1.5`、`NaN`、`Infinity`、`+Infinity`、`-Infinity`が受理される。 |
| `YAML-SUBSET-013` | integer `1`とfloating `1.0`が互いのtarget fieldをcoercionなしに満たさない。 | numeric conversionでcategory mismatchが隠される。 |
| `YAML-SUBSET-014` | single/double quoteとplain stringが定義どおりにstringになり、quote styleでdomain resultが変わらない。 | quote styleがbinary semanticsを変更する、`yes`等がbooleanになる、またはunsupportedなnumeric-looking/null tokenがstringへfallbackする。 |
| `YAML-SUBSET-015` | `|` block scalarがliteral newlineを保持する。 | `>`が受理される、またはcustom foldingが適用される。 |

## 例

次はnon-normativeな例である。

```yaml
kind: data
table: item
records:
  - itemId: 1001
    enabled: true
    label: 'Potion'
    description: |
      Line 1
      Line 2
    $tags: [debug, development]
```

次はunsupportedまたはinvalidなconstructの例である。最後のtimestamp-looking plain scalarはOpen Questionの例であり、ここでは
結論を示さない。

```yaml
---
kind: data
table: item
records: { itemId: 1001 }
value: !!str 1001
```

```yaml
kind: data
table: item
records:
  - enabled: yes
    count: 1.0
    value: .5
    timestamp: 2026-08-30
```

## 未解決事項（Open Questions）

- `2026-08-30`、`2026-08-30T12:34:56Z`のようなunquoted timestamp-looking plain scalarをreject、string、将来のDate/DateTime scalar、または別の明示的ruleとして扱うか。YAML libraryのimplicit timestamp typeをblindly採用してはならない。quotedな`"2026-08-30"`はunambiguously stringである。このquestionはDate/DateTime type designまで延期する。
- literal block scalarのindentation、chomping indicator、empty lineなど、`|` のexampleを超えるdecoded newlineの詳細をどう定義するか。
- single/double-quoted scalarのescape、Unicode文字列、flow sequenceの細かなpunctuationなど、明示的に定義していないparser dialectの境界をどうするか。
- source span、diagnostic code、duplicate/unsupported constructのerror severityをどう割り当てるか。
- GUI saveでcomment、formatting、quote、orderingを保持する必要があるか。
- YAML parser/libraryの採用、migration、maintenance policyをRFC 0002の比較からどう決定するか。

## 非目標

このproposalは、YAML parser/library migration、round-trip editorの実装、GUI save preservationの最終contract、schema/type/index/reference
のdomain semantics、Date/DateTime type、MasterMemory binary format、または新しいPrimitive Typeを実装・確定しない。
