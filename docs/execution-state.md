# Development State

Stage: objective-complete
Candidate: a8af1638d10ecff0a61818f087a27f8b09361b81
Work base: 8e0ed71587f1f05fca657361c863d6ba10037b09

## Active work

None.

## Blocking findings

None.

## Evidence

- Humanの公開依頼に従い、[v0.1.1](https://github.com/kamahir0/masterdata/releases/tag/v0.1.1)を2026-10-24 JSTに公開。Latest、draft=false、8 assets、日本語notesと公開checksumの認証なし取得を確認した。既存v0.1.0 draft / tagは保持。
- 固定tag v0.1.1は694a0ca1be3605872ec07b30436d60fbe3017a92。[全OS配布build](https://github.com/kamahir0/masterdata/actions/runs/39877635499)成功。7配布物のSHA-256、3 OSのCLI形式、Linux packageのversionを確認し、公開直前にremoteの8 asset digestも再照合した。
- Candidateの[全OS CI](https://github.com/kamahir0/masterdata/actions/runs/39982563109)成功。UIはmacOS / Linux各178件、Windows94件成功（非対応browser機能を各10件skip）。通常・native tests、起動と対象OSの開発installも成功。
- 配布DMGのアプリで新規ファイル・フォルダのEscape取消、sampleの編集・保存を実操作し、保存ファイル照合と配布CLI validateも成功。検証用appと関連processの終了、DMGのunmountを確認済み。
- UIテストのpopup待機不足を修正しfocused 12回成功。APTの停滞mirrorを除外し、通信待機と再試行を制限。Linux依存取得44秒・Playwright依存取得58秒で成功した。bash -n / shellcheck / actionlint / 型検査 / cargo check --locked / workspace / diff check成功。
- 配布tagとCandidateとの差分はtest・CI tooling・stateのみ。製品コード・設定・依存は同一で、version 6箇所は0.1.1。既存のindex.htmlのユーザー変更は含めていない。
