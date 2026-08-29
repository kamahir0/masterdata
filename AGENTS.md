# AGENTS.md

このrepositoryで作業するAI agentと開発者向けのルールです。

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

## 完了チェックリスト

1. 関連spec / ADRを更新したか
2. fixtureとtestを更新したか
3. `cargo fmt --all -- --check` が通るか
4. `cargo clippy` と `cargo test` が通るか
5. frontend checkとintegration smoke testが通るか
6. `cargo xtask check-all` の結果と未実装事項を報告したか

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
