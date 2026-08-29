# AGENTS.md

このrepositoryで作業するAI agentと開発者向けのルールです。

## Before changing code

- 実装前に関連する `docs/specs` と `docs/adr` を読む。
- `docs/specs` の `Status: Approved` または `Status: Implemented` な仕様を
  domain behaviorの正本として優先する。Draft/Proposedは確定仕様として
  扱わない。
- domain semanticsを変更する場合は、同じ変更で仕様書も更新する。
- public behaviorを追加・変更したら、対応するtestを追加または更新する。

## Architecture rules

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

## Specification workflow

- Approved specs are authoritative for domain behavior. Conversation alone is
  not permanent specification.
- Significant domain or public behavior changes MUST go through
  `refine-spec`, then `review-spec`, then explicit human approval before
  implementation.
- `Status:` is file-wide. Canonical specification files SHOULD contain
  requirements that can progress through Draft/Proposed/Approved/Implemented
  together. When maturity diverges, split the file without renaming or
  reassigning existing Requirement IDs; use directory `README.md` files only
  as non-canonical indexes.
- Do not promote proposals, preferences, ideas, or questions to approved
  behavior without evidence. Preserve MUST / SHOULD / MAY strength and keep
  unresolved decisions as Open Questions.
- Use `implement-spec` only from an explicitly Approved specification. If
  implementation exposes a specification gap, report it and return to
  refinement instead of silently inventing behavior.
- Keep ADRs for architectural decisions and keep one canonical owner for each
  normative rule; link to it instead of duplicating semantics.
- GUI behavior is also specification-worthy. Keep GUI requirements at the
  adapter boundary and shared domain semantics in `masterdata-core`.
- Keep tests and fixtures synchronized with specifications where appropriate,
  and include requirement IDs in test names or nearby comments when useful.
- AI-generated Draft/Proposed text MUST NOT be changed to Approved
  automatically. `cargo xtask check-specs` is the lightweight integrity check
  and is included in `cargo xtask check-all`; it also checks numbered RFC and
  specification-change metadata.
- RFC `Accepted` is an RFC decision, not product-spec `Approved`. A
  specification-change artifact reaches `Applied` only after an explicit human
  approval and atomic canonical merge.

## Fixtures and workflow

- fixtureはテスト用の固定入力であり、CLI/GUI実行時に直接書き換えない。
- development projectは `target/dev-project` にfixtureからコピーする。
- 未実装機能を実装済みのように偽装しない。placeholder、status、error codeを明示する。
- shell scriptへ主要ロジックを分散させず、repository workflowは `cargo xtask` に集約する。
- 作業完了前に `cargo xtask check-all` を実行し、実行できない場合は理由を報告する。

## Completion checklist

1. 関連spec / ADRを更新したか
2. fixtureとtestを更新したか
3. `cargo fmt --all -- --check` が通るか
4. `cargo clippy` と `cargo test` が通るか
5. frontend checkとintegration smoke testが通るか
6. `cargo xtask check-all` の結果と未実装事項を報告したか

## Specification workflow guardrails

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
