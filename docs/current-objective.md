# Current Objective

## Role

この文書は、現在のdevelopment priorityを記録する唯一のownerである。

この文書はSpecificationではなく、product/domainのobservable semanticsのauthorityでもない。
Approved semanticsはcanonicalな[仕様](specs/README.md)を参照し、implementation realityはcurrent code、tests、
Git historyをfreshに確認する。ここに書かれたpriorityだけを根拠に、未承認のbehavior、CLI grammar、config key、
protocol、file formatを実装してはならない。

## Objective

現在のHuman priorityは、**Schema Migration AddField plan / dry-run vertical sliceを完成させる**ことである。

GUI validationとGUI canonical buildのvertical sliceが完了し、既存のvalidation / build capabilityはNative product surfaceから利用できる状態になった。
次にauthoring systemのfoundationへ進むため、Approvedな[Schema Migration v1仕様](specs/schema-migration.md)のうち、`AddField`について
semantic command受理からdeterministic Migration Plan / dry-runまでのfrontend-independentなmigration engineを最初の閉じたsliceとして実装する。

このobjectiveではcanonical YAML sourceをauthorityとし、logical Table identityでtargetをresolveし、既存のYAML / Type System / Table semanticsを再利用する。
source-preserving patchをin-memoryへ適用した後、patched sourceをcanonical parserで再parseし、expected transformed semantic resultと
AddField固有のpostconditionを確認する。text patchが適用できただけではcompletionと扱わない。

specific CLI grammar、serialized Migration AST / JSON schema、YAML rewrite library、filesystem transaction mechanismなど、Approved specificationが
意図的に固定していないpublic surfaceまたはimplementation mechanismを、このobjectiveの都合で新しいcontractとして確定してはならない。

## Why now

GUI canonical build vertical sliceまでで、projectを開く、validateする、canonical artifactsをbuildする主要Native workflowがshared application semanticsを
再利用する形で閉じた。次は既存capabilityのGUI露出を増やすより、source authoringを安全に行うためのshared semantic foundationへ進む。

Schema Migration v1は`AddField`、`RenameField`、`DropField`のsemantic contract、deterministic plan、source-preserving rewrite、patched sourceの
semantic round-trip、safe commit boundaryをApprovedとして定義している。一方、Migration全体を一度に実装するとsource transformationとfilesystem
transaction / recoveryの異なるriskを同じwork packageへ混在させる。

最初のsliceを`AddField`のplan / dry-runまでに限定することで、Migrationの中心となるsemantic resolution、canonical constant validation、
source-preserving transformation、postcondition verification、deterministic planningを、filesystem mutationなしで先に実証できる。

## Completion boundary

次のApproved contractを満たすimplementationとevidenceをもってcompletionとする。

- frontend-independentなinternal Migration Commandとして`AddField`のsemantic intentを表現し、logical Table identityと新field declarationを受け取れる。
- target Table、対象schema、対象Tableへ寄与するdata documents、およびinitializer検証に必要なtype / semantic closureを安全かつ決定論的にresolveする。
- 新fieldのMessagePack `key`、`name`、`type`、modifierとinitializerを既存Approved YAML / Type System / Table semanticsで検証する。既存recordが1件以上ある場合はexplicit initializerを要求する。
- 既存field keyを暗黙にrenumberせず、新field declarationをschema `fields` sequence末尾へappendするexpected transformed semantic stateを構成する。
- 対象Tableの全recordへ同一initializer由来のcanonical valueを追加し、既存record mapping memberをreorderせず末尾へappendするsource patchを生成する。
- Migrationに不要なsource presentationを保持し、unaffected fileはbyte-for-byte unchangedとする。affected fileもcomments、quote style、indentation、blank lines、unrelated textを不要に変更しない。
- deterministic source patch planをin-memory sourceへ適用し、patched sourceを既存canonical parser / semantic resolutionで再parseする。
- 再parse後のsemantic resultがexpected transformed semantic resultと一致し、AddField operation-specific postconditionを満たすことを確認する。
- deterministic Migration Planとして、少なくともoperation、target table、新field declaration、`destructive = false`、affected source files、affected record count、validation result / diagnosticsをconceptually表現できる。
- plan / dry-run実行ではcanonical YAML sourceをfilesystem上でmutationしない。
- Project全体がerror-freeであることをsuccess gateにせず、必要なMigration resolution closureを構成でき、unrelated diagnosticを安全に分類できる場合はそれだけでrejectしない。一方、targetとの関係を判定できないunclassifiable sourceはfail closedする。
- pure semantic transformation / planningをfilesystem、RPC、Tauri、CLI frontendから分離し、将来の複数hostが同じengineを利用できるboundaryを保つ。
- Approved authorityから決められないobservable behaviorが必要になった場合は、implementation convenienceで補完せずSpecification Gapとして停止する。

