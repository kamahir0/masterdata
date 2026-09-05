# Schema Migration v1仕様

Status: Proposed

この文書は、YAMLをcanonical source of truthとするMasterData projectに対する、
schema-aware deterministic migrationのsemantic contractを提案する。Human Approval前の
ため、実装authorityではない。SQL-like public language、specific CLI grammar、YAML rewrite
library、filesystem transaction mechanismはこの文書では確定しない。

CLI surfaceとのcompositionは[CLI surface仕様](cli.md)、既存のYAML/schema/type/table
semanticは、[YAML subset仕様](yaml-subset.md)、[Table / Key仕様](table-and-keys.md)、
[Type System仕様](type-system/README.md)および各owner specificationを参照する。

## 用語と境界

### Migration

Migrationは、validated project snapshotに対して、table・fieldなどのsemantic intentを
適用し、project-wideにdeterministicなsource transformationを行うOperationである。単純な
文字列置換、pathごとの局所編集、generated artifactの変更とは定義しない。

### Migration Command / semantic AST

Migration CommandはMigration Operationを表すsemantic入力である。今回、具体的なRust
struct、serialized JSON、CLI subcommand grammarをpublic contractとして固定しない。ただし
semantic modelは、`AddField`、`RenameField`、`DropField`を識別でき、logical Table identity、
fieldのsemantic selectorまたはdeclaration、operationの引数を表現できなければならない
（MUST）。physical YAML pathをTable identityとして扱ってはならない（MUST NOT）。

### Source snapshot / transformed snapshot

Source snapshotは、Nativeまたは将来のBrowser Hostが取得したcanonical YAML documentsと
provenanceを含む入力snapshotである。Transformed snapshotは、Migration Commandを適用した
in-memoryの次段入力であり、YAMLがcanonical source of truthであることを置き換えない。

### Plan / execution authorization

Planはmutation前に算出可能なdeterministicな影響とvalidation結果の説明である。Execution
authorizationは、特にdestructive operationをcommitしてよいかを表す実行時の許可であり、
Migration Commandのsemantic intentそのものとは分離する。

## Normative Requirements

### MIGRATION-001

Migrationのauthorityはcanonical YAML sourceと、そこから構成されたvalidated project
snapshotでなければならない（MUST）。既存generated C#、canonical binary、artifact-set
receipt、またはbinary inspection resultをMigrationのsource authorityとして使用しては
ならない（MUST NOT）。file pathはprovenance/storage locationであり、logical Table
identityではない。

### MIGRATION-002

v1で識別可能なMigration Operationは、`AddField`、`RenameField`、`DropField`に限定する。
`ChangeFieldType`、`SELECT`、`INSERT`、`UPDATE`、`DELETE`、SQL-like text language、binary
query、binary mutationはv1 scope外であり、Migration v1の成功operationとして扱っては
ならない（MUST NOT）。

### MIGRATION-003

Migration Command semantic modelは、text edit命令ではなく、logical Table identity、field
semantic selectorまたはdeclaration、およびsemantic operationを表さなければならない
（MUST）。public SQL grammar、`migrate add-field ...`の具体的なCLI grammar、または特定の
Rust struct layoutは、このproposalでは固定しない。Command ASTを第二のsource of truthや
serialized domain schemaに昇格させてはならない（MUST NOT）。

### MIGRATION-004

同じvalidated source snapshot、同じMigration Command、および同じexecution optionsから
生成されるtransformed semantic resultとMigration Planは、観測可能なsemantic内容に
ついてdeterministicでなければならない（MUST）。recordごとの非決定的なAI生成、暗黙の
field key再採番、環境依存のpath順序によって結果を変えてはならない（MUST NOT）。

### MIGRATION-005

Migrationは、少なくとも次のsemantic flowを経なければならない（MUST）。

```text
resolve project/workspace snapshot
→ receive / validate Migration Command semantic input
→ current exact source snapshotをcanonical parse / semantic resolution
→ semantic target resolution / operation precondition check
→ affected logical documents calculation
→ Migration Commandをexpected transformed semantic snapshotへ適用
→ deterministic source patch plan作成
→ patchをin-memory sourceへ適用
→ patched sourceをcanonical parserで再parse / semantic resolution
→ expected transformed semantic resultとの整合確認
→ existing canonical validator / type / table / selectionでfull-project再検証
→ deterministic Migration Plan作成
→ execution authorization checks
→ commit直前のlost-update preflight
→ source mutation commit
```

