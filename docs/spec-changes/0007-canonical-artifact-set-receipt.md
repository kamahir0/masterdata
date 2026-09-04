# 仕様変更: canonical artifact-set receiptでpublish-onlyの入力を検証する（Specification change）

Status: Applied

<!-- Lifecycle: Draft -> Proposed -> Approved -> Applied、またはRejected。Approved canonical
     specificationを変更する前にhuman approvalが必要である。 -->

## Affected Specifications

- [`docs/specs/build-pipeline.md`](../specs/build-pipeline.md)、`BUILD-ARTIFACT-002`から
  `BUILD-ARTIFACT-004`および`PUBLISH-003`を補完する`ARTIFACT-SET-001`から
  `ARTIFACT-SET-008`。
- [`docs/specs/project-layout.md`](../specs/project-layout.md)のproject identity、canonical
  artifact location、configuration semanticsは変更しない。
- [`docs/spec-changes/0004-canonical-artifacts-publish-targets.md`](0004-canonical-artifacts-publish-targets.md)、
  [`docs/spec-changes/0006-external-publish-path-safety.md`](0006-external-publish-path-safety.md)の
  canonical build/publish separationとexternal path preflightを前提とする。

このchangeのcanonical requirement ownerは`build-pipeline.md`である。既存の
`BUILD-ARTIFACT-001`から`BUILD-ARTIFACT-005`、`PUBLISH-001`から`PUBLISH-010`、および
`PUBLISH-PATH-001`から`PUBLISH-PATH-010`はrename、reassign、または置換しない。

## 根拠と分類（Source Evidence and Classification）

Approved canonical modelでは、`build`がproject-localな`.masterdata/output/`を作成し、
`publish`がそのartifactを外部targetへ配布する。publish-only operationがcurrent YAMLを再parse
してfreshnessを確認すると、最後に成功したbuild resultを、source変更だけを理由に配布できなくなる。
一方、receiptなしの既存C#とbinaryをpublisherが推測採用すると、coherentなbuild resultであることを
検証できない。

このtask inputでHuman maintainerが次のscopeを明示的に承認した。

- `Decision`: current YAML/sourceがlast successful build後に変更されても、receiptで検証できる最後の
  successful canonical artifact setをpublishしてよい。
- `Constraint`: `publish`はcurrent YAMLのfreshnessをparse、hash、validate、compareせず、implicit
  build、C# generation、.NET builderを開始しない。
- `Decision`: full buildがcanonical C#、canonical binary、artifact-set receiptをwhole-rootの
  coherent setとして公開する。
- `Constraint`: receipt v1は`.masterdata-artifact-set.json`に保存し、`project.id`、v1 format、
  SHA-256、complete C# path/hash set、固定binary path/hashを記録する。
- `Constraint`: receiptはsource of truth、semantic schema hash、builder cache key、released
  compatibility identity、署名またはproducer authenticationではない。
- `Constraint`: receiptがmissing、malformed、unsupported、mismatch、またはartifact driftを示す場合、
  automatic adoption/repairをせず、external destinationを変更する前にrejectする。

これはpublish-only trustに関する既存のSpecification Gapを、integrity/eligibility receiptの
observable contractとして閉じるchangeである。receipt runtime、external publisher、tests、fixtures、
cache、artifact signingはこのchangeでは実装しない。

## 適用した差分（Applied Delta）

### 1. Coherent canonical set

`build-pipeline.md`へ`ARTIFACT-SET-001`を追加し、successful non-dry-run full buildがcanonical
C#、`masterdata.bytes`、`.masterdata-artifact-set.json`を同じcoherent setとして公開することを
定義した。receiptを後付けで更新せず、通常のI/O failureでは旧setとmatching receiptを保持する。
whole-root publicationとcrash/power-loss時のglobal atomicityの境界は既存contractを維持する。

### 2. Receipt v1 shape and deterministic integrity

