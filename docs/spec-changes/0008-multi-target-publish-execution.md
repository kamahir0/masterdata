# 仕様変更: 複数publish targetのexecution / failure semanticsを定義する（Specification change）

Status: Applied

<!-- Lifecycle: Draft -> Proposed -> Approved -> Applied、またはRejected。Approved canonical
     specificationを変更する前にhuman approvalが必要である。 -->

## Affected Specifications

- [`docs/specs/build-pipeline.md`](../specs/build-pipeline.md)へ、既存の`PUBLISH-001`から
  `PUBLISH-010`および`PUBLISH-PATH-001`から`PUBLISH-PATH-010`を補完する
  `PUBLISH-EXEC-001`から`PUBLISH-EXEC-005`を追加する。
- [`docs/spec-changes/0006-external-publish-path-safety.md`](0006-external-publish-path-safety.md)の
  `PUBLISH-PATH-010` all-target preflightを、execution phaseの境界へ接続する。
- [`docs/spec-changes/0007-canonical-artifact-set-receipt.md`](0007-canonical-artifact-set-receipt.md)の
  receipt validationをPhase 1の入力境界として使用する。receipt formatとtrust scopeは変更しない。

このchangeのcanonical requirement ownerは`docs/specs/build-pipeline.md`である。既存の
`PUBLISH-001`から`PUBLISH-010`、`PUBLISH-PATH-001`から`PUBLISH-PATH-010`、および
`ARTIFACT-SET-001`から`ARTIFACT-SET-008`はrename、reassign、または置換しない。

## 根拠と分類（Source Evidence and Classification）

Approved canonical modelは、receiptで検証した1つのcanonical artifact setを0..N個のexternal
publish targetへ配布する。`PUBLISH-PATH-010`は全targetのpreflightとmutation前の停止を定義するが、
preflight成功後のtarget failure、後続targetのcontinuation、partial successのoverall result、
およびtarget-local rollbackはOpen Questionに残っていた。

このtask inputでHuman maintainerは、次のdecisionを明示的に承認した。

- `Decision`: receipt validation後に全targetをpreflightし、1件でもinvalidならtargetを1つもmutationしない。
- `Decision`: 全target preflight成功後は、execution-time failureが起きても残りのindependent targetをattemptする。
- `Decision`: failureはfailed target自身のfailure domainに閉じ、previous usable stateを保持またはrollbackする。
- `Decision`: 後続targetのfailureだけを理由に、先にsuccessしたtargetをcross-target rollbackしない。
- `Decision`: 1件以上のexecution failureがあれば、partial successであってもoverall publishは`Err`になる。
- `Decision`: targetごとの`succeeded` / `failed`をstructuredに報告し、preflight failureでは必要に応じて`not_attempted`を表現する。
- `Constraint`: `publish.targets`のregistration orderはsuccess dependencyを表さず、targetの前後関係によってfailure後のskipを導入しない。
- `Decision`: partial success後の再実行は、current destination stateとtarget-local manifestからsafeに収束する方向とする。
- `Decision`: valid receiptと0 targetはsuccessful no-opとする。

これはpartial publishのobservable execution contractを閉じるchangeである。Rust、CLI publish command、
receipt runtime、external publisher、filesystem mutation、Tauri、GUI、tests、fixtures、cache、artifact
signingはこのchangeでは実装しない。

## 適用した差分（Applied Delta）

### 1. Three-phase execution boundary

canonical `build-pipeline.md`へ、publishを次の3 phaseで扱うモデルを追加した。

```text
Phase 1: canonical artifact-set receipt validation
    -> invalid: Err, no destination mutation
Phase 2: ALL configured target preflight
    -> any invalid: Err, no target attempted or mutated
Phase 3: target execution
    -> every configured target is attempted
    -> target-local success or failure/rollback
    -> per-target results are aggregated
```

Phase 1のownerは`ARTIFACT-SET-004`から`ARTIFACT-SET-006`、Phase 2のownerは
`PUBLISH-PATH-001`から`PUBLISH-PATH-010`である。Phase 3の追加contractは、stable Requirement IDを持つ
`PUBLISH-EXEC-001`から`PUBLISH-EXEC-005`である。

### 2. PUBLISH-EXEC-001 — all-target preflight / no mutation

