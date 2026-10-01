# 仕様変更0050: Interactive navigation

Status: Applied

## Canonical result

Human-selected Objectiveにより、開いたProjectのselectionをview切替とし、内部read/session architectureの置換を許可。immediate feedback、latest selection wins、generation-correct diagnosticsを適用し、dirty ownershipとfresh write preflightを維持する。

Canonical owners: [Explorer](../gui/explorer/spec.md) `GUI-EXPLORER-NAV-001`、[Data Editor](../gui/data-editor/spec.md) `GUI-DATA-VAL-005`、[ADR 0008](../adr/0008-interactive-workspace-read-session.md)。measurementは[Historical Evidence](../evidence/interactive-navigation.md)。

## Approval Record

Approval mode: Agent-autonomous。Humanのexplicit architecture direction、baselineと既存Source / Save safetyをbasisに、proposal確定後の別passでintent / ownership / compatibility / failure / testabilityをreview。Autonomous approval eligible: Yes。Human gate: None。Approved as Proposed: Yes。Blocking / Questions: None。

Canonical application: `7b2b3cbefd29d98f177c7f35d118f96bab8d872e`。詳細proposalはこのcommitのGit historyから復元できる。persisted source / config / CLI / public stable protocolの変更なし。
