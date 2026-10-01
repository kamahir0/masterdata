# 仕様変更0050: Interactive navigation

Status: Applied

## Affected Specifications

Explorer `GUI-EXPLORER-INT-001` / new `GUI-EXPLORER-NAV-001`、Data Editor `GUI-DATA-VAL-005`。Source / Save contractは変更しない。

## Source Evidence and Classification

Human Decision: Project already open時のselectionはview切替とする。内部read/session APIの置換を許可し、重複するfull reload、frontend semantic fork、stale editable cache、dirty消失を禁止する。
Human Requirement: immediate target feedback、latest selection wins、generation-correct diagnostics、external clean reload / dirty Conflict、write-time fresh preflight。
Constraint: source authority、shared semantics、inline / separate / mixed、Save / Migration、history / query / schema draft。
Evidence: [baseline](../evidence/interactive-navigation.md)と現在の独立したData/context/Table read。source切替で繰り返すproject-wide parseが支配的。

## Confirmed Decisions

Agent Decision: [ADR 0008](../adr/0008-interactive-workspace-read-session.md)へread model ownership / lifetimeとfreshnessを置く。GUI仕様にはcache algorithmを置かない。
Agent Decision: fresh projection待ちでは新targetとpendingを表示し、superseded requestはneutralにdiscardする。既存dirty overlayをsession base更新と混同しない。

## New / Changed Requirements

`GUI-EXPLORER-NAV-001`: selectionとmain surfaceのtarget表示はbackend completionを待たず反映する。old sourceをnew targetのcurrent contentとして表示しない。pending / unavailableを識別する。rapid selectionはlatest targetだけをactiveにし、obsolete responseで巻き戻さない。selectionだけでdirty / history / query / schema draftを破棄しない。cacheをeditableにする前にshared Applicationがrelevant sourceのfreshnessとcurrent projectionを確認する。

`GUI-DATA-VAL-005`: source viewのauthoring capabilityとproject diagnostics completionを分離してよい。diagnosticsはworkspace/source generationへ対応づけ、checking / unavailable / currentを区別する。古いgenerationをcurrentとして表示しない。

## Open Questions

None。Desktop baselineのevent-loop / paint診断はimplementation evidenceとして継続採取する。

## Potential ADRs

ADR 0008: long-lived read modelとauthoritative write preflightの分離。

## Compatibility Impact

Non-breaking persisted/source/config/CLI/public protocol。GUI internal APIは置換する。新しいfilesystem / network authorityは加えない。

## Implementation Impact

Application read session、Tauriのnavigation coalescing / background execution、frontend target/results orchestration、generation-correct invalidation / diagnostics。

## Acceptance

同一固定inputのbefore / after、実Desktop pointer / keyboard / rapid stress、dirty/history/query保持、external invalid/deleted/dependency change、cached sourceへのexternal mutation後のSave Conflict、ordinary authoring suite、100k/10k、repository / CI checks。

## Approval Eligibility

Autonomous approval eligible: Yes
Human gate: None
Humanがscopeとarchitecture置換を明示委任した。write authorizationとpublic compatibility boundaryは変更しない。

## Approval Record

Approval mode: Agent-autonomous。Human-selected navigation architecture Objectiveとbaselineをbasisに、proposal確定後の別passでintent / cross-spec / ownership / testability / failure / compatibilityをreview。Blocking / Questions: None。Approved as Proposed: Yes。Eligible Yes、Human gate None。
