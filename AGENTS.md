# AGENTS.md

このrepositoryで作業するAI agentと開発者向けのルールです。

## Before changing code

- 実装前に関連する `docs/specs` と `docs/adr` を読む。
- `docs/specs` の `Status: Approved` な仕様を優先する。Draftは確定仕様として扱わない。
- domain semanticsを変更する場合は、同じ変更で仕様書も更新する。
- public behaviorを追加・変更したら、対応するtestを追加または更新する。

## Architecture rules

- CLIとGUIは `masterdata-core` を共有し、domain logicを重複させない。
- GUIからCLIをsubprocess起動してdomain処理を行わない。
- GUI側にfilesystem探索やYAMLの意味解釈を実装しない。Tauri command経由でcoreを呼ぶ。
- MasterMemory internals、binary format、Source GeneratorをRustで再実装しない。
- .NET process invocationは `masterdata-dotnet` のadapterに集約する。
- YAMLのfile/directory locationにsemantic meaningを追加しない。`kind`、`table`、schema fieldsを正本とする。
- schema/type/index/referenceに関するarchitectural decisionを変更するときはADRを追加または更新する。
- 将来のSchema ASTを単なる `HashMap<String, serde_yaml::Value>` に固定しない。
- C# code generationを巨大なstring concat一関数に押し込まない。

## Specification workflow

- Approved specs are authoritative for domain behavior. Conversation alone is
  not permanent specification.
- Significant domain or public behavior changes MUST go through
  `refine-spec`, then `review-spec`, then explicit human approval before
  implementation.
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
  and is included in `cargo xtask check-all`.

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
