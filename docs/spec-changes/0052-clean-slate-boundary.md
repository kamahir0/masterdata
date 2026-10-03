# 仕様変更0052: Clean-slate境界

Status: Applied

## Source / confirmed decisions

2026-10-04 Human-selected Legacy Decommission / Clean-slate Preparation。Humanはlegacy freeze、専用branchでの旧実装退役、正式contract / 独立oracle保持、新実装禁止、fresh contextへのhandoffを選択済み。

## Affected Specifications / delta

[CLI-010](../specs/cli.md)、[Build責務境界](../specs/build-pipeline.md)、[GUI index](../gui/README.md)、[ADR0002](../adr/0002-rust-core-shared-by-cli-and-gui.md)、[ADR0003](../adr/0003-dotnet-mastermemory-bridge.md)。旧crate / service名はarchitecture constraintから除く。shared Rust semantics、in-process CLI利用、native .NET delegation、structured diagnostics、Build/Publish separationは維持する。glossaryのscaffold/hash/API説明はdomain意味とhistorical mechanismを分離する。

## Agent Decisions / compatibility / acceptance

全tracked assetをhash付きmanifestへ分類し、legacy workspaceと混在xtaskを退役。小さなboundary checkerとbranch CIだけ残す。corpus / fixture bytesを変更しない。historical source linkはlegacy-finalのforensic linkへ変換する。

公開source / config / CLI / generated consumer APIに変更なし。performance target、D1〜D6、安全契約に変更なし。新adapter / product runtimeを作らない。

Acceptance: oracle parse / reference / byte integrity、current docs link integrity、state、forbidden paths、tag / main / branch integrity、code-blind review。旧runtime suiteのpassは要求しない。

Open Questions: None。Potential ADRs: 新architecture decisionなし。

## Approval Eligibility

Autonomous approval eligible: Yes。Human gate: None（retirement方針はexplicit Human authorization済み）。file placement / transition checkerはAgent Decision。

## Fresh review / Approval Record

別passでintent fidelity / source safety / public compatibility / ownership / implementation leakageを照合。Blocking Issues: None identified。Questions: None。Approved as Proposed: Yes。

Approval mode: Human（freeze / 専用branch退役 / 新実装禁止）。Decision basis: 今回の明示Human prompt。
Approval mode: Agent-autonomous（implementation名の拘束解除 / routing / minimal checker）。Eligible: Yes。Human gate: None。公開意味論は保持し、旧architectureを再seedしない。canonical適用済み。
