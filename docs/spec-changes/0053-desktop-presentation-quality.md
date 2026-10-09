# 仕様変更0053: Desktop presentation quality

Status: Applied

## Affected Specifications

[GUI app shell](../gui/app-shell.md)のpresentation acceptanceを補う。[Color Theme](../gui/color-theme/spec.md)、[Grid Authoring](../gui/data-editor/grid-authoring.md)、[Performance](../gui/performance.md)の既存authorityを維持する。technology choiceのWHYは[ADR 0009](../adr/0009-clean-room-desktop-workspace.md)へ反映する。

## Source Evidence and Classification

Decision: 2026-10-04 Human-selected「Desktop Presentation Quality / React + Ant Design」。React + current stable Ant Designをpresentation baselineとし、feature / persistent surfaceを増やさず、professional desktop productとしてvisual / interaction / motion品質を上げる。plain DOM choiceだけを置換し、Rust semantics / workspace / source safety / bounded projectionを保持する。

Constraint: D1–D5、stable grid、single active editor、bounded rendering、latest-selection-wins、performance measurementsは維持する。gridはinteraction / performance上必要ならcustom Reactとする。

Agent Decision: observable acceptanceはapp shellへ集約し、theme / drag / performance本文は既存ownerへのlinkにする。exact pixelや特定component treeをacceptanceにしない。

## Proposed Delta

- `GUI-SHELL-PRESENTATION-001`: compact professional density、統一されたvisual hierarchy / tokens / controls / icons、標準overlayの品質。custom gridも共通systemへ統合する。正常状態の常設説明やsurface bloatを追加しない。
- `GUI-SHELL-PRESENTATION-002`: hierarchy / causality / continuityに役立つ短いmotion、reduced motion、animation中の入力受付、stable geometryを要求する。drag、theme、性能の意味論は既存ownerを維持する。
- `GUI-SHELL-PRESENTATION-003`: representative screens / interactionsのactual Desktop visual reviewをcompletion evidenceへ加える。functional DOM assertionだけでvisual quality完了としない。

## Compatibility / Implementation Impact

YAML / config / CLI / consumer API / Save scopeは変更しない。既存Rust engineとnative hostを保持し、frontend presentationをReact / Ant Designへ移行する。schema、validation、write authorizationをfrontendへ複製しない。plain DOM基準の性能証拠はhistoricalとして保持し、React commitを含むfresh実測を採る。

## Acceptance

theme/token・standard controls・compact density・overlay・purposeful/reduced motion・custom grid統合・surface bloatをfresh reviewする。actual DesktopでWelcome、Table / dirty、Problems、Complex、menu、Conflict、Migration、Build / Publish、Dark、long / wide、dragを確認する。performance distributions / work counts / bounded mounted rowsのregressionを検証する。

## Open Questions / Potential ADRs

None。ADR 0009のpresentation choiceとalternative理由だけ更新する。

## Approval Eligibility

Autonomous approval eligible: Yes（owner routing / acceptance refinement）。Human gate: None。React / Ant Designのproduct directionはHuman自身が確定済み。追加のsemantic changeはない。

## Fresh challenge review / Approval Record

proposal確定後の別passで、intent fidelity、既存theme / grid / performanceとの整合、normative strength、compatibility、testability、owner routingを確認した。standard UI systemを情報architectureやdomain authorityへ昇格せず、new capability / persistent panelを要求しない。exact pixels / component topologyは固定しない。

Blocking Issues: None identified。Non-blocking Issues: None identified。Questions: None identified。Approved as Proposed: Yes。Autonomous approval eligibility: Yes。Human gate: None。

Approval mode: Human（React / Ant Design baselineとpresentation direction）。Decision basis: 2026-10-04追加Human-selected Decision。Approval mode: Agent-autonomous（acceptance refinement / owner routing）。Objective/request basis: 継続中Clean-room Rewriteと上記Decision。Review result: Blockingなし。Application: GUI app shellの下記3 RequirementとADR 0009へ適用。technology移行やvisual reviewの完了をこのapprovalで主張しない。
