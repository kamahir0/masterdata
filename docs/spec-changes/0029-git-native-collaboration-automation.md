# 仕様変更0029: Git-native Collaboration & Automation v1

Status: Rejected

## Affected Specifications

- Product Vision
- CLI / GUI project workflow
- Development workflow

## Decision

2026-09-23 JST、HumanはProduct Simplification & Scope Cleanupを優先し、Git client、stage/unstage、commit、push、Pull Request、
credential/provider integrationをMasterdata productへ追加しないと決定した。YAML sourceは通常のGit資産としてreviewする。

## Preserved boundary

Product Save / Build / PublishへGit mutationを暗黙に結合しない。repository development workflowの通常のGit操作はAGENTS.mdと
`docs/execution-workflow.md`に従う。

## Application

Rejected。proposalの詳細はGit historyに保持し、current implementation authorityにはしない。
