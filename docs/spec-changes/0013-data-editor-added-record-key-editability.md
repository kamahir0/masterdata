# 仕様変更: Added record draftのkey field editability

Status: Applied

## Affected Specifications

- `docs/gui/data-editor/spec.md` — `Status: Approved`
  - `GUI-DATA-STATE-001`
- Related new specification: `docs/gui/data-editor/record-mutation.md` — `Status: Approved`

## 根拠と分類（Source Evidence and Classification）

- **Decision / Human priority**: Source Creation完了後、HumanはData Editorのrecord追加・削除を次priorityとして選択した。
- **Constraint**: `GUI-DATA-STATE-001`は現在、Data EditorでeditableなcellをRequired Primitive non-key fieldに限定している。
- **Requirement**: 新しいrecordをvalidな形で作るにはPrimary / Secondary Key構成fieldも初回値を入力する必要がある。existing record key mutationを許可せずにこれを実現するには、base snapshotにまだ存在しないAdded record draftだけにediting scopeを限定する必要がある。
- **Constraint**: Approved existing-record behaviorを新proposalから黙って上書きしてはならない。canonical requirementの適用範囲を明示的に更新する。

## 提案する差分（Proposed Delta）

`GUI-DATA-STATE-001`の意味を次のように更新する。

1. 現在の「初期sliceで編集可能なのはRequired Primitiveの非key field」という制約を、**base snapshotに存在するexisting recordの通常cell edit**に適用することを明示する。
2. Primary / Secondary Key構成field、Enum、Value Object、Nullable、Array、Custom Type等をexisting recordでread-only表示する現在のruleは維持する。
3. base snapshotに存在しないAdded record draftについては、別のApproved GUI specificationが初回Save前のediting scopeを定義してよい（MAY）ことを追加する。
4. このexceptionからexisting recordのkey field editabilityを導出してはならない（MUST NOT）。Added record draftがSave成功して新しいbase snapshotのexisting recordになった後は、通常の`GUI-DATA-STATE-001` scopeへ戻らなければならない（MUST）。

Canonical wording適用時は、`GUI-DATA-STATE-001`のRequirement IDを維持する。ruleの目的（existing sourceの初期typed edit scope）は変更せず、新規draftとのapplicability boundaryを明確化するためである。

## 互換性（Compatibility）

既存recordのeditable field範囲は変わらず、existing key fieldを編集可能にはしない。YAML、Table / Key semantics、generated C#、binary format、Save contractへの互換性impactはない。

新しく増えるbehaviorは、まだsourceに存在しないAdded record draftの初回field入力だけである。Save後は従来scopeへ戻るため、既存record authoringのobservable behaviorを拡張しない。

## 受け入れと実装への影響（Acceptance and Implementation Impact）

- Existing recordのkey cellが引き続きread-onlyであることを回帰testで確認する。
- Added record draftではPrimary / Secondary Key fieldを含むRequired Primitive fieldを入力できることをReact testで確認する。
- Added record draftをSaveした後、同じrowのkey fieldがexisting-record scopeとしてread-onlyになることを状態遷移testで確認する。
- shared domain/application boundaryではexisting-record `RecordValueEdit`とnew-record draft mutationを区別し、existing key editへfallbackしない。

## 未解決事項（Open Questions）

None.

## レビュー（Review）

Self-review: proposed deltaはexisting key mutationを許可せず、新しいrecordの初回definitionに必要な入力だけを明示的に分離する。`docs/gui/data-editor/record-mutation.md`のrequirementsと整合し、`GUI-DATA-STATE-001`以外のApproved Data Editor behaviorを変更しない。

Human approval後、`docs/gui/data-editor/spec.md`の`GUI-DATA-STATE-001`へdeltaを適用した。canonical requirement IDは維持し、existing recordのread-only key behaviorも維持した。

## 承認記録（Approval Record）

- Human approval: 2026-09-12
- Canonical application commit: `4e36a20c79b3bc570885bfded674313707d5bd99`
