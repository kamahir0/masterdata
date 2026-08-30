# AGENTS.md

このrepositoryで作業するAI agentと開発者向けのルールです。

## ドキュメント言語

- repository内の人間向けドキュメントは、特別な理由がない限り日本語で記述する（SHOULD）。対象には `README.md`、`docs/` 配下のspecification、ADR、RFC、contributing guide、設計メモ、およびAI agentが生成・更新する説明文を含む。
- 新規ドキュメントは原則として日本語で作成する。既存ドキュメントを実質的に編集する場合も、変更する説明部分は可能な範囲で日本語へ揃える。
- identifier、Requirement ID、Diagnostic Code、API/type/function名、CLI flag、file path、code、configuration key、language keyword、protocol/library/productの正式名称など、英語のままの方が正確なtechnical tokenは翻訳しなくてよい。
- MUST / SHOULD / MAY、Draft / Proposed / Approved / Implemented、Specification Gapなど、repositoryで意味を固定しているworkflow用語は既存表記を維持してよい。
- 外部仕様やerror messageを引用する場合は原文を保持してよいが、必要な説明は日本語で付ける。
- 日本語化のためだけにtechnical meaning、検索性、既存のstable identifier、またはcanonical terminologyを変更してはならない（MUST NOT）。
- user-facing product textのlanguage policyは、このドキュメント記述ルールとは別に仕様で定義する。

## コードを変更する前に

- 実装前に関連する `docs/specs` と `docs/adr` を読む。
- `docs/specs` の `Status: Approved` または `Status: Implemented` な仕様を
  domain behaviorの正本として優先する。Draft/Proposedは確定仕様として
  扱わない。
- domain semanticsを変更する場合は、同じ変更で仕様書も更新する。
- public behaviorを追加・変更したら、対応するtestを追加または更新する。

## アーキテクチャ規則

- CLIとGUIは `masterdata-app` のapplication workflowと
  `masterdata-core` を共有し、domain logicを重複させない。
- GUIからCLIをsubprocess起動してdomain処理を行わない。
- GUI側にfilesystem探索やYAMLの意味解釈を実装しない。Tauri command経由で
  application service/coreを呼ぶ。
- MasterMemory internals、binary format、Source GeneratorをRustで再実装しない。
- .NET process invocationは `masterdata-dotnet` のadapterに集約する。
- Requirement ID（例: `PROJECT-001`）とruntime Diagnostic Code（例:
  `E-PROJECT-NOT-FOUND`）を混同しない。
- YAMLのfile/directory locationにsemantic meaningを追加しない。`kind`、`table`、schema fieldsを正本とする。
- schema/type/index/referenceに関するarchitectural decisionを変更するときはADRを追加または更新する。
- Approved/Implemented canonical specへのsemantic changeは、先に
  `docs/spec-changes/` またはRFCへ隔離し、review-specと明示的な人間の
  承認を経てatomicに反映する。canonical specへ未承認変更を混在させない。
- 将来のSchema ASTを単なる `HashMap<String, serde_yaml::Value>` に固定しない。
- C# code generationを巨大なstring concat一関数に押し込まない。

## 仕様ワークフロー

- Approved仕様はdomain behaviorのauthorityであり、会話だけでは恒久的な仕様に
  ならない。
- domainまたはpublic behaviorの重要な変更は、実装前に
  `refine-spec`、`review-spec`、明示的な人間による承認の順を経なければ
  ならない（MUST）。
- `Status:` はファイル全体に適用される。canonical specification fileには、
  Draft/Proposed/Approved/Implementedを一緒に進められる要件を含めるべきで
  ある（SHOULD）。成熟度が分かれる場合は、既存のRequirement IDをrenameまたは
  reassignせずにファイルを分割する。ディレクトリの `README.md` は
  non-canonicalなindexとしてのみ使用する。
- evidenceなしにProposal、Preference、Idea、QuestionをApproved behaviorへ
  昇格させてはならない。MUST / SHOULD / MAYの強度を保ち、未解決の判断は
  Open Questionとして残す。
- `implement-spec` は明示的にApprovedとなった仕様からのみ使用する。実装が
  specification gapを示した場合は報告し、黙ってbehaviorを発明せずrefinementへ
  戻す。
- 実装変更が完了したら、rationale-sensitiveな変更については`review-code`を実行し、
  `check-rationale`または同等の構造参照checkと`cargo xtask check-all`を通す。
- architectural decisionはADRに残し、各normative ruleにはcanonical ownerを
  1つだけ置く。意味を重複させず、ownerへlinkする。
- GUI behaviorも仕様化の対象である。GUI要件はadapter boundaryに置き、共有する
  domain semanticsは `masterdata-core` に置く。
- 適切な範囲でtestsとfixturesを仕様に同期させ、traceabilityに有用ならtest name
  または近接するcommentへRequirement IDを含める。