receipt validation成功後、全configured targetをpreflightする。collision、protected path overlap、
symlink violation、manifest/ownership ambiguity、unexpected type、target graph overlapなどが1件でも
あればoverall `Err`とし、target parent作成、C# manifest更新、stale deletion、binary replacementを
含むmutationを開始しない。preflight failureではexecution phaseを開始せず、targetごとのstatusは
必要に応じて`not_attempted`を表現できる。

### 3. PUBLISH-EXEC-002 — all-target attempt / continuation

all-target preflightが成功した後は、configured targetを全てattemptする。先行targetのexecution-time
failureを理由に後続targetをskipしない。target orderはdeterministicなattempt/report orderに使用
できるが、registration orderはsuccess dependencyではない。全targetは同じreceipt-valid canonical
artifact setを入力とし、targetごとのimplicit build、YAML再parse、再validation、C# generation、または
.NET buildを行わない。preflight後のTOCTOU failureはそのtargetのexecution failureとして扱い、後続を
continueする。

### 4. PUBLISH-EXEC-003 — target-local failure safety

各targetを独立failure domainとする。C# targetはsuccess時にcurrent managed setとmanifestをcoherentに
公開し、failure時にはprevious managed setをusableに保ち、unmanaged contentを変更しない。binary target
はsuccess時にconfigured explicit fileへNEW binaryを公開し、既存regular fileのreplace failureでは
previous fileをusableに保つ。initial binary targetが不存在の場合、failure後にpartial/corruptなtarget
pathを公開しない。target-local rollbackまたはprevious stateの保持に失敗した場合はstructured
diagnosticを返し、success扱いしない。

### 5. PUBLISH-EXEC-004 — no cross-target rollback

target Aがsuccessした後にtarget Bがfailureしても、Bのfailureだけを理由にAをrollbackしない。複数
filesystem、repository、volumeを跨ぐglobal atomic commit、cross-target transaction coordinator、
2-phase commitはv1 contractとして保証しない。AはNEW usable stateを保持したまま、overall operationは
failureになり得る。

### 6. PUBLISH-EXEC-005 — aggregate result / retry convergence

全targetのattempt完了後にresultをaggregateする。全targetがsuccessならoverall `Ok`、1件以上の
execution/rollback failureがあればpartial successでもoverall `Err`とする。resultは少なくともtarget
ごとの`succeeded` / `failed`をstructuredに識別し、preflight failureの`not_attempted`を必要に応じて
表現できる。execution開始後、先行failureを理由に後続を`not_attempted`としてはならない。

valid receiptと0 targetはsuccessful no-opとしてoverall `Ok`にする。partial success後のretryはhidden
transaction logを前提とせず、current destination stateとtarget-local manifestを再評価して同じ
receipt-valid artifact setへ安全に収束する方向とする。unchanged fileをskipするかなどのperformance
optimizationは固定しない。

### 7. Existing requirementとの接続

`PUBLISH-008`はC# target-local failureでprevious managed setとunmanaged contentを保持するcontract
として`PUBLISH-EXEC-003`と接続した。`PUBLISH-009`はbinary explicit fileのprevious usable stateと
initial failure時のpartial target禁止を`PUBLISH-EXEC-003`で補完した。`PUBLISH-PATH-010`のall-target
preflightは`PUBLISH-EXEC-001`のPhase 2であり、`ARTIFACT-SET-004`のreceipt validationはPhase 1である。

## 互換性（Compatibility）

このchangeは、既存のC# manifest-based ownership、binary explicit-file ownership、B8のtarget graph
disjointness、receipt validation、および0..N target syntaxを維持したまま、preflight後のexecution
semanticsを追加する。preflight failureは従来どおり全target no-mutationであり、execution-time partial
failureのみtarget-local successを残しながらoverall `Err`となる。

target registration orderはdependencyではないため、既存configurationに暗黙のsuccess dependencyを
追加しない。successful targetのNEW stateを後続failureで戻さないため、複数destinationのglobal atomic
commitを想定するconsumerは追加できない。

retryは前回operationのhidden transaction logやsource freshnessに依存せず、同じreceiptとcurrent
destination/manifestから再評価できる。publishはcurrent YAMLを再parseせず、`masterdata build --publish`
の追加、receipt trustの変更、artifact signing、semantic schema hash/cache、Unity`.meta` policyはこの
changeのcompatibility contractに含めない。