## Explicit non-scope

このobjectiveは、次を今回のpriorityに含めない。

- canonical YAML sourceへのfilesystem commit
- commit直前のlost-update preflight
- staging、backup、journal、rollback、`Recovery Required`を含むmulti-file transaction / recovery implementation
- `RenameField` implementation
- `DropField` implementationとdestructive execution authorization
- concrete `masterdata migrate ...` CLI grammar、SQL-like language、CLI wiring
- Migration Command / Plan / Resultのpublic JSON schema、stdout / stderr、exit code contract
- Tauri / GUI / Web / AI adapterへのMigration wiring
- formatter operation、record standard formatting order、`$tags` placement、schema formatter
- implicit build、publish、generated C# / binary / artifact receiptの更新
- ChangeFieldType、record-level UPDATE / INSERT / DELETE、binary mutation / query
- new Requirement ID、new public Diagnostic Code、またはApproved Migration semanticsの変更
- source-preserving mechanismとしてCST / lossless parser / rope / text patch libraryのいずれかをpublic contractとして固定すること
- unrelated refactor、GUI design-system刷新、table / record editor全体の実装

## Next candidate

次のHuman priority候補は、**AddField source commit safety vertical slice**である。

今回のplan / dry-run sliceが完了した後、MIGRATION-016のlost-update preflightとMIGRATION-010のmulti-file commit / rollback / `Recovery Required`
boundaryを、実際のfilesystem mutationを含む別work packageとして閉じることを優先候補とする。具体的なstaging、backup、journal mechanismは
Approved observable contractを満たすinternal implementation choiceとして選び、未承認なpublic recovery surfaceを発明しない。

その後に`RenameField`、`DropField`へsemantic operation coverageを拡張することを候補とするが、この順序は自動昇格ではなく、各Objective完了時に
current implementation realityとproduct priorityをfreshに確認してHumanが選定する。

## Relevant authorities

- [Product vision](product/vision.md) — local-first authoring systemとshared semanticsの方向性
- [Specification index](specs/README.md) — specification lifecycleとnormative authority
- [Schema Migration v1 specification](specs/schema-migration.md) — `MIGRATION-001`〜`MIGRATION-017`、特に`MIGRATION-003`〜`MIGRATION-006`、`MIGRATION-009`、`MIGRATION-012`、`MIGRATION-014`、`MIGRATION-015`、`MIGRATION-017`
- [YAML subset specification](specs/yaml-subset.md) — canonical YAML syntax / scalar semantics
- [Table / Primary Key / Secondary Key specification](specs/table-and-keys.md) — Table / field / MessagePack key / record mapping semantics
- [Type System specifications](specs/type-system/README.md) — initializerとfield typeのcanonical semantic owner
- [Runtime hosts specification](specs/runtime-hosts.md) — pure/shared semantic engineとhost adapter boundary
- [CLI surface specification](specs/cli.md) — `migrate` top-level Operation nameと未確定CLI grammarのboundary
- [Applied CLI / Schema Migration specification change](spec-changes/0011-cli-and-schema-migration.md) — Human Approval済みdeltaとdeferred implementation/public decisions
- current project/document parser、semantic resolution、validation implementation / tests — reuseすべきimplementation reality
- current `LoadedDocument` exact source retentionとsource-loading boundary — source-preserving patch planningのimplementation reality
- [Specification workflow](contributing/specification-workflow.md) — Specification Gapとapproval lifecycle