ここで`receive / validate Migration Command semantic input`は、frontend-independentな
semantic入力を受け取り、そのshapeとoperation argumentsを検証することを表す。CLI text
syntax parserをsemantic coreが必須とする意味ではない。CLI、GUI、Web、AI adapterは、それぞれ
の入力をこのsemantic modelへ変換してよいが、specific CLI grammar、SQL-like grammar、
serialized AST shapeはこのspecで固定しない。

Migration専用の簡易validatorをcanonical validation pathの代わりに使用してはならず
（MUST NOT）、対象fileだけの局所検証で成功扱いしてはならない（MUST NOT）。transformed
project全体を既存のparser、Type System、Table/Key、Build Selection、およびvalidation
semanticsへ戻して検証する。text patchが適用できただけではMigration成功と扱ってはならず
（MUST NOT）、patched sourceを再parseしてexpected transformed semantic resultと確認する。

### MIGRATION-006

`AddField`は、target logical Tableをresolveし、新field declarationを既存のschema
contractに従って検証し、対象Tableの全recordへ新fieldを導入し、transformed projectを
full validationしなければならない（MUST）。新field declarationは少なくともMessagePack
`key`、field `name`、field `type`を持つものとして検証する。既存fieldのkeyを暗黙に
renumberしてはならない（MUST NOT）。GUI等がmax active key + 1を提案しても、それを
semantic allocation ruleとして扱ってはならない。

対象Tableのexisting recordsが1件以上ある場合、`AddField`はexplicit initializerを要求
しなければならない（MUST）。v1のinitializerはconstant valueだけであり、既存のcanonical
YAML scalar、sequence、およびtype semanticsで型検査する。例えば`0`、`""`、`null`、`[]`
は、対象field typeが許可する場合にだけ有効である。nullable fieldへ`null`を導入する場合も
explicit `null` initializerを要求し、arrayへempty valueを導入する場合もexplicit `[]`
initializerを要求する。

v1ではexpression、other-field reference、function call、recordごとのAI-generated value、
implicit language/runtime default、implicit `null`をinitializerとして許可してはならない
（MUST NOT）。existing recordsが0件の場合はinitializerを省略してよい（MAY）。initializerを
指定した場合は、recordsの有無にかかわらず同じcanonical type/value semanticsで検証する。

### MIGRATION-007

v1の`RenameField` target resolutionは、logical Table identityと対象snapshot上のcurrent
field nameの組で行わなければならない（MUST）。MessagePack `key`をfield identityへ昇格
してはならず（MUST NOT）、新しいstable Field IDを導入してはならない（MUST NOT）。Tableが
存在しない、fieldが存在しない、またはsemantic resolutionできない場合はprecondition
failureとしてsource mutationを開始してはならない。Rename後の新nameはtarget selectorでは
なくoperation argumentである。physical YAML pathはtarget identityではない。

`RenameField`は、解決されたtargetについてschema declarationのsource name、
対象Tableの全record member name、および既存Approved schema structureにおけるresolved
field referenceを更新しなければならない（MUST）。Renameはpresentation/source nameの
変更であり、MessagePack serialization `key`を変更または再割当してはならない（MUST NOT）。

Primary Key field reference、Secondary Key field referenceなど、既存Approved structureの
依存は単なる文字列置換ではなくresolved semantic referenceとして扱う。将来のReference
仕様を先取りして更新してはならない。安全に更新できない依存がある場合は、full validation
でfail closedし、成功扱いしてはならない（MUST NOT）。

### MIGRATION-008

