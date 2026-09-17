# 仕様変更: Desktop制作v1 — P1 日常編集

Status: Applied

## Affected Specifications

Applied canonical owners:

- [Authoring Batch仕様](../specs/authoring-batch.md): `AUTHORING-BATCH-001`〜`004`
- [Authoring Query仕様](../specs/authoring-query.md): `AUTHORING-QUERY-001`〜`003`
- [Data Editor Grid Authoring](../gui/data-editor/grid-authoring.md): `GUI-GRID-001`〜`006`
- [Typed Migration Initializer](../gui/typed-initializer.md): `GUI-TABLE-INT-008`, `GUI-TYPE-INT-010`

## 根拠と分類（Source Evidence and Classification）

- Decision: 2026-09-16、HumanはRFC 0008 Option B / P1–P3、file単位編集、保存前Undo、scalar一括入力、計算列の後段化を選択した。
- Requirement: raw YAMLへ戻らず、少数入力から大量調整まで日常authoringを完遂できること。
- Constraint: source preservation、exact occurrence、lossless 64-bit transport、validation non-blocking、existing key read-only、shared Rust semanticsを維持する。
- Approval: 2026-09-18、Human maintainerが0016–0018を一括で明示Approvalした。

元のProposed全文とrefinement過程はGit historyに残す。本artifactは適用後のaudit recordでありimplementation authorityではない。

## 提案する差分（Proposed Delta）

承認済みdeltaは上記canonical ownerへ適用済み。scalar batch / TSV codec、search/filter/sort、range preview、file-local Undo/Redo、typed Migration initializerをApproved contractとして追加した。

既存Data Editor / Record Mutationのsingle-cell、source順、Undo Delete等のcontractは維持する。旧文書中の「range / general Undo/Redoを初期scopeに含めない」というnon-goalは、本Applied changeで上記Approved ownerにより置換された範囲ではauthorityではない。

## 互換性（Compatibility）

source schema、serialized shape、generated API、CLI grammarを変更しない。batch/query/historyはauthoring/view semanticsであり、canonical orderingやdomain comparison capabilityを暗黙拡張しない。

## 受け入れと実装への影響（Acceptance and Implementation Impact）

codec、64-bit、invalid/null、all-or-none batch、stale preview、query composition、history lifecycle、keyboard/IME、typed initializerをfocused core/application/GUI evidenceで検証する。

実装は本artifactではなく上記Approved canonical ownerを入力とする。

## 未解決事項（Open Questions）

None within P1 scope. P5 expression / computed viewは別package。

## レビュー（Review）

一括reviewでBlocking Issues: None、Non-blocking Issues: None、Questions: None。既存Source Edit / Record Mutation / Type System / Migration contractとの整合を確認した。

## 承認記録（Approval Record）

2026-09-18 Human Approval。0016–0018を一括承認し、canonicalへ適用してimplementation-readyへ進める指示を受領。Statusを`Applied`とした。
