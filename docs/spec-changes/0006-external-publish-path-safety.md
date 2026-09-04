# 仕様変更: external publish targetのfilesystem path safetyを定義する（Specification change）

Status: Proposed

<!-- Lifecycle: Draft -> Proposed -> Approved -> Applied、またはRejected。canonical
     specificationを変更する前にhuman approvalが必要である。 -->

## Affected Specifications

- [`docs/specs/build-pipeline.md`](../specs/build-pipeline.md)、既存の`PUBLISH-001`から
  `PUBLISH-010`を補完する`PUBLISH-PATH-001`から`PUBLISH-PATH-010`。
- [`docs/specs/project-layout.md`](../specs/project-layout.md)、`PROJECT-PATH-001`の
  relative publish target baseとの整合確認。project layoutのcanonical ownerは変更しない。
- [`docs/adr/0004-file-location-has-no-semantic-meaning.md`](../adr/0004-file-location-has-no-semantic-meaning.md)。
  file locationをdomain identityと解釈しない境界を維持する。

このproposalのcanonical requirement ownerは`build-pipeline.md`である。既存の
`PUBLISH-001`から`PUBLISH-010`、`BUILD-ARTIFACT-001`から`BUILD-ARTIFACT-005`、および
`PROJECT-PATH-001`の意味をrename、reassign、または直接変更しない。`PUBLISH-PATH-*`は、
external publish implementationの前に承認されるべきpath-safety補助requirementの候補である。

## 根拠と分類（Source Evidence and Classification）

現行のApproved contractは、canonical artifactと0..N個のexternal publish targetを分離し、
C# targetではmanifest-based ownership、binary targetではexplicit file ownershipを採用している。
一方、external destinationのfilesystem-equivalent path、case equivalence、Unicode、symlink、
path containment、parent creation、およびtarget間のcollisionは、
[`build-pipeline.md`](../specs/build-pipeline.md)のOpen Questions（B8）に残っている。

このproposalのstatementは次のように分類する。

- `Constraint`: destination filesystemが同じentry/objectまたは重複namespaceとして扱うpathを、
  lexical spellingだけで別artifactとして扱ってはならない。
- `Constraint`: collision、ownership不明、unexpected filesystem typeを検出した場合、
  destination mutationより前にrejectしなければならない。
- `Constraint`: unmanaged content、manifest ownership、binary explicit-file ownership、
  canonical/source/cache/configの保護境界を混同してはならない。
- `Proposal`: v1ではpublish target rootまたはancestorのsymlink traversalをrejectする。
- `Proposal`: v1では異なるpublish targetのownership regionのoverlapをrejectする。
- `Proposal`: absolute external target pathを許可する。
- `Decision（既承認）`: relative target pathのbaseはMasterData project root、target syntaxは
  `[[publish.targets]]`、v1 kindは`csharp`と`binary`、C# ownershipはmanifest-based、
  binary ownershipはexplicit fileである。

このtask inputはB8をrefineするevidenceであり、Human Approval operationではない。canonical
Approved document、Rust、CLI、Tauri、filesystem publisher、test fixture、cache、および
production behaviorは変更しない。

## 提案する差分（Proposed Delta）

### PUBLISH-PATH-001 — destination filesystem namespace

publish targetのpath比較は、文字列のlexical equalityではなく、targetが存在するdestination
filesystemのnamespaceとentry/object identityを基準にしなければならない（MUST）。filesystemが
同じentry/objectまたは重複namespaceとして扱うpathは、同一pathまたはcollisionとして扱わなければ
ならない（MUST）。既存pathのcanonicalized prefix、filesystem identity、symlink alias、
case-sensitiveまたはcase-insensitiveなdirectory lookup、およびfilesystemが同一entryとして扱う
Unicode spellingをこの判定に含める。

このruleは、少なくとも次の比較へ同じ意味で適用する。

- current generated C# pathとexisting unmanaged entry
- manifest pathとdestination entry、manifest path同士、およびreserved manifest path
- binary targetとcurrent generated C#、manifest、existing unmanaged entry
- target同士のownership region
- publish targetとproject、`masterdata.toml`、source root、canonical artifact root、cache

publisherは`path.to_lowercase()`、ASCII-only normalization、NFCだけ、またはOS名だけを、全
filesystemに通用するequivalence algorithmとして仕様化してはならない（MUST NOT）。同一entryか
どうかをdestination filesystemから確認できる場合、その結果を優先する。確認できない場合の
fail-closedは`PUBLISH-PATH-002`に従う。

