# 仕様変更: Existing record key field edit

Status: Proposed

## Affected Specifications

- `docs/specs/source-edit.md` — `Status: Approved`
  - new requirement: `SOURCE-EDIT-017`
- `docs/gui/data-editor/spec.md` — `Status: Approved`
  - changed requirement: `GUI-DATA-STATE-001`
  - changed requirement: `GUI-DATA-EDIT-001`
  - initial non-goalからexisting record key field editを除外
- `docs/specs/authoring-batch.md` — `Status: Approved`
  - `AUTHORING-BATCH-001`のexisting key batch-edit禁止は変更しない
- `docs/gui/data-editor/grid-authoring.md` — `Status: Approved`
  - batch mutation contractは変更しない

## 根拠と分類（Source Evidence and Classification）

- **Decision / Human priority**: 2026-09-19、Desktop制作v1完了後の次priorityとしてP4を選択し、existing record key field editをP4-Aとして仕様化する。
- **Decision / Human scope**: existing recordのPrimary Key / Secondary Key構成fieldをdirect single-cell editの対象にする。
- **Decision / Human scope**: paste / fill / range Set Null等のAuthoring Batchはinitial scopeへ含めない。existing keyをbatch edit対象外とする`AUTHORING-BATCH-001`を維持する。
- **Decision / Human scope**: key edit専用の追加modal confirmationを必須にしない。通常のtyped cell edit / validation / Save lifecycleへ統合する。
- **Constraint / existing Approved contract**: `SOURCE-EDIT-001`はedit targetをexact base snapshot + source provenanceで識別し、Primary Key valueだけでtargetを特定することを禁止する。
- **Constraint / existing Approved contract**: `SOURCE-EDIT-004`はdomain validation errorだけを理由にSaveを拒否しない。
- **Constraint / existing Approved contract**: `SOURCE-EDIT-005..016`のsource preservation、lossless typed value、lost-update preflight、result lifecycle、host boundaryを維持する。
- **Constraint / terminology**: recordのkey field value editとMessagePack field `key`、Primary / Secondary Key declaration mutationは別conceptである。

## Confirmed Decisions

- Primary Key / Secondary Key構成fieldを同じexisting-record direct edit scopeへ含める。
- P4-Aではdirect single-cell editだけを追加し、existing keyへのbatch mutationは追加しない。
- key edit専用のmodal confirmationをSaveやedit確定の前提にしない。
- target occurrence identity、validation / Save分離、source-preserving patch、Conflict lifecycleは既存Source Record Edit contractを維持する。

## New Requirements

### SOURCE-EDIT-017

既存recordのPrimary KeyまたはSecondary Keyを構成するfield valueを編集する場合、そのeditは通常のexisting record value editと同じexact base snapshot、source provenance、resolved typed value、source-preserving candidate、file単位Save、lost-update preflight、および`Success / Conflict / Failure / Outcome Unknown` lifecycleを使用しなければならない（MUST）。

変更前または変更後のPrimary / Secondary Key valueをrecord occurrence identityとして使用してはならず（MUST NOT）、edit targetをkey valueだけで再検索または再特定してはならない（MUST NOT）。

key value editによってcurrent candidateがPrimary Key uniqueness、unique Secondary Key、またはその他のdomain validationに違反しても、`SOURCE-EDIT-004`のvalidation / Save分離を変更してはならない（MUST NOT）。source commit safetyを満たす限り、validation errorだけを理由にSaveを拒否してはならない（MUST NOT）。Save successはcandidateがdomain-validであることを意味しない。

本requirementはMessagePack field `key`、`primaryKey.fields`、`secondaryKeys` declarationのschema mutationを許可しない（MUST NOT）。

## Changed Requirements

### GUI-DATA-STATE-001

base snapshotに存在するexisting recordでは、shared applicationが`SOURCE-EDIT-015` / `SOURCE-EDIT-016`に基づくsupported resolved value shapeとして安全にauthoring可能と報告するfieldを、Primary / Secondary Key membershipだけを理由にread-onlyとしてはならない（MUST NOT）。Primitive、Value Object、normal Enum等、key componentとしてApprovedなshapeを含め、resolved value shapeに従ってeditableとして扱わなければならない（MUST）。