v1の`DropField` target resolutionも、logical Table identityと対象snapshot上のcurrent
field nameの組で行う。対象Table、field、またはそのsemantic resolutionが存在しない場合
はprecondition failureとして扱い、source mutationを開始してはならない（MUST NOT）。
`DropField`はschema field declarationを削除し、対象Tableの全recordから該当valueを削除
するdestructive operationである。commit mutationには、explicitでmachine-actionableな
destructive execution authorizationが必要であり、interactive promptだけに依存しては
ならない（MUST NOT）。authorizationがない場合、plan/preflightは`destructive = true`
を報告してよいが、source mutationを開始してはならない（MUST NOT）。

`--allow-destructive`はCLIでの候補表現に過ぎず、Command ASTのintentとexecution authorization
を同一conceptとして固定しない。Primary Key、Secondary Key、または既存のindex等がdrop
対象fieldに依存する場合、関連構造を黙って削除して成功扱いしてはならない（MUST NOT）。
整合するtransformed projectを生成できない場合はfail closedする。

### MIGRATION-009

Migrationはmutation前にdeterministicなPlanを生成可能でなければならない（MUST）。Planは
少なくとも、operation、target table、target fieldまたはdeclaration、destructiveかどうか、
affected source files、affected record count、validation resultまたはdiagnosticsを
conceptually表現できるものとする。dry-runまたはplan-only実行はcanonical YAML sourceを
mutationしてはならない（MUST NOT）。console formatting、JSON field名、`--json` schemaは
このproposalで固定しない。

### MIGRATION-010

複数YAML fileへ跨るMigration commitは、通常のI/O failureについて次のobservable
source-set statesを区別しなければならない（MUST）。

```text
Success
→ complete NEW migrated source set

commit failure + rollback success
→ complete OLD source set remains usable

commit failure + rollback failure
→ Recovery Required
```

`Recovery Required`の場合、successとして報告してはならず（MUST NOT）、それ以上のintentional
mutationを停止しなければならない（MUST）。recoveryに必要なstaged、backup、journal相当
の情報は可能な範囲で保持し、affected filesの状態をstructuredに報告できる方向とする。
automatic silent continuationを行ってはならない（MUST NOT）。

backup layout、journal format、staging directory、recovery CLI commandなどの具体的mechanism
は固定しない。process crash、OS crash、power lossまで含むglobal filesystem transaction
atomicityを保証してはならない（MUST NOT）。

### MIGRATION-011

Migrationは、canonical artifactの生成またはpublishを暗黙に開始してはならない（MUST NOT）。
source mutationが成功した後のcanonical buildは、既存build Operationとして別に実行する。
Migrationは既存canonical binary、generated C#、artifact receiptを更新または修復する
Operationではない。binaryはgenerated artifactであり、v1 Migrationのquery/mutation authority
ではない。

### MIGRATION-012

Migrationのpure semantic transformationは、loaded source snapshotとsemantic入力だけで
実行可能な形を保ち、filesystem commitやworkspace writeはhost-dependent boundaryとして
分離できなければならない（MUST）。Browser filesystem、RPC、Native Host endpoint、async
runtimeをMigration semantic coreへ必須依存として導入してはならない（MUST NOT）。CLI、GUI、
Web、AIが将来同じdeterministic engineを利用できる構造を目標とするが、各host adapterや
protocolを今回実装・固定しない。

### MIGRATION-013

Migrationは、current source snapshotの全体semantic validationを通過し、planと必要な
execution authorizationが揃う前にsource mutationを開始してはならない（MUST NOT）。
authorization failure、target resolution failure、precondition failure、full validation
failure、plan生成failureは、source setを変更せずにstructured diagnosticsとして返す方向
とする。

### MIGRATION-014

MigrationによるYAML source mutationは、source-preservingかつdeterministicでなければならない
（MUST）。同じexact source snapshot、同じMigration Command semantic input、および同じ
execution optionsからは、同じtransformed source bytesを生成しなければならない（MUST）。
これは、同じsemantic stateからglobally canonicalizedな同一YAML bytesを要求するものではない。

Migrationにより変更不要なsource fileはbyte-for-byte unchangedでなければならない（MUST）。
affected fileでも、migrationに不要なpresentation informationを保持しなければならない
（MUST）。少なくともcomments、quote style、indentation、blank lines、unrelated mapping /
sequence formatting、およびunrelated source textをmigrationの副作用で変更または削除しては
ならない。Migrationに必要なsource spansだけを変更する方向とする。

