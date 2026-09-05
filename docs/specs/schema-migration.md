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
→ parse Migration Command
→ semantic target resolution
→ operation type/precondition check
→ affected logical documents calculation
→ transformed source snapshotをin-memory構築
→ existing canonical parser / validator / type / table / selectionでfull-project再検証
→ deterministic Migration Plan作成
→ execution authorization checks
→ source mutation commit
```

Migration専用の簡易validatorをcanonical validation pathの代わりに使用してはならず
（MUST NOT）、対象fileだけの局所検証で成功扱いしてはならない（MUST NOT）。transformed
project全体を既存のparser、Type System、Table/Key、Build Selection、およびvalidation
semanticsへ戻して検証する。

### MIGRATION-006

`AddField`は、target logical Tableをresolveし、新field declarationを既存のschema
contractに従って検証し、対象Tableの全recordへ新fieldを導入し、transformed projectを
full validationしなければならない（MUST）。新field declarationは少なくともMessagePack
`key`、field `name`、field `type`を持つものとして検証する。既存fieldのkeyを暗黙に
renumberしてはならない（MUST NOT）。GUI等がmax active key + 1を提案しても、それを
semantic allocation ruleとして扱ってはならない。

既存recordへ導入するvalueのinitializer semanticsが確定するまで、Migration engineは
各recordへ非決定的な値を生成して成功扱いしてはならない（MUST NOT）。initializerの必須性、
empty table、nullable、expressionの扱いはOQ-Bに残す。

### MIGRATION-007

`RenameField`は、target logical Tableとfieldをresolveし、schema declarationのsource name、
対象Tableの全record member name、および既存Approved schema structureにおけるresolved
field referenceを更新しなければならない（MUST）。Renameはpresentation/source nameの
変更であり、MessagePack serialization `key`を変更または再割当してはならない（MUST NOT）。

Primary Key field reference、Secondary Key field referenceなど、既存Approved structureの
依存は単なる文字列置換ではなくresolved semantic referenceとして扱う。将来のReference
仕様を先取りして更新してはならない。安全に更新できない依存がある場合は、full validation
でfail closedし、成功扱いしてはならない（MUST NOT）。

### MIGRATION-008

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

複数YAML fileへ跨るMigration commitは、通常のI/O failureについて次のsource-set境界を
満たす方向で設計しなければならない（MUST）。

```text
Success
→ complete NEW migrated source set

ordinary commit error
→ previous complete OLD source set remains usable
```

staging、preflight、rollback、renameなどの具体的mechanismは固定しない。process crash、
OS crash、power lossまで含むglobal filesystem transaction atomicityを保証してはならない
（MUST NOT）。

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

## Operation別の境界

### AddField

`AddField`の対象はlogical Tableであり、YAML file pathではない。新fieldの`key`は既存の
MessagePack serialization keyと衝突してはならず、既存keyの意味を変えてはならない。既存
recordに値が必要なことは確定しているが、その値の供給方法はOQ-Bで未決定である。

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

## Acceptance matrix（future evidence）

この文書はProposedであり、以下はすべてplanned evidenceである。未実施のtestをpass済み
とは扱わない。

| Requirement | Planned evidence | Status |
| --- | --- | --- |
| MIGRATION-001, MIGRATION-011 | generated C#、binary、receiptをauthorityにせず、YAML sourceだけからmigration inputを構成するtest | pending implementation |
| MIGRATION-002, MIGRATION-003 | Add/Rename/Dropだけをv1 semantic operationとして識別し、SQL/text edit grammarを要求しないtest | pending implementation |
| MIGRATION-004 | 同一snapshot・command・optionsから同一plan/resultになるdeterminism test | pending implementation |
| MIGRATION-005, MIGRATION-013 | transformed snapshotを既存parser/type/table/selection/full validationへ戻し、failure時にmutationしないtest | pending implementation |
| MIGRATION-006 | AddFieldが全recordを対象にし、既存keyを変更せず、initializer未確定をsilentに補わないtest | pending implementation |
| MIGRATION-007 | RenameFieldがMessagePack keyを維持し、Primary/Secondary Key参照をsemanticに更新するtest | pending implementation |
| MIGRATION-008 | destructive authorizationなしのDropFieldがmutationせず、依存fieldを黙って削除しないtest | pending implementation |
| MIGRATION-009 | deterministic plan、dry-run no mutation、affected file/record diagnosticsのtest | pending implementation |
| MIGRATION-010 | multi-file ordinary commit failureがprevious complete source setを保持するtest | pending implementation |
| MIGRATION-012 | in-memory semantic engineがnative filesystem、RPC、async runtimeなしで呼び出せるarchitecture/WASM evidence | pending implementation |

## Open Questions / Specification Gaps

### OQ-A: Generate materializationとの境界

`generate`のmaterialization先はCLI仕様側のOQ-Aで管理する。MigrationがC#だけを
`.masterdata/output/csharp/`へ書くことは、canonical C# + binary + receiptのcoherenceを
壊すため、Generate outputの未決定事項が解決するまでMigrationからも類推してはならない。

### OQ-B: AddField initializer

既存recordへ新fieldのvalueを導入するv1 contractが未決定である。constant initializer、
empty tableでの省略、nullableへの暗黙null、expression、型ごとのdefaultのいずれを許可
するかは、既存type/value semanticsだけから一意に定まらない。決定前に実装を開始しては
ならない。

### OQ-C: YAML rewrite fidelity

comments、formatting、key formatting、quote、unmodified regionをどこまでpreserveするか
は未決定である。lossless CST/text edit、semantic-equivalent reserialization、comments
preservation、modified region以外のbyte preservationのいずれを必須にするかを決めるまで、
「ASTをserializeすればよい」または「全体再serializeでよい」と実装方針を確定してはならない。

### OQ-D: Field target identity

`RenameField`と`DropField`が対象fieldを選択するsemantic selectorを、current source
`name`、別のapproved identity、または別の明示的 declarationで表すかは未決定である。
MessagePack `key`をfield identityへ昇格することは`SCHEMA-KEY-001`に反するため、keyを暗黙
selectorとして採用してはならない。public grammarとは別に、target resolution semanticsを
Human reviewで確定する必要がある。

### OQ-E: Commit recovery detail

ordinary I/O failureでold source setを保持する高位invariantは提案するが、rollback failure、
staging directory cleanup、recovery guidance、cross-volume commitの詳細は未決定である。
process/OS crashのglobal atomicityを追加で保証しない境界も維持する。

### OQ-F: Diagnostic and result contract

Migration plan/resultのstructured diagnostic fields、CLI exit code、stdout/stderr、JSON
serializationは未決定である。semantic planが表現すべき情報と、public wire/output schema
を混同しない。

## Non-goals

このproposalは、migration parser、SQL parser、YAML mutation engine、CST導入、CLI parser変更、
GUI/Web filesystem adapter、Native Host/RPC、receipt runtime、external publisher、cache、
Build Profile wiring、Reference runtime、binary inspectorを実装しない。