- AIが生成したDraft/Proposed textをApprovedへ自動変更してはならない（MUST NOT）。
  `cargo xtask check-specs` は軽量なintegrity checkであり、
  `cargo xtask check-all` に含まれる。また、番号付きRFCとspecification-changeの
  metadataも検査する。
- RFCの `Accepted` はRFC上のdecisionであり、product-specの `Approved` ではない。
  specification-change artifactは、明示的な人間による承認とcanonical mergeが
  atomicに完了した後にのみ `Applied` となる。

## Fixtureとワークフロー

- fixtureはテスト用の固定入力であり、CLI/GUI実行時に直接書き換えない。
- development projectは `target/dev-project` にfixtureからコピーする。
- 未実装機能を実装済みのように偽装しない。placeholder、status、error codeを明示する。
- shell scriptへ主要ロジックを分散させず、repository workflowは `cargo xtask` に集約する。
- 作業完了前に `cargo xtask check-all` を実行し、実行できない場合は理由を報告する。

## Git上の文章と説明

Git historyとGitHub上の説明は、後から変更理由と検証根拠を復元できるdurable evidenceとして扱う。

- commit title、commit body、Pull Requestのtitle/body、Issueのtitle/body、review summary、およびAI agentのrepository作業完了報告は、特別な理由がない限り日本語で記述しなければならない（MUST）。technical token、identifier、code、外部error messageなどは「ドキュメント言語」の例外に従い原文を保持してよい。
- commit titleは変更によって達成した結果を日本語で簡潔に要約しなければならない（MUST）。`update files`、`fix stuff`のように内容を特定できないtitleを使用してはならない（MUST NOT）。tooling上の要求がない限り、`feat:`、`fix:`、`docs:`のような英語prefixを慣習だけで付けない。
- AI agentがrepository変更をcommitする場合、変更が小さくてもcommit bodyを省略してはならない（MUST NOT）。bodyは最低限、`背景/目的`、`変更内容`、`検証`を日本語で説明しなければならない（MUST）。semantic change、architecture decision、compatibility impact、既知の制約、残課題がある場合は、それらも記載する。
- commit bodyはdiffの逐語的な再説明ではなく、「なぜ必要だったか」「何を変えたか」「何を確認したか」を将来の開発者が理解できる内容にする。関連するRequirement ID、ADR/RFC、issue、test、CIなどがtraceabilityに有用なら参照する。
- Pull Requestを作成または更新する場合、bodyには最低限、目的、主要な変更、検証結果、未解決事項またはriskを日本語で記載する。単にcommit一覧やdiffを貼るだけの説明にしてはならない（MUST NOT）。
- review結果やAI agentの完了報告では、結論だけでなく、重要な判断理由、実行したcheck、未検証事項、commit SHA、push結果、CI statusを必要な範囲で日本語で報告する。
- commit messageやPR説明を充実させるために、存在しない検証結果、未実施test、未確認のrationaleを記載してはならない（MUST NOT）。

推奨するAI agent commit形式:

```text
<変更結果を表す日本語のtitle>

背景/目的:
<なぜこの変更が必要か>

変更内容:
- <主要変更1>
- <主要変更2>

検証:
- <実行したcheckと結果>

関連/影響:
- <必要な場合のみRequirement ID、ADR、互換性、残課題など>
```

## Git完了ポリシー

repositoryを変更するtaskでは、人間が明示的に `commitしない`、`pushしない`、または同等の指示をした場合を除き、
成功したtaskの通常の完了条件にcommitとpushを含める。`refine-spec`、`implement-spec`など個別skillはこのpolicyを継承する。
review/reportだけでrepository差分がないtaskではempty commitを作らない。

- task固有のrequired reviewとlocal checkを完了し、必要なcheckが成功した後にcommitする。
- commit前にworking treeとdiffを確認し、task scopeの変更だけをcommitする。unrelatedな既存変更を混ぜてはならない。
- unrelatedなdirty changeを安全に分離できない場合、check failure、merge conflict、またはtaskを安全に完了できない
  Specification Gapがある場合は、自動commit/pushを行わず理由を報告する。
- Draft/Proposed specificationがOpen Questionを正しく保持したまま`refine-spec`として完了する場合は、それ自体を
  failureとみなさない。Open Questionを黙って解決せず、review可能なspec差分としてcommit/pushしてよい。
- commit messageは「Git上の文章と説明」に従い、日本語のtitleと十分なbodyを持たせる。
- push先は現在のworking branchだけとし、通常のfast-forward pushを使う。自動でbranchを切り替えたり、
  force-push、history rewrite、rebaseによる公開historyの書き換えを行ってはならない（MUST NOT）。
