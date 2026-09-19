# 仕様変更: Existing record key field edit

Status: Draft

## Affected Specifications

- `docs/gui/data-editor/spec.md` — `Status: Approved`
  - `GUI-DATA-STATE-001`
- `docs/specs/source-edit.md` — `Status: Approved`
  - `SOURCE-EDIT-001`, `SOURCE-EDIT-002`, `SOURCE-EDIT-004`, `SOURCE-EDIT-005`, `SOURCE-EDIT-008..016`
  - new candidate: `SOURCE-EDIT-017`
- scope decision次第で `docs/specs/authoring-batch.md` / `docs/gui/data-editor/grid-authoring.md` もaffectedになる。

## 根拠と分類（Source Evidence and Classification）

- **Decision / Human priority**: 2026-09-19、Desktop制作v1完了後の次priorityとしてP4を選択し、existing record key field editをP4-Aとして仕様化する。
- **Constraint / existing Approved contract**: `GUI-DATA-STATE-001`はbase snapshotに存在するexisting recordのPrimary / Secondary Key構成fieldをread-onlyとする。Added record draftだけは初回Save前にkey field入力を許す。
- **Constraint / existing Approved contract**: `SOURCE-EDIT-001`はedit targetをexact base snapshot + source provenanceで識別し、Primary Key valueだけでtargetを特定することを禁止する。
- **Constraint / existing Approved contract**: `SOURCE-EDIT-004`はdomain validation errorだけを理由にSaveを拒否しない。key uniqueness violation等が発生しても、このcontractを変更する明示decisionがない限りvalidationとsource Saveを分離する。
- **Constraint / existing Approved contract**: `SOURCE-EDIT-005..016`のsource preservation、lossless typed value、lost-update preflight、result lifecycle、host boundaryを維持する。
- **Constraint / terminology**: recordのkey field value editとMessagePack field `key`、Primary / Secondary Key declaration mutationは別conceptである。
- **Open Question**: P4-AがPrimary Key構成fieldだけを対象にするか、Secondary Key構成fieldも同じeditable scopeに含めるかは未決定。
- **Open Question**: single-cell editだけを許可するか、paste / fill / range Set Null等のbatch authoringにもkey fieldを含めるかは未決定。

## 提案する差分（Proposed Delta）

以下はHuman decisionでscopeが確定するまでDraft candidateであり、implementation authorityではない。

### SOURCE-EDIT-017（candidate）

ApprovedなGUI/application contractがexisting recordのkey構成field editを許可する場合、そのvalue editは通常のexisting `RecordValueEdit`と同じexact base snapshot、source provenance、resolved typed value、source-preserving candidate、file Save、lost-update preflight、result lifecycleを使用しなければならない（MUST）。

key value変更前後の値をrecord occurrence identityとして使用してはならず（MUST NOT）、edit targetをPrimary / Secondary Key valueだけで再検索してはならない（MUST NOT）。

key value editによってcurrent candidateがPrimary Key / unique Secondary Key等のdomain validationに違反しても、`SOURCE-EDIT-004`のvalidation / Save分離を変更してはならない（MUST NOT）。Save successはcandidateがvalidであることを意味しない。

### GUI-DATA-STATE-001（changed candidate）

現在の「existing recordのPrimary / Secondary Key構成fieldはread-only」というblanket ruleを、Humanが選択したP4-A scopeに限って解除する。

editable対象はshared applicationがresolved value shapeをlosslessにauthoring可能と報告するfieldに限り、unsupported / unresolved / unsafe source shapeはkey fieldであってもread-onlyを維持する。frontendがkey compatibilityやsource mutation safetyを独自判定してはならない。

Added record draftの既存exceptionと、Save後にexisting recordへ移行するlifecycleは維持する。

### Batch authoring

`Authoring Batch` / `GUI-GRID-001..006`へのdeltaは、key fieldをbatch対象へ含めるHuman decisionがあるまで追加しない。

## 互換性（Compatibility）

- YAML serialized shape、Table identity、MessagePack field `key`、Primary / Secondary Key declaration syntax、generated C# / binary formatは変更しない。
- existing record valueが変わるため、Build Selection後のlogical dataset、Primary Key / unique Secondary Key constraint、generated binary内容は利用者のsource edit結果として変化し得る。
- target occurrence identityはsource provenanceのまま維持し、変更前/変更後key valueをpersistent edit identityへ昇格させない。
- schema migrationやreleased binary compatibility policyはこのchangeで追加しない。

## 受け入れと実装への影響（Acceptance and Implementation Impact）

scope確定後、少なくとも次をevidence化する。

- `masterdata-core/src/source_edit.rs`の現在のnon-key-only editable field filterを、Approved scopeに合わせて変更し、source provenanceだけでtarget occurrenceを保持する。
- `masterdata-app/src/authoring.rs`のcolumn/cell capabilityをshared ruleから返し、frontendへkey edit semanticsを複製しない。
- Data EditorでApproved scopeのexisting key fieldだけがeditableになり、unsupported shapeはread-onlyを維持する。
- key value edit後にduplicate Primary Key / unique Secondary Key等のdiagnosticが発生しても、validationだけを理由にSaveを拒否しない。
- Save / Conflict / explicit Overwrite / Failure / Outcome Unknown、external edit、Undo/Redoのexisting lifecycleを回帰する。
- batch scopeを採用した場合だけ、Authoring BatchとGrid Authoringへfocused regressionを追加する。

## 未解決事項（Open Questions）

1. **Key scope**: existing recordのPrimary Key構成fieldだけをeditableにするか、Primary + Secondary Key構成fieldを対象にするか。
2. **Batch scope**: P4-A initial sliceはsingle-cell editに限定するか、paste / fill / range Set Null等も同時に許可するか。
3. **UX warning**: key edit開始時またはSave時に、record lookup / ordering / uniquenessへ影響し得ることを追加confirmation / warningとして要求するか。既存contractからは決まらない。

これらはobservable behaviorを変えるため、Human decisionなしに解決しない。

## レビュー（Review）

Pending. Draft refinement完了後に`review-spec`で独立確認する。

## 承認記録（Approval Record）

未承認。
