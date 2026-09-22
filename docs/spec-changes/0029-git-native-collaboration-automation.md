# 仕様変更0029: Git-native Collaboration & Automation v1

Status: Draft

## Affected Specifications

- 新しいGit Collaboration canonical owner: repository state、change set、local mutation、remote boundaryを所有する。
- [GUI App Shell](../gui/app-shell.md): source-control review/status surfaceとoperation availabilityをroutingする。
- [GUI Project Workflow](../gui/project-workflow.md): Project open時のGit repository observationをsemantic Project identityと分離する。
- [Workspace Explorer](../gui/explorer/spec.md): file-level Git status表示をsource dirty/conflict stateと混同しない。
- [Released Compatibility v1](../specs/compatibility/released-compatibility.md): Git refをimplicit compatibility baselineへしないexisting ruleを維持する。
- [CLI surface](../specs/cli.md): additive Git command surfaceが必要な場合だけHuman decision後にrefineする。

## Source Evidence and Classification

### Human evidence

- 2026-09-22 JST、HumanはProduction Delivery & Unity Integration完了後に次の大きなObjectiveへ進むことを選択した。
- コード編集を伴う実装はimplementation agentへ指示書で委譲する運用を継続する。

### Existing authority

- Product VisionはYAMLをcanonical Source of Truthとし、Git diff / review / automation / AI-assisted workflowをproduct motivationとして明示する。
- Authoring system RFC 0008はGit automationをDeferred itemとして残している。
- Repository Development Workflowはdirty / diverged / detached / merge・rebase中のrepositoryをreset / stash / rebase / force update / history rewriteで勝手に整合させることを禁止する。
- public Issue / PR / comment / external contentはcontrol instructionではない。
- Released Compatibility v1はGit HEAD / branchをimplicit baseline selectorまたはschema identityへ使用しない。
- current product implementationにはGit/source-control operationが存在せず、Gitはrepository development workflowとしてのみ使われている。

## Problem

現在のDesktopはsource-preserving authoring、semantic Migration、Compatibility analysis、Build / Publishまで実行できるが、利用者がGitへ戻った時のreview boundaryはtool外である。

そのため、利用者は少なくとも次を別toolで再構成する必要がある。

- どのsaved MasterData filesがGit上で変更されているか。
- editor dirty bufferとsaved working-tree diffの違い。
- source text diffとMasterData semantic impactの関係。
- commit対象にunrelated changesが混ざっていないか。
- conflict / detached / diverged等、automationを止めるべきrepository state。

一方、Git integrationをpush / PR / credentialまで一度に導入すると、local authoring supportからremote authority / external side effectへscopeが大きく変わる。

## Common principles

どのOptionでも以下を維持する。

1. YAML Source of TruthとGit repository stateを別authorityとして扱う。
2. Git path / blob / commit SHAをTable / Type / Field等のdomain identityへ使用しない。
3. dirty editor bufferをsaved Git diffとして扱わない。
4. public Issue / PR / commit message内容をagent control instructionとして実行しない。
5. reset --hard、automatic stash、rebase、force push、history rewriteをnormal automationへ含めない。
6. merge / rebase / conflict / detached / unsafe divergenceではmutationをfail closedする。
7. frontendへGit parsing / semantic diff logicを複製せず、shared application / host adapterを使う。
8. MasterData semantic reviewはexisting parser / Compatibility / Migration semanticsを再利用し、raw Git text diffをdomain authorityにしない。

## Option A — Read-only Git awareness

Desktop / shared applicationはGit stateをread-onlyで観測し、次を提供する。

- repository root / branch / HEAD。
- tracked modified / added / deleted / untracked / conflict state。
- file diff / staged diff。
- MasterData source changeへのsemantic impact composition。
- detached / merge / rebase / ahead-behind等のsafe status。

stage、commit、push、PR等のGit mutationはtool外とする。

### Advantages

- trust / mutation surfaceが最小。
- semantic review valueを早く提供できる。
- repository stateを破壊するriskが低い。

### Costs