### PUBLISH-PATH-002 — missing tailとfail-closed

まだ存在しないpublish targetまたはgenerated pathを、存在しないという理由だけでunsafeと判定しては
ならない（MUST NOT）。publisherは、必要に応じてlongest existing ancestor/prefixをdestination
filesystem上で解決し、残りのmissing tailを保持したpath namespaceとしてcollisionとcontainmentを
判定しなければならない（MUST）。このmechanical strategyはcanonical specificationで特定の
Rust crateまたはOS APIに固定しない。

existing prefix、missing tail、case behavior、symlink、またはnamespaceの安全性を証明できない
場合、publisherはstructured path/config errorでrejectしなければならず（MUST）、destinationへ
mutationを開始してはならない（MUST）。通常のsafeな未存在targetは、検証済みのparent creation
policyに従ってsupportしてよい（MAY）。

### PUBLISH-PATH-003 — C# target rootとsymlink traversal

このproposalのv1 candidateでは、C# target rootまたはtarget rootへ到達する既存ancestor component
がsymlinkである場合、publishを開始してはならない（MUST NOT）。publisherはconfigured target
のspellingからsymlink先のtreeを自動的にownershipしたと推測してはならない。

C# target rootが存在しない場合は、`PUBLISH-PATH-002`および`PUBLISH-PATH-009`のpreflight後に
必要なdirectoryを作成してよい。existing target rootはreal directoryでなければならず（MUST）、
file、symlink、またはspecial filesystem objectの場合はrejectしなければならない（MUST）。
target directory全体をdeleteして再作成するownership strategyは導入してはならない（MUST NOT）。

target内のunmanaged symlinkはuser-owned contentとして扱い、publisherはfollow、delete、または
symlink先のoverwriteをしてはならない（MUST NOT）。current generated pathまたはmanaged pathが
そのsymlinkとfilesystem上でcollisionする場合はrejectする。

### PUBLISH-PATH-004 — C# managed/unmanaged path safety

C# targetのmanaged pathは`PUBLISH-004`のmanifestに記録されたrelative file pathとreserved
manifest fileに限る。manifest外のregular file、directory、`.meta`、およびsymlinkはunmanaged
contentとして保持し、current generated pathがfilesystem上でそのentryとcollisionする場合は
rejectしなければならない（MUST）。publisherはunmanaged entryをautomatic adoption、rename、
suffix付与、delete、またはoverwriteによって解決してはならない（MUST NOT）。

previous manifestに記録されたmanaged pathをretireまたはupdateする場合、destinationの対象は
expectedなregular fileでなければならない（MUST）。managed pathの現在entryがdirectory、symlink、
またはspecial filesystem objectへ置換されている場合、manifest記載だけを根拠に強制delete、
recursive removal、symlink followをしてはならず、ownership mismatchとしてrejectする。

manifest pathとcurrent generated pathがcaseまたはUnicodeを含む異なるspellingでも、destination
filesystemが同じentryとして扱うなら、同じmanaged slotまたはcollisionとして扱う。文字列だけで
stale removalとadditionを別pathだと判断してはならない。

### PUBLISH-PATH-005 — manifestのambiguityとreserved path

manifestが存在しない初回publishではprevious managed setを空として扱う、という`PUBLISH-004`の
ruleを維持する。既存manifestが次のいずれかに該当する場合、publisherはownershipを確定できない
ためpublishをrejectしなければならない（MUST）。

- JSON shape、format version、relative path、path containmentが不正である。
- 未対応のmanifest versionである。
- `files`にfilesystem上同一entryへresolveするduplicate aliasがある。
- manifest自身がregular fileではなく、symlink、directory、またはspecial objectである。

malformedまたはambiguous manifestを削除、修復、merge、再採用してはならない（MUST NOT）。reject
時はstale managed fileを削除せず、unmanaged contentとmanifestを変更してはならない（MUST NOT）。
`.masterdata-publish-manifest.json`はpublisher-reserved pathであり、current generated C# path、
binary target、または別のreserved artifactがdestination filesystem上でこのpathとcollisionする
場合はrejectする。

### PUBLISH-PATH-006 — binary explicit-file safety

