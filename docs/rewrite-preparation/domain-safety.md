# Domain / Safety Invariants案

Status: Draft

rewriteの内部APIを指定しない最小契約。詳細のcurrent authorityは各ownerへのlinkに残す。分類・provenanceは [registry](classification.md)。本書は [適用境界](README.md) に従い、canonicalの代替全文ではない。

## Sourceと意味論

| Invariant | なぜ必要か / rewriteの自由度 | Current owner |
| --- | --- | --- |
| YAML / textual sourceがcanonical | cacheや独自databaseだけで再現できる状態にすると、人のdiff / mergeと外部編集を失う。read modelは派生物 | [source edit](../specs/source-edit.md), [ADR0001](../adr/0001-yaml-is-source-of-truth.md) |
| schema-directed interpretation、shared Rust semantics | scalar lexical formをfrontendが推測するとnullable / 64-bit / custom / referenceの意味が分裂する。parser / AST / DTO名は自由 | [ADR0007](../adr/0007-source-values-and-schema-interpretation.md), [types](../specs/type-system/README.md) |
| source preservation | 対象外bytes、comments、quotes、CRLF等の要求を守る。no-op / edit reversionは対応oracleでbyte比較。locationが曖昧なら失敗し、whole-document serializerへ逃げない | [source edit](../specs/source-edit.md), [field mutation](../specs/field-declaration-mutation.md) |
| semantic invalidity ≠ Save不可 | 編集途中のschema mismatchやduplicate keyを失わず保存できる。構文・安全に表現できないmutationは別。Buildはstrictな意味検証が必要 | [source edit](../specs/source-edit.md), [Build](../specs/build-pipeline.md) |
| lossless integer boundary | int64 / uint64をJS Numberへ丸めない。wire表現はstring固定ではなく、端から端までexactであること | [primitives](../specs/type-system/primitives.md), [data editor](../gui/data-editor/spec.md) |
| occurrence identity ≠ PK value | duplicate / changed PK、同値Array itemでも狙ったoccurrenceだけを変更する。location mechanismやID型は自由 | [record mutation](../specs/source-record-mutation.md), [source edit](../specs/source-edit.md) |
| inline / separate / mixed / emptyを等価に扱う | storage topologyを編集workflowに押しつけず、physical source安全性を維持する。`records`不在と空配列は同一source表現ではない | [Tables](../specs/table-and-keys.md), [0037](../spec-changes/0037-inline-table-records.md) |