unsupported、unresolved、missing source member、またはsource shapeをlosslessにtyped authoring stateへ投影できないfieldは、key membershipにかかわらずread-onlyとして扱い、その理由をData Editorから確認できなければならない（MUST）。unsupported fieldを含むTable全体を非表示にしてはならない（MUST NOT）。Primary / Secondary Key構成fieldはeditableになった後もkey membershipを利用者が識別できなければならない（MUST）。

base snapshotに存在しないAdded record draftは既存Record Mutation contractのediting scopeを維持する。Added draftがSave成功してexisting recordになった後も、上記existing-record ruleに従う。

### GUI-DATA-EDIT-001

利用者は`GUI-DATA-STATE-001`でeditableなcellをdirect single-cell editingから変更できなければならない（MUST）。Primary / Secondary Key構成fieldだけを理由に、key edit専用の追加modal confirmationをedit確定またはSaveの必須前提にしてはならない（MUST NOT）。

key membershipの表示、validation marker、Problemsへのdiagnostic表示は通常のData Editor contractに従う。key editを行ったことだけを理由にSave、Build、Publish、Migrationを自動実行してはならない（MUST NOT）。

### Authoring Batch boundary

`AUTHORING-BATCH-001`の「existing keyはbatch編集対象外」を維持する。したがってP4-Aのexisting key editabilityから、paste、fill、range Set Null、single-cell pasteを含むAuthoring Batchへのkey mutation permissionを導出してはならない（MUST NOT）。read-only keyのcopy permissionは既存`AUTHORING-BATCH-004`を変更しない。

## Open Questions

None identified for P4-A initial scope.

## Potential ADRs

None identified. shared Rust core/application ownership、Tauri thin adapter、YAML Source of Truthの既存architectureを変更しない。

## Compatibility Impact

- YAML serialized shape、Table identity、MessagePack field `key`、Primary / Secondary Key declaration syntax、generated API shape、binary formatを変更しない。
- 利用者がexisting record key valueを変更できるため、Build Selection後のlogical dataset、lookup value、canonical record ordering、uniqueness validation、生成binary contentはsource edit結果として変化し得る。
- target occurrence identityはsource provenanceのまま維持し、変更前/変更後key valueをpersistent edit identityへ昇格させない。
- existing sourceを編集しない限り互換性影響はない。schema migrationやreleased binary compatibility policyは追加しない。

## Implementation Impact

- `masterdata-core/src/source_edit.rs`: current non-key-only editable field filterを変更し、resolved supported fieldをkey membershipにかかわらずdirect source edit可能にする。batch permissionとは分離する。
- `masterdata-app/src/authoring.rs`: Data Editor column/cell capabilityをshared ruleから返し、key membershipをpresentation metadataとして保持しつつdirect editを許可する。
- React Data Editor: direct typed editへkey fieldを含め、key専用modalを追加しない。existing batch preview pathはkey targetを引き続き拒否する。
- Tests: PK / Secondary Key direct edit、composite key、duplicate uniqueness diagnostic + Save、same-key no-op、Conflict / Overwrite / Failure / Outcome Unknown、Undo/Redo、batch key rejectionをfocused regressionで保護する。
- .NET adapter、Build/Publish implementation、fixtures formatの変更は原則不要。必要なend-to-end evidenceはtemporary workspaceを使用する。

## レビュー（Review）

### Blocking Issues

None identified.

### Non-blocking Issues

- `GUI-DATA-STATE-001`はread-onlyからeditableへのmaterial semantic changeだが、同RequirementはData Editorのfield editability boundaryを継続して所有し、旧meaningは本change artifactに履歴として残るためIDを維持する。
- batch pathは`AUTHORING-BATCH-001`でexisting keyを明示的に除外しているため、direct edit permissionがsingle-cell pasteへ漏れないことをimplementation regressionで固定する必要がある。

### Questions

None identified.

### Approved as Proposed

**Yes**。Human-selected A2 + B1 + C1を正確に反映し、Source Record Editのprovenance / source preservation / validation-save separationを維持したままdirect key editだけを追加している。Human Approvalとcanonical applicationはまだ必要。

Review dimensions: Intent fidelity=Pass、Internal consistency=Pass、Cross-spec consistency=Pass、Terminology=Pass、Normative strength=Pass、Testability=Pass、Backward compatibility=Pass with explicit source-data impact、Unresolved ambiguity=None、Implementation leakage=None identified、Unrequested behavior=None identified。

## 承認記録（Approval Record）

未承認。Human maintainerによる明示Approval後にのみcanonical specificationへ適用する。
