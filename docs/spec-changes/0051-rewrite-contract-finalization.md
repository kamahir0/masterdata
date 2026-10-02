# 仕様変更0051: Rewrite Contract Finalization

Status: Applied

## Problem / Human decision

Human-selected Rewrite Contract Finalization（2026-10-02）のD1〜D6を適用する。監査DraftだけではApproved ownerと衝突し、既存surfaceや内部APIを次実装へ再生成する危険がある。

- D1: standalone Diffをbaselineから外し、unsaved candidate / Conflict / Migrationのcompareを残す。
- D2 / D3: Search / Findを残し、Typed Filter / View SortをDeferredにする。
- D4: lossless typed copy/paste、shared preflight、unsafe failureのall-or-none、一回Undoを残し、Fill / range Set Null専用workflow / persistent Batch previewをbaselineから外す。
- D5: standalone Saved Table Overviewをbaselineから外し、physical source composition / saved vs dirty / Build inclusion / tagsの意味論をcontextual inspectionへ残す。
- D6: existing flow mappingをaccept / preserveし、安全な局所patchを許可する。新規mapping生成はblock mapping。unsafe localizationはfail closed。

## Affected Specifications

[GUI app shell](../gui/app-shell.md)、[Data Editor](../gui/data-editor/spec.md)、[Grid Authoring](../gui/data-editor/grid-authoring.md)、[Table Overview](../gui/table-overview/spec.md)、[Project Workflow](../gui/project-workflow.md)、[Rewrite baseline](../gui/rewrite-baseline.md)、[Performance](../gui/performance.md)、[Authoring Query](../specs/authoring-query.md)、[Authoring Batch](../specs/authoring-batch.md)、[YAML subset](../specs/yaml-subset.md)、[Source Edit](../specs/source-edit.md)。

## Canonical application

GUI app shell / Data Editor / Grid Authoring / Table Overview / Project Workflowは、上記surfaceの必須性を解除する。既存legacy capabilityはtransition中MAY remainであり、現在のproduction removalを要求しない。compare / clipboard / source compositionのsafetyは維持する。

Authoring QueryはSearchとsaved-source compositionの意味論を維持し、typed filter / sortの要求をlegacy scopeへ限定する。YAML subsetのflow禁止を新Requirementへ置換し、source editへ新規block writerのRequirementを追加する。置換元IDはretirement recordとして保持する。

古いinitial slice、revoked Save、Complex footer、insertion-lineの説明はhistorical ownerへrouteし、current MUSTとして復活させない。

Rewrite preparationはformal target inputを所有し、canonicalの全文を複製しない。data-only oracleはsource bytes、domain operation intent、expected outcomeを所有し、test-only adapterはlegacy APIsへ接続するだけで期待結果を生成しない。

## Agent Decisions / acceptance

corpus placement / format、test classification manifest、measurement adaptersはAgent Decision。production architecture / UI / dependencyは変更しない。performance targetはHuman指定のnext-frame feedback、warm backend p95 < 50ms級、usable editor p95 < 150ms（stretch < 100ms）。硬い構造契約とreference-hardware budgetを分ける。

acceptanceは独立byte / topology / identity / clipboard / flow / Build / Publish / consumer oracleと、Tier1 distribution evidenceで検証する。現行実装とのgapをoracle期待値へ取り込まない。実測不足はReady gateの不足として明示する。

## Compatibility / review boundary

source / config / public CLI / generated consumer APIは維持する。D6はHuman-approved existing source compatibilityの明確化。D1〜D5はrewrite baseline surfaceの縮小であり、意味論やcurrent production capabilityの削除ではない。

## Fresh challenge review / Approval Record

proposal確定後の別passでintent fidelity、cross-owner、normative strength、compatibility、testability、documentation budgetをreviewした。D1のcompare、D2のSearch、D4のpaste、D5のdomain compositionを削除しないこと、D6の新規writerと既存styleを分けることを確認した。

Blocking Issues: None。Non-blocking Issues: evidence未採取をReadyと誤表示しない。Questions: None。Approved as Proposed: Yes。

Approval mode: Human（D1〜D6）。Decision basis: 2026-10-02 Human-selected Rewrite Contract Finalization promptの確定decision。Human gate: Resolved by that explicit decision。追加のpublic break、format変更、safety弱化を承認したものではない。

Approval mode: Agent-autonomous（owner routing / corpus・measurement形式 / stale prose精製）。Autonomous approval eligibility: Yes。Human gate: None。Objective内、production変更なし、期待結果が独立に検証可能、既存compatibilityを維持する。

Application: canonical owner / Requirement routing適用済み。production legacy capabilityはtransition中保持。readiness / evidence gateはrewrite finalization reportが所有し、AppliedをReadyの代わりにしない。