`kind = "binary"` targetのpublisher ownershipはconfigured explicit fileそのものに限る。target
entryが存在しない場合は、safeなexisting ancestorのpreflight後に作成してよい（MAY）。targetが
existing regular fileの場合はcanonical binaryでreplaceしてよい（MAY）。targetがdirectory、
symlink、またはspecial filesystem objectの場合、publisherはrejectしなければならない（MUST）。
targetまでの既存ancestor componentがsymlinkの場合も、v1 candidateではrejectする。

binary publisherはC# manifestをownership metadataとして使用してはならず（MUST NOT）、parent
directory、sibling file、隣接する`.meta`、または他のdirectory entryを削除・rename・overwriteして
はならない（MUST NOT）。既存regular fileをreplaceできることは、そのparentやsiblingのownershipを
意味しない。

### PUBLISH-PATH-007 — MasterData critical pathsの保護

publish targetは、destination filesystem上で次のMasterData critical pathと、publisherがその
protected regionを管理・置換できるequalまたはancestor/descendant overlapをしてはならない（MUST）。

- MasterData project rootおよび`masterdata.toml`
- configured source rootsとそのtree
- canonical artifact rootとそのtree
- build cacheとそのtree

この判定には`PUBLISH-PATH-001`のfilesystem equivalenceを適用する。project rootそのもの、または
project rootを含む外側のdirectoryをtargetにする場合はrejectするが、project root内の独立した
`dist/`のようなsafeなdirectoryまで一律に禁止しない。source root、canonical artifact root、cache
については、targetがそのregionの内側・外側のどちらからでもregionを管理・置換できる場合を
rejectする。`masterdata.toml`自身、source YAML、canonical C#、canonical binary、またはcacheを
binary explicit targetにすることも許可しない。

### PUBLISH-PATH-008 — target graphのdisjointness

複数publish targetはindependentなownership regionとして扱う。target pathがfilesystem上で
equivalent、ancestor/descendant、またはその他のnamespace overlapとなるconfigurationは、
target mutation開始前にrejectしなければならない（MUST）。少なくとも次を含む。

- 同じbinary fileを指すduplicate targetまたはcase/Unicode alias
- C# target内にbinary targetがある構成
- C# target同士のnested overlap
- C# targetとbinary targetが同じentryまたはnamespaceを共有する構成

このproposalのv1 candidateでは、target kindが同じか異なるかにかかわらずownership regionの
overlapを許可しない。target graphのcollisionをoperation orderで解決したり、一方のtargetを
automaticにsubdirectory ownershipへ変更したりしてはならない（MUST NOT）。

### PUBLISH-PATH-009 — path baseとparent creation

relative publish target pathは引き続きMasterData project rootを基準にresolveしなければならない
（MUST）。process current working directory、source root、checkout directory basenameをbaseに
してはならない。absolute pathを許可するかどうかはこのrequirementではまだ確定せず、許可された
場合にも`PUBLISH-PATH-001`から`PUBLISH-PATH-008`のsafety checkを省略してはならない（MUST NOT）。

target rootまたはbinary targetのmissing parent directoryは、既存prefix、symlink、protected
path、target graphのpreflightが成功した後に限り作成してよい（MAY）。新規directoryの作成は、
そのparentやsiblingsのownershipをpublisherへ移さない。preflight前にparentを作成してはならない
（MUST NOT）。

### PUBLISH-PATH-010 — preflight、mutation境界、TOCTOU

既知のpath collision、ownership ambiguity、unexpected filesystem type、protected path overlap、
symlink policy違反、およびtarget graph collisionは、いずれかのpublish destinationをmutation
する前に検出してrejectしなければならない（MUST）。0..N targetを全てpreflightしてから、必要な
parent creation、C# file operation、manifest update、binary replacementを開始する。

preflight後にfilesystemが変更された場合、publisherは各mutation時点でunexpected symlink/type
changeを安全側にrejectし、危険なfollow、recursive delete、またはpartial overwriteを行っては
ならない（MUST NOT）。このruleは外部processやuserとのraceをOS-level handleで完全排除すること、
crash/power-loss時のcross-target atomicity、またはpartial publishのretry/rollback semanticsを
自動的に保証するものではない。通常のI/O failureの目標状態は既存の`PUBLISH-008`に従う。

## Filesystem pathの例

以下はpath文字列の比較結果ではなく、実際のdestination filesystemが返すnamespace/identity結果に
基づいて判定する。

- case-insensitiveなfilesystemで`Generated/Item.g.cs`と`generated/item.g.cs`が同じentryになる
  場合、C# generated pathとunmanaged entry、またはtarget同士のcollisionとしてrejectする。case-
  sensitiveなfilesystemで別entryになる場合にまで一律rejectしない。
