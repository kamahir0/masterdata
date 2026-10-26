# 表の操作位置とBrowserの識別表示

Status: Applied

## Basis / scope

Human Requirement（2026-10-26、画像付き）: 横scrollbarを最終recordの直下ではなく表編集領域の下端（追加buttonの上）に配置する。file tab切替motionを視認しやすくし、追加時のmotionは維持する。列の左右移動iconを方向に合わせる。Browser faviconを修復し、welcomeに小さくversionを表示する。
Canonical owner: Desktop EDIT-002 / UI-003、Browser WEB-001。

## Behavior / Agent Decision

- 横方向にoverflowする表のscrollbarは、少数・空・多数recordや検索結果の件数によらず、現在の表編集viewport下端に配置する。footerの追加操作を覆わず、window resize、Problems開閉、tab切替にも追従する。headerとbodyの横位置、固定列、仮想化、末尾行の操作を保持する。OS / 既存controlの自動hideを常時表示へ変更する要件ではない。
- file tab選択の上辺の伸長・fadeを約360msにし、前半から終盤まで変化を読み取れる補間にする（Agent Decision: 180msの急なease-outを緩める）。本文と入力は直ちに切り替え、同じtabへの再clickやdirty更新で再生しない。追加・closeのmotion、keyboard、連続切替、reduced motionの規則を維持する。
- 列menuの「左へ」「右へ」はそれぞれ左・右の矢印iconを使う。行menuの上下iconと実際の移動・Undoは維持する。
- Browserのtabに既存の製品iconをfaviconとして指定する。静的buildとサブpath配信でも解決でき、icon取得が404にならない。別のicon designは作らない。
- Browserのwelcomeにbuild元のpackage version（例: v0.1.1）を控えめな小さい文字で表示する。version文字列をUIへ二重管理せず、project未選択・非対応Browserでも確認できる。Desktopやprojectのversion表示、更新検知、release作成は対象外。

## Acceptance / validation

- 少数・空・多数recordで横scrollbarの下端とviewport / footer上端の一致を確認する。resize・Problems開閉後にも追従し、scrollbar dragでheader / cellが一緒に動く。既存の仮想表・横scroll・固定列・追加操作の回帰を行う。
- 切替motionのdurationと途中frameを確認し、本文の即時表示・draft保持・再click非再生・reduced motionを確認する。追加 / close motionも回帰する。
- 列menuのicon方向と移動・Undoを確認する。
- production buildをBrowserで開き、faviconの参照・SVG内容とwelcomeのpackage version表示を確認する。静的サブpathでもassetが解決できることを確認する。
- 通常tests、型検査、Chromium / WebKit focused tests、代表作業の実操作、workspace / diff checkと必要なCI。

## Safety / unresolved

None。表示・配置と配信assetの修正で、保存形式・権限・データ互換性を変更しない。新しいGitHub Releaseは作成しない。Autonomous approval eligible: Yes（依頼内の可逆なUI判断）。

## Review / approval

別passで、空・少数・仮想化された多数record、resizeとProblems、本文の即時切替・draft・reduced motion、列と行の方向、静的サブpathとversionの単一ownerをchallenge reviewした。Blocking Issues / Non-blocking Issues / Questions: None。Approved as Proposed: Yes。Autonomous approval eligible: Yes（依頼内の表示改善で、保存・権限の境界変更なし）。Human gate: None。

Approval mode: Agent-autonomous
Objective / request basis: 2026-10-26の5点の明示依頼。review後にorchestrationが承認しcanonicalへ適用済み。
