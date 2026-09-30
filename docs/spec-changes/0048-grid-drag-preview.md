# 仕様変更0048: Grid dragの追従表示

Status: Applied

## Canonical result

2026-10-01のHuman requestに基づき、Column / Record Rowがpointerへ追従し、周囲が移動先を空ける表示へ変更する。Agent Decisionとして操作軸への追従、非対話的なpreview、cancel境界とreduced motion対応を採用した。既存drop時のみのmutation/history、keyboard経路、sticky / bounded rendering、shared source semanticsを維持する。

現在のbehaviorは次が所有する。

- [Unified Table Editor](../gui/table-editor/spec.md): `GUI-UNIFIED-008`
- [Grid Authoring](../gui/data-editor/grid-authoring.md): `GUI-GRID-007`

## Approval Record

Approval mode: Agent-autonomous。Basis: Human-selected drag追従表示実装、proposal確定後の別passによるreview、non-breaking GUI変更。

Blocking Issues / Non-blocking Issues / Questions: None identified。Approved as Proposed: Yes。Autonomous approval eligibility: Eligible Yes; Human gate None。Human intentとAgent Decisionを区別し、history/identity、failure semantics、sticky/virtualization、owner、normative strength、testability、compatibilityを照合してcanonicalへ適用した。