- `Café.g.cs`と`Café.g.cs`がdestination filesystem上で同一entryになる場合はcollisionとしてreject
  する。filesystemが別entryとして扱う場合、MasterDataが文字列をNFC等へ変換して別意図を作っては
  ならない。
- `../unity/NewGenerated`のようにtarget自体が未存在でも、既存ancestorが安全に解決でき、missing
  tailのnamespaceにcollisionがない場合は、preflight後のparent creationを許可できる。
- target rootまたは既存ancestorがsymlinkである場合、v1 candidateではsymlink先をtarget namespace
  と推測せずrejectする。

## 推奨する初期値とHuman review項目

以下はこのproposalのrecommended initial valueであり、Human Approval前の未確定選択である。

| 項目 | 推奨（Recommended） | 代替案 | trade-off |
| --- | --- | --- | --- |
| A. absolute publish path | v1でallowする | v1ではrejectする | separate repositoryやCI workspaceを相対pathへ固定せずに扱える一方、absolute pathにも同じpath-safety検証が必要になる。 |
| B. target root/ancestor symlink | v1ではtraversalをrejectする | identity-safeな解決を条件にallowする | ownership境界とreviewは単純になるが、symlinkで構成されたworkspaceの柔軟性を失う。 |
| C. target ownership overlap | equivalentまたはnested overlapをrejectする | operation orderと明示ownershipでallowする | 0..N targetの独立性とpreflight/no-mutationを保ちやすいが、同じnamespaceへの意図的な複数配置は表現できない。 |

A、B、Cは、agentがApprovedと宣言せず、Human reviewerがこのproposalを承認するときに選択する。
特にAをallowする場合も、project-local canonical artifact rootの制約をexternal targetへ拡張したり、
path safetyを省略したりしない。

## 互換性（Compatibility）

現在external filesystem publisherは未実装であるため、現行production operationとのruntime
backward compatibilityは発生しない。このproposalが適用された場合、過去に文字列比較だけで
受理できたtargetの一部が、filesystem identity、symlink、manifest ambiguity、protected path、
またはtarget graph collisionを理由にrejectされ得る。これは既存user contentを破壊しないための
意図的なsafety boundaryであり、`PUBLISH-004`、`PUBLISH-006`、`PUBLISH-008`、`PUBLISH-009`を
弱めるものではない。

既存canonical build、YAML、Type System、Table/Key、project identity、relative target base、
C# manifest ownership、stale managed file retirement、unmanaged preservation、binary explicit-file
ownershipは変更しない。ASCII lowercase、NFC、OS名によるcase rule、Git repository topologyを
新しいidentity semanticsとして追加しない。

このproposalはpublish-only trust metadata、semantic schema hash、builder cache、Unity`.meta`
lifecycle、cross-target rollback/retry、`build --publish`、GUI、Referenceを定義しない。

## 受け入れと実装への影響（Acceptance and Implementation Impact）

これは`refine-spec`と`review-spec`のためのproposalであり、以下のtestはfuture evidenceである。
現時点でtestまたはfixtureが実装済み・pass済みであることを主張しない。

