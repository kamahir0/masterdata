---
name: refine-spec
description: Turn a design conversation or request into a traceable Draft/Proposed specification change without promoting assumptions, ideas, or questions into approved behavior.
---

# refine-spec

## 目的

conversation、issue、requestがproduct、domain、compatibility、またはuser-visibleなGUI behaviorを変更する
可能性がある場合に、このskillを使用する。役割は「何をspecificationにすべきか」を判断することである。
review可能なDraft/Proposed changeと、簡潔なrefinement reportを作成する。product behaviorを実装せず、
specificationを `Approved` に自動変更してはならない（MUST NOT）。

typo fix、formatting-only change、public/domain semantic effectを持たないinternal refactorでは、通常この
skillは不要である。この境界が不明な場合はskillを使用し、不確実性を記録する。

## 必須のcontext

1. repository fileを変更する前に `AGENTS.md` を読む。
2. semantic changeを提案する前に、`docs/specs` の関連fileと `docs/adr` の関連fileを読む。alternative designを
   まだ比較している場合は `docs/rfcs` も読む。
3. `docs/product/terminology.md` を読み、そのtermを使用する。termがrepository glossaryと衝突する場合は、
   synonymを発明せずconflictを明示する。
4. IDを追加または新ruleを置く前に、`rg` で既存Requirement IDと関連wordingを検索する。
5. requestがCLI、GUI、`masterdata-core`、`masterdata-codegen-csharp`、`masterdata-dotnet`、fixtures、または
   testsに影響するかを特定する。これはimpact analysisであり、implementationの許可ではない。

参照されたconversation、issue、source documentが利用できない場合は、提供されていないと明記する。記憶や
明示されていない仮定から復元しない。

## 手順

### 1. Evidenceを抽出する

requestをevidenceとして読み、明示された内容と単に便利そうな内容を分離する。speakerのintentとscopeを保持する。
statementはnormativeでなくても重要であり得る。

関連する各statementを、次のいずれかに分類する。

- `Decision`: 明示的に確定したchoice。proposed ruleの根拠にはなるが、reviewとhuman approvalはなお必要。
- `Requirement`: desired capabilityまたはoutcome。発言を確認せずMUST、SHOULD、MAYを割り当てない。
- `Constraint`: boundary、prohibition、condition。明確なprohibitionはproposal内のMUST NOTを支持し得る。
- `Preference`: binding commitmentを伴わないfavored optionまたはpriority。
- `Proposal`: 検討対象として示されたcandidate solution。
- `Idea`: exploratory possibilityまたはbrainstorming thought。
- `Question`: informationまたはclarificationのrequest。
- `Open Question`: unresolved choiceまたはambiguity。follow-upのために残す。
- `Rejected`: 明示的に退けられたoptionまたはbehavior。黙って再導入してはならない。

「これでもよいかもしれない」「こちらの方がよさそう」「Xはどうか？」は、それだけでは `Decision` evidenceでは
ない。「Xにする」「Yは望まない」「Xは必須である」は、scopeが明確なら `Decision` または `Constraint` となり得る。

### 2. Repository contractと比較する

canonical owner specificationを特定する。同じsemantic ruleを複数documentへcopyせず、既存requirementを更新する
ことを優先する。ApprovedとProposedのspecification、ADR、GUI boundary、terminology glossaryとのconflictを確認する。
conflictを黙って上書きせず、両方を報告して必要なhuman decisionを示す。

canonical documentが `Approved` または `Implemented` の場合、semantic deltaを直接編集せず、
`docs/spec-changes/` 配下の別のdurable proposalへ記録する。alternativeを比較中ならRFCを使用し、human-approved
atomic mergeが完了するまでcanonical documentを唯一のauthorityとして扱う。

`Status:` はcanonical specification file全体に適用される。候補file内のすべてのnormative requirementが同じ
lifecycle stateを共有できるか確認する。成熟度が異なる場合はapproval前にcanonical fileを分割する。構造上の
分割では既存Requirement IDを保持し、移動だけを理由に新IDを割り当てない。directoryの `README.md` はnon-canonical
indexとしてのみ使用できる。

`Accepted` RFCは選択されたdesign directionを記録するが、それ自体はproduct specificationではなくimplementationを
許可しない。human decision後、採用されたbehaviorをcanonical specification（既存canonical documentへのchangeなら
specification-change artifact）へ送り、未解決のcompatibility decisionをcopyしない。

### 3. Normative strengthを保持する

evidenceが支える最も弱いwordingを使う。

- `MUST` / `MUST NOT`: 明示的なrequirementとconstraint。
- `SHOULD` / `SHOULD NOT`: 明示的な強いrecommendationとその理由。
- `MAY`: permissionまたはcapability。recommendationではない。

「可能」を「推奨」へ、また「推奨」を「必須」へ変換してはならない。例えば、すべてのtable fileを同じdirectoryに
置けるようにするrequestは、「file location MUST NOT determine table identity」および「multiple table data files
MAY coexist in one directory」を支持し得る。しかし、明示的なrecommendationがない限り「そこへ置くべきである」
（SHOULD）とはできない。

