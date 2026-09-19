---
name: refine-spec
description: Turn a request or Current Objective into a traceable Draft/Proposed specification change, resolving low-risk Objective-local decisions while escalating only true Human gates.
---

# refine-spec

## 目的

product/domain/compatibility/user-visible GUI behaviorを変更する可能性があるrequestを、review可能なDraft/Proposed changeへ変換する。

このskillはproduct implementationを行わない。authoring passとchallenge passを分離するため、refinement中に`Approved`へstatus transitionしない。ただしHuman-selected Objective内のlow-risk ambiguityをAgent Decisionとして解決してよい。

## 必須context

1. `AGENTS.md`、Current Objective、Development State。
2. affected Approved / Implemented spec、関連ADR/RFC、Product terminology。
3. existing Requirement ID / wording search。
4. affected implementation / tests / fixturesはimpact evidenceとして必要な範囲。
5. `docs/contributing/specification-workflow.md`のHuman gate / autonomous approval条件。

memory、stale conversation、current codeだけからproduct ruleを作らない。

## Evidence classification

各statementを必要な範囲で分類する。

- `Decision`
- `Requirement`
- `Constraint`
- `Preference`
- `Proposal`
- `Idea`
- `Question`
- `Open Question`
- `Rejected`
- `Agent Decision`

Agent DecisionはHuman発言と区別し、選択理由とalternativeを短く残す。

## Agent Decisionを使える条件

次をすべて満たす場合、未指定default / UX detail / failure presentation等をAgent Decisionとして解決してよい（MAY）。

- Current Objectiveのscope内。
- reversible / low blast radius。
- non-breaking。
- destructive / security / permission / external irreversible effectでない。
- Approved architecture / product directionと整合。
- test可能。
- alternative間のtrade-offを説明できる。

material product fork、breaking change、data loss、security boundary等は解決せずHuman gateへ送る。

「implementationが楽だから」だけを理由にobservable behaviorを選ばない。

## Canonical owner

Approved / Implemented canonical documentへのsemantic deltaは`docs/spec-changes/`の別artifactへ置く。未reviewed semanticsをcanonicalへ直接混ぜない。

new canonical specではfile-level statusが一貫するようscopeを切る。Requirement IDは既存definitionを検索してから割り当て、再利用しない。

Accepted RFCはimplementation authorityではない。

## Normative strength

evidence / Agent Decisionが支える強度を使う。

- `MUST / MUST NOT`: required invariant / constraint
- `SHOULD / SHOULD NOT`: strong default with documented exceptions
- `MAY`: permission / capability

Human statementの強度を勝手に上げない。Agent Decisionの場合は「Humanが要求した」と書かない。

## Open Questions

Open Questionを機械的にHumanへ戻さない。

- Agent Decision条件を満たす -> alternativesを比較し、1つ選び、Confirmed Decisionsへ記録。
- Human gate -> Open Question + Human decision needed。
- Objective外 -> explicit non-scope / deferred。
- evidence不足で安全に判断不能 -> Human gateまたはKnowledge/Specification Gap。

## Required artifact sections

### Affected Specifications

file / status / affected Requirement ID。

### Source Evidence and Classification

Human/durable evidenceとAgent Decisionを区別する。

### Confirmed Decisions

Human decisions + autonomous criteriaを満たしたAgent Decisions。Agent Decisionには短いrationaleを付ける。

### New Requirements

stable ID + normative wording。

### Changed Requirements

existing ID / old meaning / proposed meaning / compatibility。

### Open Questions

Human gateまたは未解決事項だけ。なければ`None identified`。

### Potential ADRs

cross-cutting WHYが必要ならADR、それ以外は`None identified`。

### Compatibility Impact

backward compatible / migration required / breaking / not applicableを明示。

### Implementation Impact

affected boundary / focused tests / fixture / verification。

### Approval Eligibility

- `Autonomous approval eligible: Yes|No`
- `Human gate: None|<reason>`
- eligibilityの短い根拠。

## Status

整理中は`Draft`、review可能なら`Proposed`。refine-spec pass自身は`Approved`へ変更しない。

review後、workflow orchestrationがautonomous approval条件を満たせばHuman promptなしでapprove/applyしてよい。

## Safety

- conversation logをspecへ貼らない。
- Human evidenceとAgent Decisionを混同しない。
- current implementationを未承認requirementへ昇格しない。
- Requirement IDとDiagnostic Codeを混同しない。
- 1 ruleを複数ownerへcopyしない。
- breaking/destructive/security decisionをAgent Decisionで処理しない。
- future maintainerが意図を壊し得るdecisionはspec / ADR / rationale / testへdurably残す。
