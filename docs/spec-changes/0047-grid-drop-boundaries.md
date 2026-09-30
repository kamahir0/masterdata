# 仕様変更0047: Grid dragの挿入境界

Status: Applied

## Canonical result

2026-09-30のHuman requestに基づき、同じカラム間・レコード間の挿入線が2箇所へずれる問題と、縦線がfield定義部全体に一致しない問題を修正する。Humanが参考として許可したMasterData-Legacyのgapモデルを比較した。source mutation、identity、API、Saveは変更しない。

現在のbehaviorは次が所有する。

- [Unified Table Editor](../gui/table-editor/spec.md): `GUI-UNIFIED-008`
- [Grid Authoring](../gui/data-editor/grid-authoring.md): `GUI-GRID-007`

## Approval Record

Approval mode: Agent-autonomous。Basis: Human-selected drag品質修正、2026-09-30のproposal確定後の別passによるreview、non-breaking GUI correction。

Blocking Issues / Non-blocking Issues / Questions: None identified。Approved as Proposed: Yes。Autonomous approval eligibility: Eligible Yes; Human gate None。Human intent、existing drop/no-op・history、sticky/virtualization、owner、normative strength、testability、compatibilityを照合し、canonicalへ適用した。