semantic AST全体を通常serializerで全面再出力する方式は、v1 Migrationの通常成功pathに
使用してはならない（MUST NOT）。CST library、lossless parser、rope、text patch engine、
source span representationなどのmechanismは固定しない。

### MIGRATION-015

source-preserving rewriteであっても、text patchが適用できただけではMigration成功と扱って
はならない（MUST NOT）。実行は、current exact source snapshotをcanonical parse / semantic
resolutionし、Migration Commandをexpected transformed semantic snapshotへ適用し、
deterministic source patch planを生成し、patchをin-memory sourceへ適用した後、patched
sourceをcanonical parserで再parseしなければならない（MUST）。再parse後のsemantic resultは
expected transformed semantic resultと整合し、さらにfull-project validationを通過しなければ
ならない。内部のsnapshot、patch、comparison representationは固定しない。

### MIGRATION-016

Migrationはsnapshot取得後に他のHuman、AI、またはprocessが行った変更を古いsnapshotで
上書きしてはならない（MUST NOT）。source commit開始直前に、少なくともaffected source
files全体について、current source bytesまたは同等のfile identityがMigration Planのbase
となったexact source snapshotと一致することをpreflightしなければならない（MUST）。

1つでも不一致がある場合、source mutationを開始してはならず（MUST NOT）、concurrent
modificationまたはstale planとしてstructured diagnosticで報告できる方向とする。mtimeだけ
をsemantic identityとして固定せず、exact bytes、hash、file identityなどのmechanismは後続
実装に残す。

## Operation別の境界

### AddField

`AddField`の対象はlogical Tableであり、YAML file pathではない。新fieldの`key`は既存の
MessagePack serialization keyと衝突してはならず、既存keyの意味を変えてはならない。既存
recordがある場合は、MIGRATION-006に従うexplicit constant initializerが必要である。

### RenameField

`RenameField`はsource-level field nameと、同じfieldを参照する既存Approved structureを
semanticに更新する。field nameの出現箇所を全ファイルで機械的に置換してよい、という意味
ではない。MessagePack key、Table identity、source file path、generated C# identifierの
意味を混同しない。

### DropField

`DropField`はdestructive authorizationを要求する。Planはauthorizationがない状態でも
影響範囲とdestructive性を示してよいが、authorizationなしのcommitを許可しない。fieldが
index等の整合性に必要な場合は、別の暗黙削除で穴埋めせず、full validation failureとして
扱う。

## Binaryとartifact authority

Binary query/inspectおよびbinary mutationはv1 scope外である。将来read-only binary
inspectorを作る場合でも、YAML project Migration/query engineと内部実装を共通化することを
このspecは要求しない。canonical artifact set receiptはpublish eligibilityとbyte integrity
を扱う既存contractであり、Migration input authorityではない。

## Refined v1 decisions

このProposed refinementで、次のv1 observable semanticsを提案として具体化する。

- existing recordsが1件以上の`AddField`にはexplicit constant initializerを要求する。
- existing recordsが0件の場合、`AddField` initializerは省略できる。initializerを指定した
  場合はcanonical type/value semanticsで検証する。
- `RenameField` / `DropField`のtargetはlogical Table identityとcurrent field nameで解決
  し、MessagePack `key`や新しいField IDをidentityにしない。
- YAML rewriteはsource-preservingかつdeterministicで、不要なpresentation informationを
  変更せず、affected filesだけを必要なsource spansで更新する。
- semantic transformed resultとpatched sourceをcanonical parserで突き合わせ、patch成功
  だけではMigration成功にしない。
- commit直前にaffected source filesのlost updateを検出し、stale planならmutationを開始
  しない。
- ordinary commit failureは、rollback成功ならOLD、rollback failureなら`Recovery Required`
  として報告する。

これらはまだHuman Approval前であり、`Status: Proposed`のcontract候補である。

## Acceptance matrix（future evidence）

この文書はProposedであり、以下はすべてplanned evidenceである。未実施のtestをpass済み
とは扱わない。