## 受け入れと実装への影響（Acceptance and Implementation Impact）

これはcanonical specificationへのdocs-only applicationであり、以下のevidenceはfuture implementation
testsである。現時点で実装済み・pass済みとは扱わない。

| Requirement | Planned evidence（すべてpending implementation） | Fixture / observation |
| --- | --- | --- |
| PUBLISH-EXEC-001 | `preflight_failure_mutates_no_targets` | receipt validation後の全target preflightで1件でも失敗した場合、parent、manifest、binaryを変更しない。 |
| PUBLISH-EXEC-002 | `execution_failure_continues_to_later_targets`; `toctou_failure_on_one_target_does_not_skip_others` | execution phase開始後は先行failureで後続targetをskipせず、全targetをattemptする。 |
| PUBLISH-EXEC-003 | `execution_failure_rolls_back_only_failed_target`; `binary_failure_preserves_previous_file`; `initial_binary_failure_does_not_publish_partial_file`; `csharp_failure_preserves_previous_managed_set` | C# managed/unmanaged境界、binary explicit file、initial failure、rollback failureの安全性を確認する。 |
| PUBLISH-EXEC-004 | `successful_target_is_not_rolled_back_by_later_failure` | 後続failureだけを理由に先行success targetをrollbackせず、global atomicityを主張しない。 |
| PUBLISH-EXEC-005 | `publish_returns_error_after_partial_success`; `publish_reports_per_target_status`; `retry_after_partial_success_converges`; `zero_targets_is_successful_noop` | aggregate `Ok`/`Err`、per-target status、再実行収束、0 target no-opを確認する。 |

## 未解決事項（Open Questions）

複数targetのall-target preflight、target-local failure/rollback、continuation、cross-target rollback禁止、
aggregate `Err`、per-target result、retry convergence、registration orderの非依存性、および0 target no-opは
このchangeで確定したため、Open Questionには残さない。

以下は引き続きこのchangeのscope外である。

- `masterdata build --publish`を提供するか。提供する場合、canonical build成功とpublish失敗をCLI resultへどう表すか。
- semantic schema hash、builder cache key、released-schema binary compatibilityをreceiptと独立して定義するか。
- Unity`.meta` lifecycle、generated .NET/cache、source discovery symlink policy。
- artifact signing、producer authentication、supply-chain attestation、remote provenanceを将来必要とするか。
- crash/power-loss時のcross-target global atomicityは保証しない。OS-level durabilityとrecovery UXはこのchangeの実装contractに含めない。

## レビュー（Review）

Status `Applied`。Human Approval済みのpartial publish execution contractをcanonical
`build-pipeline.md`へ適用し、`PUBLISH-EXEC-001`から`PUBLISH-EXEC-005`をstable normative requirement
として記録した。canonical `build-pipeline.md`のStatusは`Approved`のままとし、external publisher、
receipt runtime、target executionのimplementation evidenceがないため`Implemented`へ変更していない。

このapplicationではRust、CLI publish command、Tauri、filesystem publisher、receipt runtime、tests、
fixtures、cache、artifact signingを変更していない。

## 承認記録（Approval Record）

このtask inputにおいてHuman maintainerは、次のscopeを明示的に承認した。

- receipt validation後のall-target preflight failureは、全target no-mutationのoverall `Err`である。
- all-target preflight成功後は、execution-time failureが起きても残りのindependent targetをattemptする。
- failureはfailed target自身のtarget-local failure domainで保持またはrollbackし、rollback failureはsuccessにしない。
- 先にsuccessしたtargetを、後続target failureだけを理由にcross-target rollbackしない。
- 1件以上のtarget failureがあればpartial successでもoverall `Err`とする。
- per-target `succeeded` / `failed`をstructuredに報告し、preflight failureでは`not_attempted`を表現できる。
- `publish.targets`のregistration orderはsuccess dependencyではない。
- partial success後のretryはcurrent destination stateとtarget-local manifestから安全に収束し、valid receiptと0 targetはsuccessful no-opとする。

このapprovalを根拠としてcanonical `build-pipeline.md`へdeltaを適用し、本artifactをapplication済みdeltaの
audit recordとして`Status: Applied`にした。partial publish executionは未実装であり、project-layoutの
normative semantics、receipt format、B8 path-safety semanticsは変更していない。
