# 仕様変更: Complex Value Authoring v1

Status: Applied

> Historical audit record. Current implementation authority is the canonical specification listed below. Full proposal / review detail remains in Git history.

## Why

shared schema-driven value authoring modelをexisting editとAdd Rowで共用し、Fine-grained source preservationとAdded draftのYAML null placeholderを採用した。

## Canonical result

- `docs/specs/source-edit.md` — affected `SOURCE-EDIT-003/005/006/014..016`
- `docs/specs/source-record-mutation.md` — affected `SOURCE-RECORD-002..004/014..015`
- `docs/gui/data-editor/spec.md` — affected Data Editor state/edit/validation requirements
- `docs/gui/data-editor/record-mutation.md` — affected Added record requirements

## Approval / application

Human maintainerが2026-09-16にOption C + Fine-grained preservation + YAML null placeholderを明示承認。同じcanonical mergeで4 ownerへdeltaを適用した。
