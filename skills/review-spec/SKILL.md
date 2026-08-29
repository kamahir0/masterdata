---
name: review-spec
description: Independently audit a Draft or Proposed specification change against its source intent and repository contracts before human approval.
---

# review-spec

## 目的

refinementで作成されたspecification changeにchallengeを行うskillである。reviewerはauthorと同じ仮定を共有せず、
target change、元のrequestまたはconversation（利用可能な場合）、関連specification、ADR、RFC、terminology glossaryを
確認する。結果はreview reportであり、status transitionではない。

`Approved as Proposed` は「reviewerがhuman considerationを妨げるblocking issueを見つけなかった」という意味である。
`Status: Proposed` を `Status: Approved` へ変更してはならない（MUST NOT）。approval operationを行うのは人間の
maintainerだけである。

## Inputとscope

必須inputはtarget spec changeとそのpathである。source request/conversationとaffected testまたはfixture planも、
可能なら提供する。sourceがない場合はintent fidelityを検証できないと記載し、memoryから補完してはならない。

次を読む。

1. `AGENTS.md`。
2. target specification全体（status、ID、Open Questions、examples、Non-Goalsを含む）。
3. 関連する `docs/specs` と、referenceされた各IDのcanonical owner。
4. 関連する `docs/adr`、`docs/rfcs`、`docs/product/terminology.md`。
5. current behaviorのevidenceとしてexisting tests/fixturesを読む。ただし、Approved contractを上書きする許可とは
   扱わない。

changeが `Approved` または `Implemented` canonical documentを対象とする場合は、`docs/spec-changes/` 配下の別proposal
（alternative比較にはRFC）を要求する。approved canonical documentには、approvedな旧behaviorとunapprovedな新behaviorを
混在させてはならない。

`Status:` はcanonical file全体に適用される。document内のrequirementがmaterially異なるmaturityを持つ場合は、status
transitionをrecommendする前にstructural splitを要求する。splitでは既存Requirement IDを保持する。requirementの移動
だけではsemantic changeではない。directoryの `README.md` はspec familyのindexであり、normative requirementを所有しない。

RFC statusはproduct-spec statusとは別に扱う。`Accepted` RFCはdecision recordでありimplementation inputではない。
specification-change artifactでは、`Approved` が明示的なhuman decisionを記録し、`Applied` はatomic canonical merge
後だけに使われていることを確認する。

duplicate ID、terminology variant、conflicting phraseは `rg` で検索する。reviewはsemanticに集中し、typoまたはpurely
internal refactorのためのspecificationを要求しない。

## Review checklist

findingを作る場合は、file、Requirement ID、evidence、impact、具体的なresolutionを記録する。

### Intent fidelity

- すべてのnormative ruleに、supplied requestまたはexisting approved contractのevidenceがあるか。
- `Preference`、`Proposal`、`Idea`、`Question`、`Open Question` をnon-normativeに保ったか。
- Rejected choiceを排除したか。
- wordingがspeakerのscopeとcertaintyを保持しているか。
- source evidenceなしにcurrent implementation behavior、diagnostic code、test nameをrequirementへ昇格していないか。

### Internal consistency

- status、Summary、Normative Requirements、Validation Rules、Compatibility、Examples、Open Questions、Non-Goalsが
  整合しているか。
- file内のnormative requirementが宣言されたdocument statusを合法的に共有できるか。splitが必要ではないか。
- Requirement IDがchange内でuniqueであり、editをまたいでstableか。
- requirement同士が同一document内で矛盾していないか。
- requirement definitionとreferenceを区別し、referenceされたIDをcanonical documentが1つだけ所有しているか。

### Cross-spec consistency

- changeがaffectedなApproved/Proposed specすべてと一致しているか。
- core/CLI/GUIと.NET bridge boundaryなど、ADRのarchitectureを尊重しているか。
- canonical ownerが明確で、copyではなくlinkを使っているか。
- duplicateまたはconflicting requirementが他に存在しないか。
- existing implementationにtarget specが許可していないsemanticsがあり、retroactive approvalではなくremovalまたは
  別proposalが必要になっていないか。
- diagnostic-to-requirement relationが実際にrequirement semanticsを表しているか。単にprojectまたはnumber prefixを
  共有するだけではないか。

### Terminology consistency

- termが `docs/product/terminology.md` に従っているか。
- table/file、field ID/index number、unique/non-unique、source/generated artifactなどの区別を保っているか。
- 定義またはroutingなしに新しいtermを導入していないか。

### Normative strength

- 各 `MUST`、`MUST NOT`、`SHOULD`、`SHOULD NOT`、`MAY` が根拠を持つか。
- capabilityをrecommendationまたはrequirementへ強めていないか。
- recommendationにlabelを付け、理由を示しているか。
- exampleまたはimplementation noteがnormative behaviorとして誤読されないか。
- runtime Diagnostic CodeとRequirement IDが明確に区別され、shared namespaceやmisleadingなnumeric suffix reuseがないか。