YAMLの許容syntaxは [yaml-subset](../specs/yaml-subset.md) を参照する。ただしflow mappingはApprovedとtestが衝突しており [D6](human-decisions.md#d6--yaml-flow-mapping) の解決まで新しいacceptance oracleにしない。禁止syntaxを黙って広げることも、現在受理されるsourceを互換性調査なしに拒否することも避ける。

Table/type/key/referenceのnominal identity、modifier legality、nullableとdefaultの区別、Reference Required/Nullableとunique/non-uniqueの振る舞いはdomain contractとして残す。exact generated C# public shape、MessagePack key、reference helperはconsumer compatibilityでもある。[table/key](../specs/table-and-keys.md)、[reference](../specs/index-and-reference.md)、[C# naming](../specs/type-system/csharp-naming.md) の期待結果を移植し、内部module名と一緒に捨てない。

## Local authoringとread lifetime

- workspace base readとdirty overlayを別所有にする。navigation / diagnostics / dependency refreshはdirty、schema draft、Added Rows、Pending delete、history、query stateを破棄・保存・他sourceへ転用しない。
- Undo/Redoはsource-local authoring history。diskへのSaveをUndoして過去bytesへ戻す一般的filesystem rollbackではない。input編集中のnative Undoとの優先順位も保つ。
- row orderはsource上のpresentation、column orderはdeclaration上のpresentation、Array orderは値の意味。order変更だけでPK / Reference / MessagePack key / Buildのcanonical順序を変えない。
- clean external changeは新generationへrefreshする。dirty external changeはlocal draftを保ちConflictにする。missing / invalid source、missing dependency、changed bindingから古いeditable viewを返さない。
- diagnosticsには対象generation / pending状態が必要。old validation resultを新sourceのcurrent診断にしない。shapeに必要な解決とproject-wide diagnostics completionを分離できる。
- historyはSave success等のcanonical base更新で当該sourceだけclearする。Failure / Conflict / Unknownでは保持する。memory都合のevictionは事前通知しcurrent bufferを保持する。exact capacityは固定しない。

Current owners: [grid authoring](../gui/data-editor/grid-authoring.md)、[Table editor](../gui/table-editor/spec.md)、[source edit](../specs/source-edit.md)、[ADR0008](../adr/0008-interactive-workspace-read-session.md)。具体的latencyとwork countは [performance](performance.md) が所有する。

## Authoritative writes

| Boundary | 精製する契約 / failure mode |
| --- | --- |
| Fresh preflight | Save / Migration / structural operationはactual current source identityとexpected identityをfreshに比較する。read cache一致をcommit authorizationにしない。mtime一致だけでも不十分 |
| Conflict | changed sourceでsilent overwriteしない。local stateを保持し、明示reload / compare / 許可されたoverwriteへ進む。overwriteにも確認したexternal identityとのfresh比較が必要 |
| Config | config identityとbinding変更を別途確認する。sourceで許可されるoverwriteをconfigへ拡張しない。project/shared、local tooling、user UIの永続scopeを混同しない。Desktop user preferenceはOS per-user application領域をauthorityとし、legacy WebView値は移行inputである |
| Current Table Save | dirty schema（inline recordsとのcompositionを含む）とselected record sourceが対象。inactive record sources / 他Tableを含めない。既知Conflictは対象全体のpreflightでcommit前に止める |
| Save All | 他dirty sourceを明示的に対象とする別操作。fileごとの結果と残ったlocal bufferを正確に表示する。Save scopeを「新architectureが簡単だから」で変えない |
| Partial / uncertain outcome | commit開始後のSuccess / Conflict / Failure / Outcome Unknown / NotAttemptedを区別する。Unknownを自動retryして二重適用しない。成功fileだけbaseを進め、他draftを保持する |
| Recovery Required | migration rollback等でOLD / NEW / mixed状態を判別できない場合、canonical gateに従いwrites / Buildを止める。単にdialogを閉じて解除しない。read navigationは利用できる |
| Path / creation | source creationはexclusive、rename/moveもsource / destinationをfresh確認。configured root、logical / physical identity、symlink escape、case aliasを守る。cached pathで別fileへSaveしない |

理由は、外部editor / Git操作 / concurrent write / filesystem failureが通常のdesktop環境で起こるため。これらは特定transaction classやglobal Mutexを要求しない。全fileのpower-loss atomicity、general recovery database、automatic repairを追加保証しない。

Owners: [source edit](../specs/source-edit.md)、[0044 Save/history](../spec-changes/0044-table-authoring-save-history.md)、[config](../specs/project-config-edit.md)、[source paths](../specs/source-path-mutation.md)、[schema migration](../specs/schema-migration.md)、[type migration](../specs/type-migration.md)。

## Migration / Build / Publish

Migrationはexplicit authorizationとreview可能なPlanを持つ。対象closureのactual input identity / config / source membershipが変われば古いPlanを適用しない。unsupported mutationを暗黙coercionやtext search/replaceで実行しない。ordinary direct authoringにPlan / Apply surfaceを要求する理由にはしない。

Buildはsaved source + captured config/profileからdeterministicなC# / MasterMemory binary / receiptのcoherent setを作る。失敗時に以前の正常setを部分更新しない。sourceのpresentation順ではなくcanonical key順で構築する。GUI / RustがMasterMemory binary internalsを再実装せず、native .NET / MasterMemoryへdelegationする。exact bridge executable / JSON DTOは内部手段。

Publishはlast successful artifact receiptを配布する。current YAMLのdirty / invalid状態をBuild済みartifactの不適格条件にしない。BuildとPublishを暗黙連鎖しない。全destinationのpreflight、managed / unmanaged境界、critical namespace / symlink安全性、partial target failureを守る。Unityの`.meta` / GUIDはUnity ecosystemの所有である。

Owners: [migration](../specs/schema-migration.md)、[Build](../specs/build-pipeline.md)、[selection](../specs/build-selection.md)、[request preview](../specs/build-request-preview.md)、[Unity](../specs/unity-integration.md)、[ADR0003](../adr/0003-dotnet-mastermemory-bridge.md)。CLI command / generated C# / Unity public API / source configはpublic compatibilityとして [corpus](compatibility-corpus.md) で扱う。