`ARTIFACT-SET-002`と`ARTIFACT-SET-003`で、reserved filename、`version = 1`、
`hash_algorithm = "sha256"`、`project_id`、C# relative path/hash list、`masterdata.bytes`の
固定path/hashをcanonical contractにした。SHA-256はexact bytesへ適用し、hashはlowercase 64桁hex、
C# pathsは`csharp/`基準のsafeなrelative pathでdeterministic orderかつuniqueとする。timestamp、Git
provenance、checkout path、cwdはreceipt identityに含めない。

### 3. Publish eligibility and source freshness

`ARTIFACT-SET-004`から`ARTIFACT-SET-006`で、publish前にreceipt、project id、actual C# set、各hash、
binary、filesystem entry typeを検証する。missing/malformed/unsupported/tampered/pre-receipt setは
automatic adoptionせず、`masterdata build`をguidanceとしてdestination mutation前にrejectする。
`ARTIFACT-SET-005`により、current YAML/sourceの変更、削除、rename、publish targetの変更だけでは
valid receiptをinvalidにしない。

### 4. Trust boundary and full-build issue rule

`ARTIFACT-SET-007`と`ARTIFACT-SET-008`で、receiptをartifact consistency/drift detectionの
integrity receiptに限定し、署名・attestation・malicious simultaneous rewrite・semantic/cache/released
compatibilityを保証しないことを明記した。complete C#とvalidated binaryを生成したfull buildだけが
receiptを発行・更新でき、dry-runやpipeline途中のoperationはreceiptを変更しない。

receipt validationの後に、既存の`PUBLISH-PATH-001`から`PUBLISH-PATH-010`のall-target preflightを
行う。canonical receiptとexternal C# destinationの`.masterdata-publish-manifest.json`は別metadataで
あり、receiptからdestination ownershipを推測しない。

## 互換性（Compatibility）

これはpublish-only inputの検証境界を追加するchangeである。receipt導入以前に作られた
`.masterdata/output/`にreceiptがない場合、future external publisherはpublish-eligibleとみなさず、
`masterdata build`によるreceipt付きcoherent setの再生成を要求する。compatibility grace period、
automatic migration、既存artifactの推測adoptionは定義しない。

current YAMLの変更やsource YAMLの削除・rename後でも、current project configをloadでき、
`project.id`がreceiptと一致し、artifact bytesとsetが検証できれば最後のreceipt-valid setをpublishできる。
directory basename、absolute path、Git repository、project name/version、publish target path/kindは
receipt identityにしない。

canonical receiptはcanonical YAML、semantic schema hash、builder cache identity、released binary
compatibility、またはcryptographic authenticityを置き換えない。external destinationへcopyするのは
receipt自体ではなく、receiptで検証したC# setと`masterdata.bytes`だけである。

## 受け入れと実装への影響（Acceptance and Implementation Impact）

これはcanonical specificationへのdocs-only applicationであり、以下のevidenceはfuture implementation
testsである。現時点で実装済み・pass済みとは扱わない。

