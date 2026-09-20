# 仕様変更: Project conventions、Settings scope、public generate removal

Status: Applied

> Historical audit record. Current implementation authority is the canonical specification listed below. Full proposal / review detail remains in Git history.

## Why

public CLIからgenerateを外し、project source convention、settings scope、tool-state ownership、init scaffoldをcanonical化した。

## Canonical result

- `docs/specs/cli.md` — `CLI-002`, historical `CLI-004`, `CLI-011`
- `docs/specs/project-layout.md` — project conventions / settings / init / tool-state requirements
- `docs/specs/README.md` — index only

## Approval / application

Human maintainerがreview済み0012 proposalを明示承認。public generate removal、kind-first convention、3-scope settings separation、tool-state ownership等のdeltaをcanonical ownersへ適用した。empty-directory placeholder strategyは承認scope外。