### Testability

- 各normative requirementを観測しtestできるか。
- success、validation failure、error、compatibility outcomeがacceptance testを書ける程度に定義されているか。
- 有用な場合、Requirement IDをtest nameまたは近接commentに対応付けているか。
- fixtureが必要なend-to-end ruleにだけfixture coverageを要求しているか。
- test name/comment内のRequirement IDが、単にnumberを借りるのではなく実際に対象behaviorを説明しているか。

### Backward compatibility

- stable table/field/enum identity、serialized data、generated API、file interpretation、migrationへ影響するか。
- not applicable を含め、compatibilityを明示しているか。
- deprecated IDとmigration/replacement noteを保っているか。

### Unresolved ambiguity

- default、nullability、ordering、error severity、missing data、edge caseが、evidenceで定義されるかOpen Questionに
  記録されているか。
- Open Questionをprose、example、test planで黙って解決していないか。
- 未回答のquestionがimplementation behaviorを変えるほど大きく、approvalをblockするか。

### Implementation leakage

- convenientなdata structure、algorithm、crate layout、shell commandではなくobservable behaviorを定義しているか。
- GUI textがGUI behavior boundaryに留まり、domain semanticsを `masterdata-core` に残しているか。
- MasterMemory internalの再実装や、.NET process invocationの別crateへの移動を避けているか。

### Unrequested behavior

- requestまたはapproved contractにないfeature、default、validation、UI state、migration behaviorを追加していないか。
- adjacent ideaをNon-Goals、RFC、Open Questionsへ置いているか。
- semantic changeをApproved/Implemented canonical fileへ直接混ぜず、durable proposalへ隔離しているか。
- Accepted RFCまたはcurrent implementationがcanonical `Approved` specification gateを迂回していないか。

### Reverse traceabilityとimplementation rationale

implementation diff、current code、または関連evidenceがreview scopeに含まれる場合は、
次も確認する。全branch、clone、allocationへcommentを要求するのではなく、straightforwardな
実装から意図的に外れたnon-obvious codeを対象にする。

- non-obvious codeに、必要な範囲でrecoverableな`WHY`と、削除・簡略化時のfailure modeがあるか。
- rationaleがWHATの説明だけでなく、protected invariantを説明しているか。
- referenced regression testが、commentで主張されたbehaviorを実際に保護しているか。
- Requirement ID referenceが、実際のrequirement semanticsを正しく指しているか。
- platform/library/toolchain workaroundのaffected constraint、protected behavior、removal condition、durable evidenceが古くなっていないか。
- refactorによってrationaleが旧locationに取り残されていないか。
- performance optimizationにbenchmark、profile、allocation evidence、またはknown hot pathがあるか。
- undocumented invariantを越えてcodeがsimplify、delete、deduplicateされていないか。

local rationaleはproduct specificationの代替ではない。Approved specが既に定義するbehavior
へのimplementation違反はbug findingとし、specを後付けで正当化しない。behaviorの選択自体が
必要なら`Specification Gap`としてrefinementへ戻す。

## Required output

人間が素早くapprovalを判断できるよう、次の構造を使用する。

### Blocking Issues

proposalをunsafe、ambiguous、contradictory、untestable、over-specified、またはsource evidenceで支持されない状態に
するissue。空なら `None identified` と記載する。

### Non-blocking Issues

approvalを妨げないeditorial、traceability、maintainability上のconcern。空なら `None identified` と記載する。

### Questions

authorまたはhuman maintainerへのquestion。特に、blocking issueではないが未決定のbehaviorを含む。空なら
`None identified` と記載する。

### Approved as Proposed

`Yes` または `No` と短い理由を記載する。これはreview recommendationに過ぎず、human approvalとstatus transitionが
まだ必要であることを明記する。

さらに、Intent fidelity、Internal consistency、Cross-spec consistency、Terminology consistency、Normative strength、
Testability、Backward compatibility、Unresolved ambiguity、Implementation leakage、Unrequested behaviorを含むcompactな
verdict tableまたはlistを付ける。

implementation rationaleをreviewした場合は、rationale gap、orphaned rationale、stale reference、
または `None identified` をこのreportへ含める。

## 安全策

- review中にOpen Questionを解決しない。
- weak statementをneatnessのために強い表現へnormalizeしない。
- specを `Approved` または `Implemented` に変更しない。
- IDまたはtermが共有される場合、変更されたparagraphだけをreviewしない。
- 明示的なevidenceなしにcurrent code behaviorを新しいrequirementにしない。
- implementation、diagnostic、mismatched test labelをrequirementのauthorityとして扱わない。
