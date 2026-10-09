# Desktopの日本語を第一言語にする

Status: Applied

## Affected Specifications

[App shell](../gui/app-shell.md)のpresentationへ`GUI-SHELL-LANGUAGE-001`を追加する。

## Source Evidence / Confirmed Decisions

2026-10-09 Humanの明示request（Decision / Requirement）：アプリを全面的に日本語中心とし、特に設定項目の説明文を日本語化する。一般的な日本語版開発ツールでも英語を保つ製品名・システム名・ディレクトリ等は英語のままにする。

## Canonical result

[GUI-SHELL-LANGUAGE-001](../gui/app-shell.md#gui-shell-language-001)がcurrent authority。日本語化のscopeとtechnical identity保持を所有する。追加の言語選択・多言語基盤は要求しない（Agent Decision）。

## Compatibility / Implementation / Acceptance

表示だけの変更。YAML / TOML bytes、wire / CLI / generated API、dirty / history / write / navigation semanticsを変更しない。Ant Design日本語locale、既存全Desktop surfaceの文言・アクセシビリティ・focus selectorを更新し、既存native Desktop adaptersとactual代表画面で確認する。f7cf90dの証拠を新Candidateの証拠へ読み替えない。

## Open Questions / Approval Eligibility

Open Questions: None。Potential ADRs: None（presentation-only）。Autonomous approval eligible: Yes。Human gate: None（日本語化自体は明示authorization済み）。既存macOS permission / Windows / Unity evidence gateは維持する。

## Approval / application

Approval mode: Agent-autonomous。Basis: 2026-10-09 Human requestとCurrent Objectiveの明示追加scope。確定proposalの別pass reviewでintent / consistency / terminology / compatibility / testability / ownershipを確認。Blocking Issues / Non-blocking Issues / Questions: None identified。Approved as Proposed: Yes。Eligible: Yes。Human gate: None。表示以外のsemanticsを変えず、未取得の旧Candidate証拠を新Candidateへ転用しない。
