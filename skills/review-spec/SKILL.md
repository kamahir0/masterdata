---
name: review-spec
description: Independently challenge a Draft/Proposed specification change and decide whether it is safe for autonomous approval or requires a Human gate.
---

# review-spec

## 目的

refinementで作成されたproposalを、authoring assumptionsから離れたfresh challenge passとしてreviewする。

review-spec pass自身はstatusを変更しない。結果はworkflow orchestrationへ渡し、autonomous approvalまたはHuman gateを決める。

## Input

- target Draft / Proposed change
- Current Objective / Development State
- source request / durable evidence
- affected Approved / Implemented specs
- related ADR/RFC/terminology
- relevant current tests / implementation evidence
- `docs/contributing/specification-workflow.md`のautonomy / Human gate criteria

source evidenceが取得できない場合、intent fidelityを推測しない。

## Review principles

- Draft / Proposed / current codeをApproved authorityとして扱わない。
- Agent DecisionはHuman statementとして扱わず、autonomy criteriaに照らして妥当性をreviewする。
- review中に新しいobservable choiceを黙って追加しない。必要ならrefine-specへ戻す。
- minor editorial issueでHuman gateを作らない。
- Human gateはblast radius / compatibility / security / product significanceに基づく。

## Checklist

### Intent fidelity

- Human Decision / Requirement / Constraintのscopeとstrengthを保持しているか。
- Preference / IdeaをHuman Decisionとして偽装していないか。
- Agent Decisionが明示され、Objective内で必要か。
- Rejected / non-scopeを再導入していないか。

### Internal consistency

- status、requirements、validation、compatibility、Open Questions、non-scopeが整合するか。
- Requirement IDがunique / stableか。
-同一ruleのduplicate ownerがないか。

### Cross-spec / architecture

- affected Approved contractsと矛盾しないか。
- core/application/GUI/.NET/host boundaryを尊重しているか。
- Accepted RFCやcurrent implementationでApproved gateを迂回していないか。

### Normative strength

- MUST / SHOULD / MAYがevidenceまたは明示Agent Decisionで支持されるか。
- implementation detailをobservable requirementに固定していないか。

### Testability / failure semantics

- success / failure / conflict / compatibility outcomeをtest可能か。
- data safety、lost update、partial failure等のboundaryが必要なscopeで定義されているか。
- focused regression evidenceを作れるか。

### Compatibility

- stable identity / serialization / public API / protocol / CLI / config / migration impactが明示されているか。
- breakingならHuman gateへ送っているか。

### Human gate classification

次のどれかを含む場合は原則`Autonomous approval eligible: No`。

- Objective / priority material expansion
- intentional data loss / destructive irreversible semantics
- breaking compatibility / stable identity / persisted or public contract change
- security / trust / permission boundary
- external irreversible publish/deploy/cost
- material product fork
- source evidence / rationale不足で安全にchoiceを説明できない

### Maintainability / durability

- future maintainerが意図に反するsimplificationをし得る重要decisionが適切なownerへ残るか。
- architecture WHYならADR、regressionならtest、unusual implementation constraintならlocal rationaleへrouteできているか。
- documentation quantityではなくdecision recoverabilityを満たすか。

## Required output

### Blocking Issues

unsafe / contradictory / untestable / unsupported / Human gate misclassification。なければ`None identified`。

### Non-blocking Issues

approvalを妨げないconcern。なければ`None identified`。

### Questions

refinementへ戻す必要のあるquestion。なければ`None identified`。

### Approved as Proposed

`Yes` / `No`。これはsemantic review verdictでありstatus transitionではない。

### Autonomous approval eligibility

- `Eligible: Yes|No`
- `Human gate: None|<reason>`
- `Rationale: ...`

`Approved as Proposed: Yes`でもHuman gateがあればautonomous approvalは`No`になり得る。

### Review dimensions

Intent fidelity、Internal consistency、Cross-spec consistency、Terminology、Normative strength、Testability、Backward compatibility、Unresolved ambiguity、Implementation leakage、Unrequested behavior、Durabilityをcompactに示す。

## Handoff

- Approved as Proposed=Yes + Eligible=Yes -> orchestrationはapprove/applyし、同じautonomous runでimplementationへ進めてよい。
- Approved as Proposed=Yes + Eligible=No -> `decision-required`へHuman gateを記録。
- Blockingあり -> refine-specへ戻して修正。Human gateでなければ同じrunで再reviewする。
- Specificationではなくimplementation違反を発見 -> review-code / bug fixへroute。

## Safety

- review pass中にproposal外のsemantic choiceを発明しない。
- Human gateを「念のため」で増やさない。
- breaking / destructive / security-sensitive changeをautonomous eligibleにしない。
- current code、test、diagnosticをproduct authorityに昇格しない。
