# Desktop日本語・開発者用語の見直し

Status: Applied

## Why / Adopted decision

日本語第一言語を維持し、説明・通常操作と開発者の慣用的な概念名を役割で使い分ける。型カテゴリとBuild / Publishは原語、配列・主キー・参照は日本語、Table declaration surfaceは「テーブル定義」。操作ラベルを短縮しても対象と安全上必要な説明を失わない。

## Canonical result

[GUI-SHELL-LANGUAGE-001](../gui/app-shell.md#gui-shell-language-001)が表示方針、[Desktop表示語](../product/terminology.md#desktop表示語)が用語対応のowner。以前の0055を否定せず、そのHuman intentの解釈を精製する。新feature / 多言語基盤 / domain semantics変更は含まない。

## Approval / application

Approval mode: Agent-autonomous。Basis: 2026-10-09日本語中心request、2026-10-10全体見直しplanの依頼と、そのplanに対するHumanの「実装」指示。Proposed確定後の同一agent別pass reviewでintent / cross-spec / terminology / normative strength / testability / compatibility / ownershipを確認。Blocking Issues / Non-blocking Issues / Questions: None identified。Approved as Proposed: Yes。Eligible: Yes。Human gate: None for this presentation scope。既存external evidence gatesは維持。

canonical applicationはこのApplied記録と同じ変更でatomicに行う。詳細plan / reviewはProposed版のGit history（79781b66aa43f83315ed2f614c7596d398b75324）に保持する。
