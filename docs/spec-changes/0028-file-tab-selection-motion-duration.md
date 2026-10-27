# タブ選択アニメーション時間の調整

Status: Applied

## Basis / scope

Human Requirement（2026-10-28）: 「時間2倍はやりすぎなので1.5倍くらいに抑えたい」。
直前の変更（提案0026）で180msから360ms（2倍）へ延長されたfile tab切替時の緑の上辺アニメーション時間について、過度な遅延感を解消するため、基準値180msの1.5倍にあたる約270msへ調整する。
Canonical owner: Desktop UI-003（[Desktop Authoring](../specs/desktop.md)）。

## Behavior / Agent Decision

- file tab切替時の選択先tab上辺の伸長・fadeを約270ms（基準180msの1.5倍）とする。イージング補間（`cubic-bezier(.4, 0, .2, 1)`）は維持し、変化の視認性を保ちながら快適な操作感を実現する（Agent Decision: 180ms * 1.5 = 270msを採用）。
- 本文と入力の即時切り替え、同じtabへの再clickやdirty更新で再生しない挙動、タブ追加・閉じる（close）motion、keyboard操作、連続切替、reduced motionの規則はすべて維持する。

## Acceptance / validation

- file tabのアニメーションdurationが270msであることを確認する。
- 切替motionのdurationと中間frame（スケール・不透明度）が適切に補間されることを確認する。
- 本文の即時表示・draft保持・再click時の非再生・reduced motionの尊重が損なわれていないことを既存E2Eテストで検証する。
- 追加・close時のmotionに影響がないことを確認する。
- `tests/desktop.e2e.ts` のタブアニメーション検証を更新し、focused tests、workspace / diff checkがすべてpassすること。

## Safety / unresolved

None。CSSアニメーション時間の局所的な調整であり、保存形式・権限・データ互換性・他UIへの影響はない。Autonomous approval eligible: Yes。

## Review / approval

別passで、Human intentへの忠実性、中間frameの視認性と操作感の両立、本文の即時切替やdraft保持などの不変規則をchallenge reviewした。Blocking Issues / Non-blocking Issues / Questions: None。Approved as Proposed: Yes。Autonomous approval eligible: Yes。Human gate: None。

Approval mode: Agent-autonomous
Objective / request basis: 2026-10-28のHuman指示「時間2倍はやりすぎなので1.5倍くらいに抑えたい」。review後に承認しcanonicalへ適用済み。