- pushがbranch protection、permission、non-fast-forwardなどで拒否された場合は迂回せず報告する。
- push後のremote CIは原則として非同期のverificationとして扱う。人間がCI完了確認を明示的に要求した場合、または
  workflow上remote greenが状態遷移のgateとして明示されている場合を除き、CI完了をpollして待たず、commit SHAと
  push結果、CIがpending/runningである旨を報告してtaskを完了する。
- commit/pushはHuman Approvalを意味しない。Draft/Proposedをpushしても、Approvedへのstatus transitionは従来どおり
  明示的な人間の承認を必要とする。

## 完了チェックリスト

1. 関連spec / ADRを更新したか
2. fixtureとtestを更新したか
3. `cargo fmt --all -- --check` が通るか
4. `cargo clippy` と `cargo test` が通るか
5. frontend checkとintegration smoke testが通るか
6. rationale-sensitiveな変更では、影響するrationaleとevidenceを再検証したか
7. `cargo xtask check-all` の結果と未実装事項を報告したか
8. repository差分があるtaskでは、scope内の変更だけをcommitしたか
9. 自動pushが許可されるtaskではcurrent working branchへpushし、commit SHAとpush結果を報告したか
10. commit title/body、PR説明、完了報告が「Git上の文章と説明」に従い、変更理由と検証結果を十分に残しているか

## 仕様ワークフローのガードレール

- 会話は仕様の証拠であり、永久的な仕様ではない。`refine-spec` で発言を
  Decision / Requirement / Constraint / Preference / Proposal / Idea /
  Question / Open Question / Rejectedに分類する。
- `MAY`を`SHOULD`や`MUST`へ強めず、未指定のdefault・edge case・nullability・
  error policyを勝手に確定しない。不明点はOpen Questionまたは
  Specification Gapとして残す。
- `review-spec` は `implement-spec` の前に実行する。AI-generated Draftを
  自動でApprovedへ変更しない。
- Project/domain behaviorの実装はApproved canonical specから開始する。
  implementationで仕様の穴を見つけた場合、既存コードを正本にせず
  refine-specへ戻す。
- `cargo xtask check-specs` はRequirement IDのdefinitionとreferenceを区別
  し、duplicate definition、malformed ID/status、duplicate ADR/RFC/proposal
  number、change metadata、broken linkを確認する。

## 実装理由とReverse Traceability

- straightforwardな実装から意図的に外れたnon-obvious codeは、将来その理由と保護している
  invariantを復元できるだけのrationaleを保持しなければならない（MUST）。
- unusual、冗長に見える、削除・簡略化できそうなcodeを変更する前に、nearby rationale
  comment、Requirement ID、regression test、ADR、issue/reference、benchmark、
  platform/library/toolchain constraintを検索しなければならない（MUST）。
- 理由を確認せず `looks unnecessary -> delete` と進めてはならない（MUST NOT）。
- non-obvious codeを追加する場合は、必要に応じて`WHY`、削除・簡略化した場合のfailure mode、
  `EVIDENCE / REFERENCE`、`REMOVAL CONDITION`をprotected invariantの近くへ残す。reference
  だけで理由を置き換えてはならない（MUST NOT）。
- `Spec`はobservable behavior、`ADR`はarchitecture、`Test`はregression evidence、
  `Comment`はlocal implementation rationaleを所有する。rationale commentを新しいproduct
  requirementへ自動昇格させてはならない（MUST NOT）。
- Approved specがすでにbehaviorを定義しimplementationだけが違反している場合は、specを
  変更せずbug fixとfocused regression testを行う。behaviorを選択する必要がある場合は
  `Specification Gap`として`refine-spec`へ戻す。
- refactorでcodeの場所が移動する場合、rationaleはprotected invariantとともに移動しなければ
  ならない（MUST）。詳細は[実装理由ガイド](docs/contributing/implementation-rationale.md)を参照する。

## 実装変更時のRationale Freshness

- nearby implementation rationaleがあるcodeを変更した場合、そのrationaleを同じ変更内で再検証しなければ
  ならない（MUST）。結果は、正確なので保持、invariantまたは理由が変わったので更新、または理由が不要に
  なったので削除のいずれかにする。
- testが成功してもrationale commentが正確である証拠にはならない。逆に、正しいcommentだけでは必要な
  regression evidenceの代わりにならない。`Test`はbehavior、`Comment`はimplementation shapeの理由を
  別々に検証する。
- stale commentを、参照先のtestがまだ通るという理由だけで残してはならない（MUST NOT）。
- 実装diffの最終確認では`review-code`を使用する。`review-spec`はspecificationの正しさを、
  `review-code`は実装diff・rationale freshness・evidence integrity・architecture boundaryを担当する。
- `cargo xtask check-rationale`は、commentから明示されたRequirement ID、ADR/RFC、`Regression:` test name、
  repository-relative documentation pathの存在を、確認可能な範囲で検証する。commentの意味や鮮度を
  機械的に判定したことにしてはならない。
