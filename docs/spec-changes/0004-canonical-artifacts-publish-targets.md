# 仕様変更: canonical build artifactsとpublish targets modelを承認・適用する（Specification change）

Status: Applied

<!-- Lifecycle: Draft -> Proposed -> Approved -> Applied、またはRejected。Approved canonical
     specificationを変更する前にhuman approvalが必要である。 -->

## Affected Specifications

- [`docs/specs/build-pipeline.md`](../specs/build-pipeline.md)、`BUILD-ARTIFACT-001`から
  `BUILD-ARTIFACT-005`および`PUBLISH-001`から`PUBLISH-010`。
- [`docs/specs/project-layout.md`](../specs/project-layout.md)、`PROJECT-CONFIG-003`および
  `PROJECT-PATH-001`。

## 根拠と分類（Source Evidence and Classification）

基準commit `c7605c10a5112dcfa146aa408ec580e7faa8eb4a` の
`docs/specs/build-pipeline.md` は、project-local canonical artifactとexternal publish targetを分離するDraftとして、
artifact layout、ownership、manifest、stale cleanup、target syntax、およびrelative path baseを整理していた。

このtaskのHuman Approvalは、次のDecisionとConstraintを明示的に承認した。

- `Decision`: MasterData projectはproject-localな`.masterdata/output/`をcanonical artifact rootとして持ち、canonical rootをtool-ownedとする。
- `Decision`: `build`はcanonical artifactを作成し、external destinationへの配置は0..N個の`publish.targets`による別operationとする。
- `Decision`: v1のtarget kindは`csharp`と`binary`とし、relative target pathはMasterData project rootを基準にする。
- `Decision`: C# publishはtarget directory配下の`.masterdata-publish-manifest.json`によるmanifest-based ownershipとする。
- `Constraint`: manifest外のunmanaged file、`.meta`、sibling、parentをsilent deleteまたはoverwriteしてはならない。
- `Constraint`: previous managed setからcurrent canonical C# setにないpathはsuccessful publish時にretireする。

承認対象は、上記のDraftにすでに定義されていた`BUILD-ARTIFACT-001`から`BUILD-ARTIFACT-005`および`PUBLISH-001`から
`PUBLISH-010`である。これはcurrent implementationからの推測ではなく、このtask inputに明示されたHuman Approvalの記録である。

同時に、既存の`docs/specs/project-layout.md`は`Status: Implemented`で、`build.output`、`build.binary_output`、および旧path ruleを
current contractとして記載していた。新しいcanonical configuration contractと矛盾するため、artifact/publish semanticsを
`build-pipeline.md`へ委譲し、project-layoutのconfiguration/path requirementsを新contractへ更新して`Status: Approved`へ戻す必要がある。

legacy configurationの受理期間、warning/error、自動移行、既存artifactの移動・削除、およびbackward compatibility期間は、このapprovalの対象外である。

## 提案する差分（Proposed Delta）

### 1. Build pipeline canonical contract

`docs/specs/build-pipeline.md`の`Status`を`Draft`から`Approved`へ変更し、既存の次のrequirementをcurrent normative contractとして適用する。

- `BUILD-ARTIFACT-001`から`BUILD-ARTIFACT-005`: project-localなcanonical artifact root、tool ownership、v1 layout、coherent artifact set、
  canonical buildとexternal publishの責務分離。
- `PUBLISH-001`から`PUBLISH-003`: 0..Nの`[[publish.targets]]`、v1 kind、project-root-relative path、canonical artifactだけをpublish inputとする境界。
- `PUBLISH-004`から`PUBLISH-008`: C# manifest location/content、managed/unmanaged classification、stale managed file retirement、collision rejection、
  通常のI/O failure時の目標状態。
- `PUBLISH-009`から`PUBLISH-010`: binaryのexplicit file ownership、C# manifestとの分離、Git repository topology非依存。

Requirement IDはrename、reassign、duplicateしない。B8、publish-only trust、legacy migration、partial publish、absolute path、Unity`.meta`連携などの
未確定事項はOpen Questionとして残す。

### 2. Project layout configuration boundary

`docs/specs/project-layout.md`の`Status`を`Implemented`から`Approved`へ変更する。configuration shapeを次のcanonical targetへ更新する。

```toml
[build]
artifact_dir = ".masterdata/output"
cache = ".masterdata/cache"

[[publish.targets]]
kind = "csharp"
path = "../unity/Assets/MasterData/Generated"

[[publish.targets]]
kind = "binary"
path = "../unity/Assets/StreamingAssets/masterdata.bytes"
```

