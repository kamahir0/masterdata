# 仕様変更: Desktop制作v1 — P2 Workspace・Tag・設定

Status: Applied

## Affected Specifications

Applied canonical owners:

- [Source Tag Edit仕様](../specs/source-tag-edit.md): `SOURCE-TAG-001`〜`003`
- [Data Editor Tag Authoring](../gui/data-editor/tag-authoring.md): `GUI-TAG-001`
- [Authoring Query仕様](../specs/authoring-query.md): `AUTHORING-OVERVIEW-001`〜`003`
- [Table Overview](../gui/table-overview/spec.md): `GUI-OVERVIEW-001`〜`002`
- [Project Config Edit仕様](../specs/project-config-edit.md): `CONFIG-EDIT-001`〜`004`
- [Project Settings](../gui/project-settings/spec.md): `GUI-SETTINGS-001`〜`003`

## 根拠と分類（Source Evidence and Classification）

- Decision: Desktop制作v1では横断viewを保存済みread-only snapshotとし、mutationはfile単位とする。
- Requirement: 分割TableとProfile selectionを把握し、Tag / Profile / Publish targetをGUIから編集する。
- Constraint: occurrenceとPKの分離、shared selection、source/config preservation、lost-update防止、Build / Publish非連動を維持する。
- Approval: 2026-09-18、Human maintainerが0016–0018を一括で明示Approvalした。

元のProposed全文とrefinement過程はGit historyに残す。本artifactは適用後のaudit recordでありimplementation authorityではない。

## 提案する差分（Proposed Delta）

承認済みdeltaは上記canonical ownerへ適用済み。Record Tag authoring、saved Table Overview / Profile preview、lossless TOML config editingとProject Settings lifecycleをApproved contractとして追加した。

Record Tagはdomain fieldではなくmetadataのまま、Profile selectionは既存Build Selection semanticsを使用する。旧Data Editor / Record Mutation文書中の`$tags` authoring非目標は、本Applied changeで上記Approved ownerにより置換された範囲ではauthorityではない。

## 互換性（Compatibility）

既存YAML / TOML canonical shape、tag selection formula、project identity、binary semanticsを変更しない。config editorはsource-preserving patch、exact-content conflict検出、validation non-blockingの境界を守る。

## 受け入れと実装への影響（Acceptance and Implementation Impact）

Tag source preservation、Overview snapshot/provenance、Profile preview、TOML lossless edit、config conflict / Outcome Unknown、Save All composition、keyboard/accessibilityをfocused evidenceで検証する。

実装は本artifactではなく上記Approved canonical ownerを入力とする。

## 未解決事項（Open Questions）

None within P2 scope. general TOML editor、Profile rename/delete、target remove/reorderは別package。

## レビュー（Review）

一括reviewでBlocking Issues: None、Non-blocking Issues: None、Questions: None。Build Selection、Source Edit、Project layout、GUI dirty/recovery contractとの整合を確認した。

## 承認記録（Approval Record）

2026-09-18 Human Approval。0016–0018を一括承認し、canonicalへ適用してimplementation-readyへ進める指示を受領。Statusを`Applied`とした。
