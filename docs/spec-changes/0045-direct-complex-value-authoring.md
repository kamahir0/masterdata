# 仕様変更: Complex Valueの直接・可逆な編集

Status: Applied

## Affected Specifications

- `docs/gui/data-editor/spec.md` (`GUI-DATA-EDIT-003`, 新規`GUI-DATA-EDIT-004`)
- `docs/gui/data-editor/grid-authoring.md` (`GUI-GRID-004..006`)
- `docs/gui/table-editor/spec.md` (`GUI-UNIFIED-005..006`、既存契約を維持)

## 根拠と分類（Source Evidence and Classification）

- **Human Requirement:** Complex Valueの意味あるcontrol操作はlocal bufferへ直ちに反映し、操作ごとにfile-local Undo/Redo可能にする。編集面全体の`Apply to buffer` / `Cancel`依存をなくす。nested scalarのtypingは確定まで一時入力とする。
- **Human Constraint:** close / outside clickは確定済み操作を取り消さない。unknown / invalid source値、source preservation、0044 Save、grid geometry、keyboard/focus、Problems navigationを維持する。
- **Approved authority:** `GUI-GRID-004`はcomplex control一操作をhistory単位とし、`GUI-GRID-006`はtext controlのlocal Undo優先、`GUI-DATA-EDIT-003`はresolved authoring shapeを使用する。
- **Desktop friction inventory (変更前、disposable showcase):** Array `[]`はopen→Add→item入力→Applyの4段階で、outside clickでは全変更が消えた。Customのnested Array moveもmenu選択後にApplyが必要。Flags toggleもApply待ち。Custom materialize後はfocusがeditor内の次controlへ移らなかった。Nullable primitiveとEnumはinline編集であり、同じsession footerを使わない。Added RowのArray / Customは同じValueEditorだが、materializeしてもApply待ち。追加行のnullはinvalidとして表示され、元値は開く時点では置換されなかった。nested Customとunknown source memberは表示される。schema不一致の既存Array scalarは元値とdiagnosticを表示するが、snapshotがcellをread-onlyにするためGUIからrepair不能だった。

## 提案する差分（Proposed Delta）

`GUI-DATA-EDIT-004`を追加する。

1. Complex editorのcheckbox、selection、materialize、null切替、Array add/move/remove、明示repairは、実際に値が変わるとき、その一操作を対象physical sourceのlocal bufferとhistoryへ確定しなければならない（MUST）。editor close、outside click、Escapeは既確定操作をrollbackしてはならない（MUST NOT）。no-opはhistoryを増やさない。
2. nested scalarのtypingはcontrol内で一時保持し、Enter / Tab / blurで一回確定する。Escapeはその未確定typingを破棄し、未確定typingがない場合はeditorを閉じる。Cmd/Ctrl+Sは未確定typingを先に確定してから通常のcurrent Table context Saveを行う（MUST）。text control内のnative Undoをfile historyが横取りしない。
3. Arrayのnull、空sequence、要素を区別し、unknown / invalid値とCustomのunknown memberを明示操作なしに置換・削除しない（MUST）。source valueがlosslessにauthoring stateへproject可能で、source-preserving candidateを構成できる場合、schema不一致だけを理由に既存cellの明示repair操作をread-onlyにしてはならない（MUST NOT）。project不能またはlocalization不能なら安全理由を示して拒否する。nested sequence provenanceをoperation historyで維持する。意味が変わらない操作やeditorを開くだけの行為はdirtyにしない。
4. existing record / Added Rowへ同じresolved ValueEditorを適用し、pointerとkeyboardによるopen、内部移動、secondary action、close後のorigin cell復帰、Problemsから一意なnested controlへのfocusを維持する（MUST）。complex editorの開閉・操作・diagnostic変化は通常grid geometryを変えない。

`GUI-GRID-006`の「Escapeはactive edit cancelを優先」は、scalar cellの未確定入力とcomplex editor内の未確定nested textに適用し、既にbufferへ確定したcomplex control操作の取消には適用しないと明確化する。`GUI-GRID-004..005`のhistoryと`GUI-UNIFIED-005..006`のsource-local / stable-surface契約は維持する。

## Confirmed Decisions / Agent Decision

- **Agent Decision:** nested scalarのblurはcommitとする。既存scalar cellのblur確定と一致し、outside clickで入力だけが消える意外性を避ける。Escapeはまずその入力のみをcancelし、次のEscapeでeditor closeとする。focusを失うだけで全sessionを取り消す現行挙動は採らない。
- **Agent Decision:** footerのtransactional pairを廃し、必要ならcloseのみをsecondary actionとして置く。変更を戻す通常経路はfile-local Undoとする。

## 互換性（Compatibility）

GUI interactionと未保存履歴の粒度のみ変更する。source format、shared Rust interpretation、physical dirty ownership、Save / Migration safetyを変更しない。既存の`Apply`を最終commitとして利用していた操作は、各control確定時にdirtyとなる。

## 受け入れと実装への影響（Acceptance and Implementation Impact）

Frontend ValueEditor / grid編集状態とhistory dispatch、およびshared Coreのsnapshot authoring eligibilityを変更する。Array null/empty/items、move/remove/Undo/Redo、Nullable、Enum unknown、Flags unknown、Custom unknown/nested、Added Row、invalid repair、outside click、text Escape / Save、Problems focus、grid位置をfocused regressionとDesktopで確認する。Coreのsemantic interpretation authorityは増やさない。

## 未解決事項（Open Questions）

None.

## レビュー（Review）

Fresh challenge pass（提案確定後）: Blocking Issues: None identified。Non-blocking Issues: Desktopの高負荷・nested diagnostic focusは実装検証で確認する。Questions: None identified。Approved as Proposed: Yes。Intent fidelity、Approved Grid history / text precedence、Save境界、source safety、testability、compatibility、documentation ownershipを照合。`GUI-GRID-006`のEscape文言だけは今回の規範要件で対象を明確化する。Autonomous approval eligibility: Eligible Yes; Human gate None。理由: Human-selected Objective内のreversible GUI interactionで、source formatやsafety authorityを変更しない。

## 承認記録（Approval Record）

Approval mode: Agent-autonomous。Basis: Human-selected Complex Value Objectiveと上記review。2026-09-28に`GUI-DATA-EDIT-004`、`GUI-GRID-006`へ適用。提案本文とreviewを本artifactに保持する。
