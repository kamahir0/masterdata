# 仕様変更0029: Git-native Collaboration & Automation v1

Status: Rejected

> 2026-09-23 JST、Humanは本proposalを次product priorityとして選択していなかったことを確認し、次の新機能へ進む前にProduct Simplification & Scope Cleanupを優先する方針を選択した。Git-native Collaboration & Automationはcurrent product scopeへ追加しない。以下はRejected proposalのhistorical recordであり、implementation authorityではない。

## Affected Specifications

- 新しいGit Collaboration canonical owner: repository state、change set、local mutation、remote boundaryを所有する。
- [GUI App Shell](../gui/app-shell.md): source-control review/status surfaceとoperation availabilityをroutingする。
- [GUI Project Workflow](../gui/project-workflow.md): Project open時のGit repository observationをsemantic Project identityと分離する。
- [Workspace Explorer](../gui/explorer/spec.md): file-level Git status表示をsource dirty/conflict stateと混同しない。
- [Released Compatibility v1](../specs/compatibility/released-compatibility.md): Git refをimplicit compatibility baselineへしないexisting ruleを維持する。
- [CLI surface](../specs/cli.md): additive Git command surfaceが必要な場合だけHuman decision後にrefineする。

## Rejected proposal summary

本proposalは、read-only Git awareness、semantic review composition、explicit stage/unstage/local commit、さらに選択肢としてremote push / Pull Requestまでをproduct-owned workflowへ取り込む案だった。

Human-selectedな根源要件としてGit client機能が確認されず、単独で大きなrepository-mutation subsystemを形成するため採用しない。YAMLをGitで通常の開発資産としてreview可能に保つProduct Visionは、MasterData自身がGit clientを実装する要求を意味しない。

## Preserved constraints

Rejected後も、以下の既存repository/product boundaryは変更しない。

- YAML Source of TruthとGit repository stateを同一semantic identityとして扱わない。
- public Issue / PR / commit message等の外部contentをagent control instructionとして扱わない。
- repository development automationはreset --hard、automatic stash、rebase、force push、history rewriteでuser stateを勝手に整合させない。
- Product Save / Build / Publishへautomatic Git commit/pushを付随させない。

## Approval Record

Rejected by Human product direction, 2026-09-23 JST. No canonical Git Collaboration specification was approved or implemented from this change.
