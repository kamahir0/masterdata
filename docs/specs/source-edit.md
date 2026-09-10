# Source Record Edit仕様

Status: Proposed

Domain: Source Editing

## 概要

本仕様は、GUI等のauthoring surfaceから既存Data document内のrecord valueを変更し、YAMLをSource of Truthのままfile単位で安全に保存するためのobservable contractを定義する。

YAML syntax / scalar classificationは[Masterdata YAML subset](yaml-subset.md)、Table / record semanticsは[Table / Primary Key / Secondary Key](table-and-keys.md)、Primitive value domainは[Primitive Types](type-system/primitives.md)、host compositionは[Runtime hosts](runtime-hosts.md)が所有する。本仕様はそれらを再定義せず、source snapshot、record provenance、source-preserving patch、file commit、lost-update防止、およびsave resultを所有する。

Schema Migrationはproject-wideなschema transformationであり、通常のrecord value editのauthorityではない。ただし[Schema Migration v1](schema-migration.md)のsource-preserving rewriteとlost-update safetyを、この単一file edit contractの既存安全性evidenceとして参照する。

## 用語

- **Base snapshot**: editorが対象fileを読み込んだ時点、または最後の成功Save後に確定した、そのsource fileのexact content identityとsource provenance。
- **Local buffer**: base snapshotに対する未保存のrecord value変更を反映したeditor側の作業状態。
- **Source provenance**: logical Table identityとは別に、base snapshot内のどのData document・record occurrence・field source locationを編集対象としているかを特定する情報。
- **Save candidate**: 1つのsource fileについて、base snapshotへ現在のlocal editsをsource-preservingに適用して得る保存予定content。
- **Conflict**: Save直前のworkspace sourceがbase snapshotと一致せず、通常Saveでlost updateを避けるためcommitを停止した状態。
- **Outcome Unknown**: write開始後のI/Oまたはhost failureにより、workspace上の最終contentがold/newのどちらであるかを安全に断定できない状態。

## 規範要件

### SOURCE-EDIT-001

既存record value editは、exactなbase snapshotとsource provenanceに対して実行しなければならない（MUST）。編集対象recordをPrimary Key valueだけ、source record orderだけ、またはfilesystem pathから導出したdomain identityだけで特定してはならない（MUST NOT）。

同一logical Tableに複数Data documentが存在し、Build Selection前には同じPrimary Key valueを持つsource recordが共存し得るため、source provenanceは少なくとも選択されたData document内のrecord occurrenceとfield source locationを、そのbase snapshotに対して一意に再特定できなければならない（MUST）。exact internal handle shapeは固定しない。

### SOURCE-EDIT-002

本仕様の初期operationは、既存Data document内の既存record member valueの変更だけを対象としなければならない（MUST）。recordの追加・削除、schema変更、field追加・削除・rename、`$tags`変更、Table identity変更、source file rename/moveをこのoperationへ暗黙に含めてはならない（MUST NOT）。

GUIの初期編集可能field範囲はGUI specificationが所有する。本source-edit contractの対象が既存record memberであることから、将来の別typed editor capabilityを自動的に許可してはならない（MUST NOT）。

### SOURCE-EDIT-003

editor / host / application boundaryを通るscalar edit representationは、対象Primitive valueまたは入力textをlosslessに保持できなければならない（MUST）。特に`long`と`ulong`は、それぞれApproved Primitive Typesが定める全64-bit rangeをroundingなしで往復できなければならず（MUST）、frontendのIEEE-754 `number`等、全rangeをexactに表現できないrepresentationへ強制変換してはならない（MUST NOT）。

wire JSON shape、Rust type、TypeScript type、RPC field名は本仕様で固定しない。

### SOURCE-EDIT-004

record valueのSave可否を、domain validation errorの有無に依存させてはならない（MUST NOT）。現在のSave candidateに対するvalidation diagnosticが存在しても、source location、workspace write authority、lost-update preflight等のsource commit safetyを満たす限り、validationだけを理由にSaveを拒否してはならない（MUST NOT）。

Saveとvalidationは別のoperation resultとして扱わなければならない（MUST）。Save成功はprojectがvalidであることを意味せず、validation failureもそれ自体ではsource write failureを意味しない。

