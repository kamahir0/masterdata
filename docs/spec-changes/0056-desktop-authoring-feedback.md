# Desktop操作feedbackの修正

Status: Applied

## Affected Specifications / evidence

2026-10-10 Human request。Recent Projects / themeの到達性、左ペイン開閉、rootノード省略、inline文字サイズ、左右 / 上下dragの連続motion、menu dismiss、viewport固定のAdd操作を対象とする。既存のGUI-SHELL-LAYOUT-001、GUI-PROJECT-002、GUI-THEME-001/007、GUI-GRID-006/007、GUI-SHELL-PRESENTATION-002は保持する。

## Decisions / delta

Requirement: 左ペイン全体の開閉、inline入力と表示の同じ文字サイズ、menuの外側click / Escape閉鎖、列追加はviewport右端、行追加はviewport左下で常時到達可能。
Proposal: 単一configured source rootの冗長なfolder行を省略する。Agent Decision: 単一rootはその直下を表示し、複数rootは区別を維持する。source path / creation destination / domain identityは変更しない。folder creation / rename / typed selectionは既存authorityへ委譲する。
Agent Decision: Welcomeのrecent sectionは0件でも空状態を示す。theme入口はProjectの有無を問わず専用Application Settings buttonから到達可能にする。sidebarはunmountせずtree / draft / focus状態を保持し、keyboard shortcutを提供する。

canonical delta: GUI-EXPLORER-001に単一rootの表示省略。GUI-SHELL-LAYOUT-001に全pane開閉とstate保持。GUI-PROJECT-002にWelcome空状態。GUI-THEME-007にWelcomeからの到達。新GUI-GRID-008にinline typography / viewport-fixed Add / menu dismiss。motionは既存007 / PRESENTATION-002の違反修正で新要件を追加しない。

## Compatibility / acceptance / questions

presentation-only。source / config / wire identity / CLI / shared Rust semantics / physical historyは不変。既存native Project / focus / geometry / authoring adaptersへ指定behaviorの回帰を追加し、actual Desktopで確認する。新fixture・quality bar・OS permission変更なし。
Open Questions: None。Potential ADRs: None。Autonomous approval eligible: Yes。Human gate: None（今回の明示requestの範囲）。既存external evidence gatesは維持する。

## Review / approval

確定proposalの別pass review: Intent fidelity / consistency / terminology / strength / testability / compatibility / ambiguity / implementation leakage / ownership: Pass。root省略はHumanのProposalをAgent Decisionとして区別し、複数rootとphysical pathを維持する。theme / recent / motionは既存authorityへrouteし、永続設定や新featureを増やさない。Blocking / Non-blocking / Questions: None identified。Approved as Proposed: Yes。Eligible: Yes。Human gate: None。

Approval mode: Agent-autonomous。Basis: 2026-10-10 Human-selected Desktop feedback修正。canonical ownerへ適用済み。
