# Current Objective

## Role

この文書は、現在のdevelopment priorityを記録する唯一のownerである。

この文書はSpecificationではなく、product/domainのobservable semanticsのauthorityでもない。
Approved semanticsはcanonicalな[仕様](specs/README.md)を参照し、implementation realityはcurrent code、tests、
Git historyをfreshに確認する。ここに書かれたpriorityだけを根拠に、未承認のbehavior、CLI grammar、config key、
protocol、file formatを実装してはならない。

## Objective

現在のHuman priorityは、**AddField source commit safety vertical sliceを完成させる**ことである。

AddFieldのsemantic command受理、deterministic Migration Plan / dry-run、source-preserving transformation、patched sourceの
canonical reparse / postcondition verificationまでのvertical sliceは完了した。次はApprovedな[Schema Migration v1仕様](specs/schema-migration.md)のうち、
`MIGRATION-016`のlost-update preflightと`MIGRATION-010`のmulti-file commit / rollback / `Recovery Required` boundaryを、
実際のcanonical YAML filesystem mutationまで含む閉じたwork packageとして実装する。

このobjectiveでは、既に検証済みのexact source snapshotとtransformed sourceをauthority boundaryとして扱い、commit直前にsource inputsが
staleでないことを確認してからmutationを開始する。複数fileへのcommit途中でfailureした場合は、complete NEW、rollback後のcomplete OLD、
rollback failure時の`Recovery Required`を明確に区別し、partial successを成功として扱わない。

staging、backup、journal、temporary file、rename strategy等の具体的mechanismはinternal implementation choiceであり、Approved specificationが
固定していないpublic recovery surface、CLI grammar、serialized transaction formatをこのobjectiveの都合で新しいcontractとして確定してはならない。

## Why now

直前のObjectiveでAddField plan / dry-runはexternal final reviewを通過し、source-preserving patchとsemantic round-tripのcorrectness boundaryが閉じた。
ここからauthoring operationを実用的なsource mutationへ進める際の主要riskは、semantic transformationそのものではなく、plan作成後の外部更新と
multi-file write failureによってcanonical source setをpartial / stale stateへ壊すことである。

そのため`RenameField`や`DropField`へoperation coverageを広げる前に、AddFieldでsafe commit boundaryを実証する。これにより後続Migration operationが
同じcommit safety primitiveを再利用でき、semantic operation追加とfilesystem transaction riskを分離したまま進められる。

## Completion boundary

次のApproved contractを満たすimplementationとevidenceをもってcompletionとする。

- AddField plan / dry-runで検証済みのexact source snapshotとtransformed sourceを、frontend-independentなcommit pathから安全にcommitできる。
- mutation開始直前に、project config、source file set、およびMigration closure / postcondition判断に使用したsource inputsについてstale updateを検出する。
- preflightで不一致を検出した場合はcanonical sourceへintentional mutationを開始せず、stale planとして失敗する。
- preflight成功後に対象source filesへdeterministicなtransformed bytesをcommitし、成功時はcomplete NEW migrated source setだけを残す。
- multi-file commit途中で通常のI/O failureが発生した場合、可能な範囲でrollbackし、rollback成功時はcomplete OLD source setを利用可能な状態へ戻す。
- commit failureに加えてrollbackも失敗した場合は`Recovery Required`として成功扱いせず、それ以上のintentional mutationを停止する。
- `Recovery Required`時は、recoveryに必要なstaged / backup / journal相当の情報を可能な範囲で保持し、affected filesの状態をstructuredに報告できるboundaryを持つ。
- staging、backup、journal、atomic rename等のmechanismはApproved observable contractを満たすinternal choiceとして実装し、mechanism自体をpublic contractへ昇格させない。
- source commit成功後にcanonical build、publish、generated C# / binary / artifact receipt更新を暗黙に開始しない。
- filesystem mutation / transaction coordinationをpure semantic transformation / planningから分離し、Migration semanticsをstorage layerやfrontendへ複製しない。
- stale preflight、successful multi-file commit、write failure + rollback success、rollback failure + `Recovery Required`をfocused regression / fault-injection evidenceで固定する。
- required checksとexternal final `review-code`で、data safety、spec conformance、rationale freshness、architecture boundaryにBlockingがないことを確認する。
- Approved authorityから決められないobservable recovery behaviorやpublic surfaceが必要になった場合は、implementation convenienceで補完せずSpecification Gapとして停止する。

## Explicit non-scope

このobjectiveは、次を今回のpriorityに含めない。

- `RenameField` implementation
- `DropField` implementationとdestructive execution authorization
- concrete `masterdata migrate ...` CLI grammar、SQL-like language、CLI wiring
- Migration Command / Plan / Result / recovery stateのpublic JSON schema、stdout / stderr、exit code contract
- Tauri / GUI / Web / AI adapterへのMigration wiring
- crash / OS crash / power lossまで含むglobal filesystem transaction atomicityの保証
- public recovery command、backup directory layout、journal file format等の新しいobservable contract
- formatter operation、record standard formatting order、`$tags` placement、schema formatter
- implicit build、publish、generated C# / binary / artifact receiptの更新
- ChangeFieldType、record-level UPDATE / INSERT / DELETE、binary mutation / query
- new Requirement ID、new public Diagnostic Code、またはApproved Migration semanticsの変更
- unrelated refactor、GUI design-system刷新、table / record editor全体の実装

## Next candidate

次のHuman priority候補は、**RenameField plan / dry-run vertical slice**である。

AddFieldでsemantic planningとsource commit safetyの両boundaryが閉じた後、既存のMigration engine / commit safety primitiveを再利用して
`RenameField`のlogical Table / current field name resolution、resolved dependency更新、MessagePack key維持、source-preserving rewrite、
semantic round-tripまでを別work packageとして閉じることを候補とする。

その後に`DropField`とdestructive execution authorizationへ進むことを候補とするが、この順序は自動昇格ではなく、各Objective完了時に
current implementation realityとproduct priorityをfreshに確認してHumanが選定する。

## Relevant authorities

- [Product vision](product/vision.md) — local-first authoring systemとshared semanticsの方向性
- [Specification index](specs/README.md) — specification lifecycleとnormative authority
- [Schema Migration v1 specification](specs/schema-migration.md) — `MIGRATION-001`〜`MIGRATION-017`、特に`MIGRATION-005`、`MIGRATION-009`〜`MIGRATION-016`
- [YAML subset specification](specs/yaml-subset.md) — canonical YAML source semantics
- [Runtime hosts specification](specs/runtime-hosts.md) — pure/shared semantic engineとhost adapter boundary
- [CLI surface specification](specs/cli.md) — `migrate` top-level Operation nameと未確定CLI grammarのboundary
- [Applied CLI / Schema Migration specification change](spec-changes/0011-cli-surface-and-schema-migration.md) — Human Approval済みdeltaとdeferred implementation/public decisions
- current AddField Migration Plan / transformed source / exact source retention implementationとtests — reuseすべきimplementation reality
- current project loading / filesystem boundaryとfile provenance implementation — stale preflight / commit integrationでfreshに確認すべきimplementation reality
- [Specification workflow](contributing/specification-workflow.md) — Specification Gapとapproval lifecycle