### SOURCE-EDIT-005

Save candidateの生成はsource-preservingかつdeterministicでなければならない（MUST）。同じexact base snapshotと同じordered local edit setからは、同じcandidate source bytesを生成しなければならない（MUST）。

変更不要なsource fileはbyte-for-byte unchangedでなければならない（MUST）。対象fileでも、編集対象valueと無関係なcomments、quote style、indentation、blank lines、mapping / sequence formatting、record order、mapping member order、およびunrelated source textを変更または削除してはならない（MUST NOT）。

編集対象scalar自身のquote / styleは、新しい入力を安全に表現するために必要な場合だけ変更してよい（MAY）。semantic AST全体を通常serializerで全面再出力する方式を通常Save pathとして使用してはならない（MUST NOT）。

### SOURCE-EDIT-006

base snapshotに対して編集対象source locationを安全かつ一意に再特定できない場合、Save candidate生成を失敗させなければならず（MUST）、full-file reserialization、Primary Keyだけによる別record探索、または近似的なtext searchへfallbackして成功扱いしてはならない（MUST NOT）。

このfailureではworkspace sourceを変更してはならない（MUST NOT）。

### SOURCE-EDIT-007

通常Saveのcommit unitは1つのsource data fileでなければならない（MUST）。同じfileのlocal bufferに含まれる複数record / field変更は、1つのSave candidateとしてまとめてcommitしなければならない（MUST）。

1つのfileの通常Saveを理由に別のdirty source fileを暗黙に保存してはならず（MUST NOT）、別fileのcontentを変更してはならない（MUST NOT）。複数fileを保存する`Save All`は、各fileのSOURCE-EDIT-008以降のpreflight / resultを独立に満たす上位workflowとして扱う。

### SOURCE-EDIT-008

通常Saveはworkspace mutation開始直前に、対象source fileのcurrent exact content identityがbase snapshotと一致することを確認しなければならない（MUST）。一致しない場合はConflictとしてsource mutationを開始してはならず（MUST NOT）、stale baseから外部変更を暗黙に上書きしてはならない（MUST NOT）。

exact content identityのmechanismはbytes比較、cryptographic hash、host snapshot token等から選択してよい（MAY）が、mtimeだけを唯一のcontent identityとして固定してはならない（MUST NOT）。

### SOURCE-EDIT-009

Conflictに対するexplicit Overwriteは通常Saveと区別されたauthorizationでなければならない（MUST）。Overwriteを行う前に、applicationはconflict UIが対象としているcurrent external source identityを取得し、その後さらに変更されていないことを確認しなければならない（MUST）。

Overwriteは明示的に対象とされたcurrent external contentをSave candidateで置き換えてよい（MAY）が、未知または再度staleになったexternal stateを黙って上書きしてはならない（MUST NOT）。利用者がOverwrite前に必ずCompare viewを開くことまでは要求しない。Reloadはlocal buffer破棄を伴う別workflowであり、通常Save successとして扱ってはならない（MUST NOT）。

### SOURCE-EDIT-010

Save resultは少なくとも`Success`、`Conflict`、`Failure`、`Outcome Unknown`を観測上区別できなければならない（MUST）。exact API enum名は固定しない。

- `Success`: intended Save candidateのcomplete contentがworkspace sourceとしてcommitされた。
- `Conflict`: SOURCE-EDIT-008またはSOURCE-EDIT-009のlost-update preflightによりmutation前に停止した。
- `Failure`: operationが成功しなかったことが確定しており、applicationがworkspaceのcurrent source identityを再取得できる状態。
- `Outcome Unknown`: write開始後のfailure等により、old/newどちらのcontentがworkspaceに存在するか安全に断定できない。

`Conflict`、`Failure`、`Outcome Unknown`を`Success`として報告してはならない（MUST NOT）。

### SOURCE-EDIT-011

`Success`後は、commitされたsource contentを新しいbase snapshotとして扱える状態にしなければならない（MUST）。`Conflict`または`Failure`ではcallerがlocal bufferを保持してrecoveryできる情報を失ってはならない（MUST NOT）。

