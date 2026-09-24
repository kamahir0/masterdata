# Desktopアプリ終了の未保存保護

Status: Applied

## Why / Adopted Decision

macOSのCmd+Qが未保存bufferを失わせる実機再現に対し、通常の終了にも既存guardを適用する。
Humanの2026-09-24 Desktop UX改善依頼をbasisとするAgent Decision。source/config/APIの互換性は不変。

## Canonical result

[GUI app shell](../gui/app-shell.md) — `GUI-SHELL-LIFECYCLE-001`

## Approval Record

Approval mode: Agent-autonomous
2026-09-24、Human-selected Desktop UX改善Objectiveに基づき別passでreview。Blocking Issues / Non-blocking Issues / Questions: None identified。Approved as Proposed: Yes。Eligible: Yes。Human gate: None。Intent、既存Save契約、用語、normative strength、testability、互換性、ambiguity、implementation leakage、scope、ownerを確認した。canonical ownerへ適用済み。