未指定のdefault、edge-case behavior、error severity、ordering、nullability、migration policy、implementation
constraintを発明してはならない。各未解決事項を `Open Questions` に残す。implementation planを容易にするために
questionを解決しない。

### 4. ChangeをDraftする

新しいdomain specificationには `docs/specs/_template.md`、新しいGUI surface specificationには
`docs/gui/_template.md` を使用する。新しいnormative requirementにはstable IDを付ける。IDはuppercase segmentと
3桁の末尾numberを使い、`SCHEMA-VO-001` や `GUI-TABLE-EDIT-001` のような形式にする。

IDを割り当てる前に、すべてのspecificationを検索する。IDは削除後も再利用しない。既存
requirementの意味が変わる場合、history上旧意味を残す必要があれば旧IDを保ち、predecessor/deprecation noteを持つ
新IDを割り当てる。既存requirementへのreferenceは新しいrequirementではない。

整理中のnew documentは `Draft`、review可能になったdocumentは `Proposed` とする。file境界は、そのfile内の
normative requirementが同じstatusを共有できる程度に狭くする。Approved/Implemented documentへのsemantic changeは
statusが `Draft` または `Proposed` の `docs/spec-changes/` artifactへ記録し、canonical documentをdowngrade
してはならない。`Proposed` はapprovalではない。このskill中にdocumentを `Approved` へ変更しない。

### 5. Downstream workを評価する

compatibilityとimplementation impactを、missing semanticsを設計せずに記述する。想定するacceptance testとfixture
needを特定する。小さなruleにはfixtureを必須にせず、unit testで十分な場合はそう記載する。GUI behaviorでは関連
stateとinteractionを特定するが、core domain logicを重複させない。

大きなalternativeを比較するdesignならRFCをrecommendする。選択したarchitectural boundaryまたはtrade-offの
rationaleを永続化する必要があればADRをrecommendする。いずれも黙ったdecisionではない。

### 6. Namespaceとimplementation boundaryを保つ

Requirement IDはnormative specification ruleを表す。runtime diagnostic codeは観測されたfailureを表し、
`PROJECT-001` と `E-PROJECT-NOT-FOUND` のように明確に分けたnamespaceを使用しなければならない（MUST）。
Diagnostic codeをRequirement IDとして使用したり、既存のdiagnostic/test nameからrequirementを推測したりしてはならない。
既存code behaviorはinspectするevidenceであり、product ruleへ昇格するauthorityではない。

refinement中は、field name `id` がprimary keyを意味する、またはdiagnostic codeをRequirement IDとして扱うなど、
owning specificationに支えられていないcurrent behaviorを明示的に監査する。そのbehaviorはremoval/reportingの候補で
あり、approvedであることのevidenceではない。

## Required output

最初にsource evidenceとstatement classificationを示し、以下の見出しを正確に使用する。空のsectionには
None identified と記載する。

### Affected Specifications

- file、current status、affected Requirement ID。

### Confirmed Decisions

- 提供されたevidenceで支持される明示的なdecision/constraintだけを記載する。

### New Requirements

- stable IDと正確なMUST/SHOULD/MAY wordingを持つnew normative requirement。

### Changed Requirements

- existing ID、old meaningとproposed meaning、compatibility impact。

### Open Questions

- behaviorを変え得る、未回答のすべてのpoint。missing source contextも含める。暗黙に回答してはならない。

### Potential ADRs

- ADRに値する可能性があるarchitectural decision、または None identified。

### Compatibility Impact

- stable identity、serialized shape、generated API、file interpretation、migration implication。
  該当しない場合も明示する。

### Implementation Impact

- likely crate、GUI/Tauri boundary、.NET adapter、tests、fixtures、verification。これはplanでありimplementationではない。

最後にproposed document status（`Draft` または `Proposed`）と、まだ必要なhuman approval actionを記載する。
conciseなchange summaryも必須である。

## 絶対に外せない安全策

- conversationは恒久的なspecificationではない。
- 過去のstale memoryを使用しない。
- `Preference`、`Proposal`、`Idea`、`Question`、`Open Question` を明示的なdecisionなしにnormative behaviorへ昇格させない。
- `MAY` を `SHOULD` に、`SHOULD` を `MUST` に、可能性をrecommendationへ強めない。
- 未指定のdefaultとedge caseを追加しない。
- 1つのruleを複数のcanonical documentへ重複させない。
- conflicting specificationを黙って上書きしない。
- 承認成熟度の異なるrequirementを、splitでstatusを明確にできるのに同じcanonical fileへ混在させない。
- Approved/Implemented canonical documentへsemantic changeを混ぜず、別のchange artifactを使用する。
- `Accepted` RFCを `Approved` canonical specificationの代替にしない。
- runtime diagnostic code、test number、current behaviorを、明示的なevidenceなしにspecification requirementとして扱わない。
- Draft/Proposed specificationを自動的に `Approved` にしない。
- refinementの一部としてproduct featureを実装しない。