`Outcome Unknown`では、同じstale baseを使って自動Save retryまたは自動Overwriteを行ってはならない（MUST NOT）。次のmutation前にworkspace sourceを再取得し、current source identityを確定してからrecovery workflowへ進まなければならない（MUST）。

### SOURCE-EDIT-012

single-file commit implementationは、通常I/O failureによってpartial/truncated candidateを成功状態として公開してはならない（MUST NOT）。hostがatomic replace、staging、temporary write等を利用できる場合は、それらを用いてoldまたはcomplete new contentへ収束させてよい（MAY）。

process crash、OS crash、browser crash、power lossを含むglobal filesystem transaction atomicityは本仕様では保証しない。crash後にworkspace contentがold/newのどちらか安全に判定できない場合は、再open時にactual sourceをauthorityとして再取得する。

### SOURCE-EDIT-013

record SaveはBuild、Publish、Git stage / commit / push、schema Migration、generated C#更新、binary更新を暗黙に開始してはならない（MUST NOT）。Buildは保存済みsourceを入力とする既存Build operationとして別に実行する。

### SOURCE-EDIT-014

source patch derivationとsource commit I/Oの責務は分離できなければならない（MUST）。source location resolution、candidate derivation、validationとのcomposition等のshared application/domain semanticsをTauri frontend、Browser Host、Native Host adapterごとに再実装してはならない（MUST NOT）。

Native filesystem write、Browser workspace write、permission、path safety、exact file identityの取得等は[Runtime hosts](runtime-hosts.md)のhost boundaryに従う。

## 検証ルール

少なくとも次をfocused unit / integration / GUI workflow evidenceで検証する。

- 同一PK valueを持つ別source recordが存在しても、選択したsource occurrenceだけが変更される。
- `long` / `ulong` boundary valueがfrontend/application boundaryでroundingされない。
- 同一fileの複数cell変更は1 candidateへ入り、別fileは変更されない。
- domain-invalidなedited valueでもvalidationだけを理由にSave拒否されない。
- comment、blank line、unrelated quote/indent/order、およびunchanged file bytesが保持される。
- source locationを再特定できない場合にfull serializationへfallbackしない。
- base snapshot後のexternal editを通常SaveがConflictとして拒否し、external bytesを上書きしない。
- explicit Overwrite前にもcurrent external identityを再確認する。
- Success / Conflict / Failure / Outcome Unknownが混同されず、unknown resultでstale automatic retryしない。
- SaveだけでBuild / Publish / Git operationを開始しない。

fixtureを使用する場合、既存fixture sourceを通常GUI/CLI executionで直接書き換えず、temporary workspace copyを使用する。

## 互換性

既存YAML syntax、Table identity、Field semantics、MessagePack key、generated C#、binary formatを変更しない。source-preserving editは既存source textとGit workflowとの互換性を守るための新しいauthoring contractである。

本仕様はsource file pathを新しいdomain identityへ昇格させず、source provenanceとしてのみ使用する。wire/API serialized shapeを固定しないため、このproposal単体ではpublic protocol compatibilityを追加しない。

## 例

次はnon-normativeな概念例である。

```yaml
kind: data
table: item
records:
  # starter item
  - id: 1
    price: 100
    note: 'keep this style'
```

`price`だけを`120`へ変更するSave candidateでは、comment、record/member order、`note`のquote style等はそのまま保持される。base snapshot取得後に外部editorが同じfileを変更していれば、通常SaveはConflictとなり、そのexternal editを暗黙に消さない。

## 未解決事項（Open Questions）

None identified for the initial existing-record save contract. Exact library、patch data structure、wire shape、atomic replace mechanism、debounce interval、diagnostic codeはimplementation / GUI refinement detailとして本仕様では固定しない。

## 非目標

- recordの追加・削除。
- schema / field / key / typeの変更。
- source file rename / move / folder operation。
- multi-file atomic transaction。
- crash / power-loss耐性を保証するjournal format。
- Build / Publish / Git operation。
- spreadsheet range edit、一括paste、Undo/Redo。
- Programmable View / Computed / Annotation columnの保存format。
