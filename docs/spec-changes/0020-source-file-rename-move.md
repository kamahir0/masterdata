# 仕様変更: Source file rename / move

Status: Applied

> Historical audit record. Current implementation authority is the canonical specification listed below. Full proposal / review detail remains in Git history.

## Why

same configured source root内のexisting source rename / move、dirty target resolution、no-overwrite Conflict、case-only rename、shared application / host safetyをP4-B contractとして採用した。

## Canonical result

- [Source Path Mutation](../specs/source-path-mutation.md) — `SOURCE-PATH-001..007`
- [Project Layout](../specs/project-layout.md) — path / identity boundary
- [Source Creation](../specs/source-creation.md) — rename/move routing
- [Workspace Explorer](../gui/explorer/spec.md) — `GUI-EXPLORER-STATE-004`, `GUI-EXPLORER-INT-004..006`, `GUI-EXPLORER-ERR-002`
- [GUI App Shell](../gui/app-shell.md) — `GUI-SHELL-CAPABILITY-001`

## Approval / application

Human approval: 2026-09-20 JST。Canonical application commit: `5ce8e0e8154e3d4f152c883f88bac34537d89ca6`。Implementation authorityはcanonical Approved specificationsのみ。
