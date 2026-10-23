# Development State

Stage: verification-ready
Candidate: a8af1638d10ecff0a61818f087a27f8b09361b81
Work base: 8e0ed71587f1f05fca657361c863d6ba10037b09

## Active work

Completed: version整合、local checksとself-review、固定tagの全OS release build、配布物検証、UIテストのmotion待機修正。
In progress: APT取得対策後の全OS Workspace CI。
Remaining: 公開と結果確認（日本語notes反映済み）。

## Blocking findings

None。公開はHumanの明示依頼でauthorization済み。

## Evidence

- 製品修正Candidate 335c667の[全OS CI](https://github.com/kamahir0/masterdata/actions/runs/39870208073)成功を確認済み。以後はstate metadataのみ。
- 既存v0.1.0は古いcommitのdraft。既存tagを保持し、修正を含む新しいpatch versionとしてv0.1.1を選択した。

- version 6箇所の一致、型検査、cargo check --locked、workspace / diff check、actionlint成功。製品コードは335c667と同じで、versionと作業stateだけを変更した。
- [Release](https://github.com/kamahir0/masterdata/actions/runs/39877635499)成功。固定tag v0.1.1は694a0caを指す。8 assetsが揃い、7配布物のSHA-256、3 OSのCLI形式、Linux packageのversionを確認した。
- 配布DMGを検証し、アプリを実パスへ配置して起動成功。macOS CLIでsampleのvalidate成功。検証で起動したappは終了し、DMGもunmount済み。
- [tag Workspace CI](https://github.com/kamahir0/masterdata/actions/runs/39877635474)のWindows / macOSで、表示アニメーション中のmenu clickが空振りするテスト不備が発生。b774ec2までにmotion完了と実際の行・列削除を待つよう修正し、2テストをChromium / WebKit各3回（計12回）成功。
- [テスト修正後CI](https://github.com/kamahir0/masterdata/actions/runs/39879798616)のmacOS / WindowsはUI・起動・開発installまで成功。Linux依存取得はAzure mirrorへの再試行で停滞し、再実行でも同じ兆候を確認した。
- a8af163で停滞mirrorを除外し、APTの通信待機と再試行を制限。署名検証とpackage集合を保持し、エラーは成功扱いにしない。bash -n / shellcheck / actionlint / workspace / diff check成功。[最終CI](https://github.com/kamahir0/masterdata/actions/runs/39982563109)を確認中。置き換えられた旧Linux再実行はcancelした。
- 配布tagとの差分はtest・CI tooling・stateだけで、製品コード・設定・依存は同一。既存のindex.htmlのユーザー変更は含めていない。