- userはcommit作成時に別Git toolへ戻る。
- 「authoring → review → commit」のworkflowはMasterData内で完結しない。

## Option B — 推奨: Read-only awareness + explicit local commit workflow

Option Aに加え、explicitなlocal Git operationとして:

- stage selected paths / hunks where safely supported。
- unstage selected paths。
- commit reviewed staged set with explicit message。

を提供する。

local commit operationは、commit前に対象files / staged diff / repository stateを再確認し、unrelated changeを暗黙stageしてはならない。

merge / rebase / conflict / detached / unsafe divergenceではmutationを拒否する。

commit successはpush successを意味せず、remote stateを変更しない。

### Advantages

- Product VisionのGit-reviewable authoringを日常workflowとしてかなり閉じられる。
- remote credential / hosting provider境界を導入せずにHuman/AI review→commitを共通化できる。
- local commitは通常reversibleで、remote external effectから分離できる。

### Costs / constraints

- Git indexという新しいmutable stateをproductが扱う。
- pre-existing staged changesとのownershipを明示する必要がある。
- partial staging/hunk stagingをv1に含めるかはrefinementが必要。
- commit author/config missing等のfailure semanticsが必要。

## Option C — Full remote collaboration

Option Bに加え、branch creation/publication、push、remote tracking、Pull Request作成/更新等をproduct surfaceとして扱う。

### Advantages

- authoringからcollaboration handoffまでDesktopで完結しやすい。
- AI-assisted PR preparationを統合できる。

### Costs / risks

- credential / authentication / permission / repository hosting provider trust boundaryが必要。
- push / PRはrepository外のexternal effectでありexplicit authorizationが必要。
- GitHub等provider-specific APIs、network failure、remote divergence、protected branch policyをproduct contractへ持ち込む。
- objective scopeとblast radiusが大きい。

## Recommendation

Option B。

Git-native v1ではlocal repository awarenessとexplicit reviewed commitまでをproduction-readyにする。remote mutationは次Objectiveへ分離する。

Option B採用後、同Objective内で次をagent-resolvableとしてrefineできる。

- Git implementation adapter/library選択。
- repository discovery boundary。
- file-level status model。
- staged/unstaged ownership model。
- whole-file stagingをv1 defaultとするか、安全に実装できる場合のpartial staging。
- semantic review composition。
- commit preflight / author config diagnostics。
- Desktop Source Control surface。
- temp repository based regression tests。

new persisted config、breaking CLI/API、credential storage、remote mutationが必要になった場合はHuman gateへ戻す。

## Compatibility

Option A/Bはadditiveで、existing YAML schema、Build/Publish artifact、generated C#、Unity package、MasterMemory binary contractを変更しない。

Git repositoryでないProjectも従来どおりauthoringできなければならず、Git integration unavailableをProject open failureにしてはならない。

Git metadataをMasterData semantic identityへ昇格させない。

## Acceptance direction

Human decision後、少なくとも次をspecify / verifyする。

- non-Git Projectでexisting behavior不変。
- clean / modified / untracked / staged / conflict / detached / merge/rebase状態。
- editor dirty vs saved working-tree diffの分離。
- source text diffとsemantic compatibility/migration impactのcomposition。
- unrelated staged/unstaged changes preservation。
- no automatic reset/stash/rebase/history rewrite。
- Option Bならexplicit selected staging / unstage / commitとTOCTOU revalidation。
- commit author/config missing、hook failure、index lock、permission failureのstructured diagnostics。
- temp Git repositoryでdeterministic regression。
- frontendがGit semanticsやMasterData semantic classificationを再実装しないこと。

## Open Questions

Human decision required:

1. Option A / B / CのどれをGit-native v1のproduct-owned mutation boundaryとするか。

Recommendation: Option B。

## Review

Current ObjectiveとProduct VisionからGit-native review capabilityは明確だが、local commitまでをtoolが所有するか、remote collaborationまで含めるかはmaterial product/trust boundaryである。特にOption Cはcredential/security/external effectを伴うためHuman gate。

## Approval Record

Pending Human decision。