| Requirement | Planned evidence（すべてpending implementation） | Fixture / observation |
| --- | --- | --- |
| PUBLISH-PATH-001 | `publish_path_rejects_filesystem_equivalent_unmanaged_collision`; `publish_path_rejects_equivalent_binary_targets`; `publish_path_handles_case_sensitive_and_insensitive_volumes` | runtime filesystem behaviorをprobeし、OS名だけで分岐しない。 |
| PUBLISH-PATH-002 | `publish_path_accepts_safe_missing_target_tail`; `publish_path_rejects_unresolvable_identity_without_mutation` | existing prefix、missing tail、case behaviorの判定不能をfail closedで確認する。 |
| PUBLISH-PATH-003 | `publish_path_rejects_csharp_target_symlink`; `publish_path_rejects_csharp_ancestor_symlink`; `publish_path_never_follows_unmanaged_symlink` | symlink capabilityのあるplatformで実行する。 |
| PUBLISH-PATH-004 | `publish_path_rejects_case_or_unicode_equivalent_unmanaged_csharp_entry`; `publish_path_rejects_managed_path_type_change`; `publish_path_preserves_meta_and_unmanaged_files` | file/file、file/directory、directory/file、nested pathを含む。Unicodeはfilesystemがaliasと扱う場合だけcollisionを期待する。 |
| PUBLISH-PATH-005 | `publish_path_rejects_malformed_manifest_without_mutation`; `publish_path_rejects_manifest_alias_duplicates`; `publish_path_rejects_reserved_manifest_alias_collision` | missing manifestの初回publishはempty managed setとして確認する。 |
| PUBLISH-PATH-006 | `publish_path_rejects_binary_symlink_or_directory`; `publish_path_replaces_explicit_regular_file`; `publish_path_preserves_binary_siblings` | explicit fileだけがpublisher-ownedであることを確認する。 |
| PUBLISH-PATH-007 | `publish_path_rejects_source_canonical_cache_and_config_overlap`; `publish_path_allows_safe_project_local_dist` | target pathをMasterData critical pathへaliasさせ、sentinel不変を確認する。 |
| PUBLISH-PATH-008 | `publish_path_rejects_nested_csharp_targets`; `publish_path_rejects_csharp_binary_overlap`; `publish_path_rejects_target_aliases` | missing target同士もdestination namespaceで比較する。 |
| PUBLISH-PATH-009 | `publish_path_resolves_relative_target_from_project_root`; `publish_path_creates_missing_parent_after_preflight` | cwd変更、space/Unicode path、absolute path policyの選択を分離して検証する。 |
| PUBLISH-PATH-010 | `publish_path_validates_all_targets_before_mutation`; `publish_path_rejects_type_change_during_mutation`; `publish_path_preserves_previous_destination_on_preflight_error` | crash durabilityやpartial retryはこのproposalのevidenceに含めない。 |

実装時は、`PUBLISH-PATH-001`から`PUBLISH-PATH-010`を`masterdata-app`のpublish orchestrationへ
接続し、path resolution/identityのmechanismを適切なfilesystem adapterへ隔離する。Rust coreの
YAML semantics、canonical artifact builder、または.NET builderへpublish ownershipを複製しない。

## B8の境界

このproposalが閉じるのは、external publishを開始する前に、destination filesystemのidentityと
ownership namespaceを安全に検証するためのcontractである。filesystem crash/power-loss時の
cross-target atomicity、partial successのretry/rollback、publish-only artifact trust、Unity
importer、`.meta` lifecycle、remote publishingは別scopeである。

`PUBLISH-PATH-001`の原則は、Unicode pathについて「すべての文字列をNFCへ変換する」ことを意味
しない。destination filesystemが同一entryと扱うことを確認できたときだけcollisionとし、確認
不能時は`PUBLISH-PATH-002`に従ってrejectする。

## 未解決事項（Open Questions）

このproposalにおいて、Human reviewerがpath-safety contractとして選択する必要があるものだけを
残す。

- absolute external publish pathをv1でallowするかrejectするか（A）。
- C# targetおよびbinary targetの既存root/ancestor symlink traversalをv1で全面rejectするか、
  filesystem identityを安全に証明できる場合にallowするか（B）。
- 異なるpublish targetのequivalent/nested ownership regionを全面rejectするか、明示的な
  ordering/ownership contractを追加してallowするか（C）。

以下はこのproposalで推奨するcontractとして整理したため、追加の未定義semanticにはしない。

- existing prefixとmissing tailによるfuture pathの扱い、およびidentity判定不能時のfail-closed。
- manifest ambiguity、reserved manifest collision、unexpected entry typeのsafe rejection。
- binary explicit-file ownership、parent creationはpreflight後に限ること、protected MasterData pathの拒否。

partial publishのretry/rollback、publish-only trust metadata、semantic hash/cache、Unity`.meta`、
build --publishなどの既存Open Questionは、このB8 proposalへ複製しない。

## レビュー（Review）

Status `Proposed`。Human Approvalは未実施であり、このproposalをimplementation authorityとして
扱ってはならない。reviewでは、既存Approved `PUBLISH-001`から`PUBLISH-010`との重複・強度・
ownership boundaryを確認し、A/B/Cの選択と、target-to-target partial operationを別scopeへ残す
ことを確認する。

canonical `build-pipeline.md`、`project-layout.md`、spec status、Rust code、CLI、Tauri、
filesystem publisher、tests、fixturesはこのrefinementでは変更しない。

## 承認記録（Approval Record）

<!-- Human maintainerによる明示的なapproval operationはまだ実施されていない。 -->
