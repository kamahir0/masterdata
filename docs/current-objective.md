# Current Objective

## Objective

タブ切り替え時の選択アニメーション時間を約270ms（基準時間180msの約1.5倍）に調整し、過度な遅延感を解消しつつ視認性を維持する。

## Completion boundary

[変更提案0028](spec-changes/0028-file-tab-selection-motion-duration.md)の仕様策定・review・承認、[Desktop Authoring](specs/desktop.md)への適用、`src/style.css`と`tests/desktop.e2e.ts`の更新、focused testsとworkspace check、通常commit / push。本文の即時切替・draft保持・追加/close motionの維持。
Authorization: Humanの「時間2倍はやりすぎなので1.5倍くらいに抑えたい」（2026-10-28）。

## Non-scope

タブ追加・削除アニメーションの変更、他のUIアニメーションの変更、実データの変更、リリース作成。