`PROJECT-CONFIG-003`は`build.artifact_dir`と`build.cache`のnon-empty ruleへ更新し、publish targetのshapeは
`build-pipeline.md`の`PUBLISH-*`へrouteする。`PROJECT-PATH-001`はsourceとcanonical artifact pathのproject-root resolutionおよび
canonical artifactのproject-local constraintへ更新し、relative publish targetのbaseを`PUBLISH-002`へrouteする。

project marker、project identity、source discoveryの意味は変更しない。現行parserが旧`build.output`等を受理することはmigration未実施の
implementation gapとして記録し、旧configを新canonical contractと並列のMUSTとして残さない。

### 3. Statusとcanonical application

このdeltaを適用したcanonical documentsは、implementation evidenceがまだないため`Implemented`へ変更しない。`Approved`はHuman Approval済みの
normative contractを表し、implementation、tests、fixtures、migrationの完了を意味しない。

## 互換性（Compatibility）

canonical configuration surfaceは`build.output` / `build.binary_output`から`build.artifact_dir` / `publish.targets`へ移行するため、旧configurationを
入力するcurrent implementationとの間にimplementation gapがある。legacy inputをいつrejectするか、warningを出すか、自動変換するか、既存artifactを
どのように移動・保持・削除するかはこのdeltaで決定しない。

canonical YAML domain semantics、Table/Key semantics、Type System semantics、MessagePack専用field `key` semantics、およびproject identity ruleは
変更しない。external publish targetのfilesystem safety、released binary compatibility、semantic hash、builder cache identityもこのdeltaでは追加しない。

## 受け入れと実装への影響（Acceptance and Implementation Impact）

canonical documentsがApprovedになった後のimplementation taskでは、少なくとも次を確認する。

- default `.masterdata/output/`と`csharp/`配下のC#、root直下の`masterdata.bytes`をproject-local artifactとして扱う。
- canonical buildとpublishを分離し、canonical artifact以外のraw YAMLや任意generated C#をpublish inputにしない。
- C# manifestでprevious successful publishのmanaged pathsを追跡し、current setとの差分でstale managed fileだけをretireする。
- manifest外のregular file、directory、`.meta`、binary targetのparent/siblingをsilent deleteまたはoverwriteしない。
- `[[publish.targets]]`の複数target、v1 kind、project-root-relative pathをconfiguration parser、CLI、application workflowへ接続する。
- legacy migration、publish-only trust metadata、B8、partial publish、`build --publish`は別途承認された仕様またはimplementation taskで扱う。

このspecification changeのcanonical application自体はdocs-onlyであり、Rust、config parser、CLI、Tauri、filesystem publisher、MasterMemory builder、tests、
fixtures、current production behaviorを変更しない。

## 未解決事項（Open Questions）

- publish-only canonical artifactのtrust metadata、build identity、およびmetadata保存場所。
- external publish pathのfilesystem-equivalent path、case equivalence、Unicode normalization、symlink、path containment policy（B8）。
- absolute publish pathの許可、external targetのparent creation、既存destinationの詳細I/O policy。
- 複数publish targetの一部成功時のretry、rollback、再開、operation/exit semantics、およびcross-target atomicity。
- `masterdata build --publish`の提供可否と、build成功・publish失敗のCLI result表現。
- legacy `build.output` / `build.binary_output`と既存artifactのmigration timing、compatibility、diagnostic、自動移行の有無。
- canonical artifact metadata/version、semantic schema hash、builder cache key、released-schema binary compatibilityの独立specification owner。
- Unity`.meta` lifecycleとpublish manifestの連携をMasterData publisherが持つか、Unity importerへ委譲するか。
- generated .NET projectのownership、cache eviction、Unity asset importとの連携。
- source discoveryでsymlinkをfollowまたはignoreするproduct policy。

## レビュー（Review）

このdeltaは、現行Draftのcanonical artifact / publish target modelに対するHuman Approvalを記録し、`build-pipeline.md`と
`project-layout.md`へatomicに適用した。未承認のlegacy migration、B8、publish failure retry、trust metadataをnormative contractへ昇格させていない。

## 承認記録（Approval Record）

このtask inputにおいてHuman maintainerは、`BUILD-ARTIFACT-001`から`BUILD-ARTIFACT-005`および`PUBLISH-001`から`PUBLISH-010`を含む
canonical artifact / publish targets modelを明示的に承認した。このapprovalを根拠としてcanonical documentsを`Approved`へ更新し、本artifactを
承認済みdeltaのaudit recordとして`Status: Applied`にした。