| Requirement | Planned evidence | Status |
| --- | --- | --- |
| MIGRATION-001, MIGRATION-011 | generated C#、binary、receiptをauthorityにせず、YAML sourceだけからmigration inputを構成するtest | pending implementation |
| MIGRATION-002, MIGRATION-003 | Add/Rename/Dropだけをv1 semantic operationとして識別し、SQL/text edit grammarを要求しないtest | pending implementation |
| MIGRATION-004 | 同一snapshot・command・optionsから同一plan/resultになるdeterminism test | pending implementation |
| MIGRATION-005, MIGRATION-013 | transformed snapshotを既存parser/type/table/selection/full validationへ戻し、failure時にmutationしないtest | pending implementation |
| MIGRATION-006 | AddFieldが全recordを対象にし、既存keyを変更せず、record存在時のexplicit constant initializerを要求するtest | pending implementation |
| MIGRATION-007 | RenameFieldがMessagePack keyを維持し、Primary/Secondary Key参照をsemanticに更新するtest | pending implementation |
| MIGRATION-008 | destructive authorizationなしのDropFieldがmutationせず、依存fieldを黙って削除しないtest | pending implementation |
| MIGRATION-009 | deterministic plan、dry-run no mutation、affected file/record diagnosticsのtest | pending implementation |
| MIGRATION-010 | multi-file commitがOLD / NEW / Recovery Requiredを正しく報告するtest | pending implementation |
| MIGRATION-012 | in-memory semantic engineがnative filesystem、RPC、async runtimeなしで呼び出せるarchitecture/WASM evidence | pending implementation |
| MIGRATION-014 | unaffected fileのbyte preservation、affected fileのpresentation preservation、deterministic source bytesのtest | pending implementation |
| MIGRATION-015 | patched sourceの再parseとexpected transformed semantic resultの整合確認test | pending implementation |
| MIGRATION-016 | commit直前のconcurrent modification / stale planを検出しmutationしないtest | pending implementation |

## Open Questions / Specification Gaps

### OQ-A: Generate materializationとの境界

`generate`のmaterialization先はCLI仕様側のOQ-Aで管理する。MigrationがC#だけを
`.masterdata/output/csharp/`へ書くことは、canonical C# + binary + receiptのcoherenceを
壊すため、Generate outputの未決定事項が解決するまでMigrationからも類推してはならない。
`.masterdata/generated/csharp`などのuser-facing recommended project directory layoutも
このproposalでは決めない。

### OQ-B: Concrete public argument grammar

`migrate`の具体的なargument grammar、CLI syntax、GUI/Web/AI adapterからsemantic inputへ
変換するsurfaceは未決定である。MIGRATION-003とCLI-009のsemantic boundaryは固定するが、
SQL-like languageやserialized ASTを選択しない。

### OQ-C: Concrete source patch mechanism

CST library、lossless parser、rope、text patch engine、source span representationなどの
具体的なimplementation mechanismは未決定である。MIGRATION-014/015のobservable contractを
満たす方法は、後続のimplementation designで選択する。

### OQ-D: Concrete staging / journal / recovery mechanism

backup layout、journal format、staging directory、rollback implementation、Recovery Required
からのrecovery CLI commandは未決定である。MIGRATION-010のobservable state、silent
continuation禁止、crash/power-lossへの非保証境界はこのproposalで固定する。

### OQ-E: Recommended project directory layout

user-facing MasterData projectのrecommended directory layoutは後続の独立specificationで
設計する。`.masterdata/generated/`、`sources/schemas/`、`sources/data/`、`sources/types/`
等を、このproposalからcanonicalまたはrecommended layoutとして導出してはならない。既存の
project root、`build.artifact_dir`、`build.cache`のApproved contractは変更しない。

### OQ-F: Diagnostic and result contract

Migration plan/resultのstructured diagnostic fields、CLI exit code、stdout/stderr、JSON
serializationは未決定である。semantic planが表現すべき情報と、public wire/output schema
を混同しない。

## Non-goals

このproposalは、migration parser、SQL parser、YAML mutation engine、CST導入、CLI parser変更、
GUI/Web filesystem adapter、Native Host/RPC、receipt runtime、external publisher、cache、
Build Profile wiring、Reference runtime、binary inspectorを実装しない。