| Requirement | Planned evidence（すべてpending implementation） | Fixture / observation |
| --- | --- | --- |
| ARTIFACT-SET-001 | `full_build_writes_matching_artifact_receipt`; `failed_build_preserves_previous_receipt_and_set` | C#、binary、receiptをwhole-rootで公開し、failure時にmatching setを保持する。 |
| ARTIFACT-SET-002 | `artifact_receipt_has_v1_shape_and_sha256_hashes`; `artifact_receipt_is_deterministic` | v1 fields、SHA-256、deterministic semantic contentを確認する。 |
| ARTIFACT-SET-003 | `artifact_receipt_covers_complete_csharp_and_binary_set`; `artifact_receipt_rejects_unsafe_paths` | C# exact relative set、fixed binary path、safe path、separator、duplicateを確認する。 |
| ARTIFACT-SET-004 | `publish_validates_receipt_before_external_mutation`; `publish_rejects_tampered_artifact_set` | receipt validationがB8 preflightとdestination mutationより前であることを確認する。 |
| ARTIFACT-SET-005 | `publish_accepts_last_receipt_after_current_yaml_change`; `publish_does_not_rebuild_current_sources` | source change/deletionとtarget config changeでreceiptがinvalidにならないことを確認する。 |
| ARTIFACT-SET-006 | `publish_rejects_missing_or_malformed_receipt_without_mutation`; `publish_rejects_legacy_pre_receipt_set` | automatic adoption/repair/migrationをせず、destinationを変更しない。 |
| ARTIFACT-SET-007 | `artifact_receipt_does_not_claim_authenticity_or_cache_identity` | consistency receiptと署名、provenance、cache、released compatibilityを区別する。 |
| ARTIFACT-SET-008 | `dry_run_leaves_receipt_untouched`; `partial_build_does_not_issue_receipt`; `receipt_generation_failure_preserves_previous_set` | full buildだけがreceiptを発行し、failure時にreceiptだけ更新しない。 |

実装時は、receipt validation failureのstructured diagnostic、receiptのpath containment、expected
regular-file type、current project idとの一致、およびexternal destination mutationが発生しない
境界を確認する。`PUBLISH-PATH-001`から`PUBLISH-PATH-010`のimplementationをこのchangeのruntime未実装
部分として先取りしない。

## 未解決事項（Open Questions）

receiptによるpublish eligibility、source freshnessの扱い、receiptの保存場所、v1 format、hash、
project identity、trust scopeはこのchangeで確定したため、Open Questionには残さない。

以下はreceiptとは独立した未解決事項である。

- 複数publish targetの一部成功時のretry、rollback、再開、operation/exit semantics、およびcross-target atomicity。
- `masterdata build --publish`の提供可否とbuild成功・publish失敗のCLI result表現。
- semantic schema hash、builder cache key、released-schema binary compatibilityをreceiptと独立して定義するか。
- Unity`.meta` lifecycle、generated .NET/cache、source discovery symlink policy。
- artifact signing、producer authentication、supply-chain attestation、remote provenanceを将来必要とするか。

## レビュー（Review）

Status `Applied`。Human Approval済みのreceipt contractをcanonical `build-pipeline.md`へ適用し、
`ARTIFACT-SET-001`から`ARTIFACT-SET-008`をstable normative requirementとして記録した。
canonical `build-pipeline.md`のStatusは`Approved`のままとし、receipt runtimeとexternal publish runtimeの
implementation evidenceがないため`Implemented`へ変更していない。

このapplicationではRust、CLI、Tauri、filesystem publisher、receipt reader/writer/verifier、publish
manifest runtime、tests、fixtures、cache、artifact signingを変更していない。

## 承認記録（Approval Record）

このtask inputにおいてHuman maintainerは、current YAML/sourceが変更されてもlast successful
receipt-valid canonical artifact setをpublishできること、publishがimplicit build/freshness revalidationを
行わないこと、およびfull buildがreceipt付きcoherent artifact setを発行することを明示的に承認した。
承認されたreceipt scopeは次のとおりである。

- filenameは`.masterdata-artifact-set.json`。
- v1は`version = 1`、`hash_algorithm = "sha256"`、`project.id`、complete C# paths/hashes、
  `masterdata.bytes` path/hashを持つ。
- receiptはsource of truth、semantic schema hash、builder cache key、released compatibility、
  cryptographic authenticityではない。
- missing、malformed、unsupported、mismatch、またはlegacy pre-receipt setはautomatic adoptionせず、
  external mutation前にrejectする。

このapprovalを根拠としてcanonical `build-pipeline.md`へdeltaを適用し、本artifactをapplication済みdeltaの
audit recordとして`Status: Applied`にした。project-layoutのnormative semanticsは変更していない。
