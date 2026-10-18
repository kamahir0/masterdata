# Tauri移行と開発版の固定インストール

Status: Applied

Approval mode: Agent-autonomous
Basis: 2026-10-18 HumanによるTauri移行・一括build/install・開発版の識別と重複防止の依頼、および通常の技術判断の委任。

Review: 別passで入力保持、更新失敗・起動中の保護、固定identity、native境界、UI差分の検証を確認。Blocking / Questions: None。Approved as Proposed: Yes。Human gate: None。

GUIを作り直さずdesktop境界を交換し、開発版のidentityと更新先を固定する変更。[Desktop Authoring](../specs/desktop.md)へ適用済み。技術判断と理由は[ADR-0001](../adr/0001-desktop-session.md)、詳細な提案・検証・修正履歴はGitが所有する。
