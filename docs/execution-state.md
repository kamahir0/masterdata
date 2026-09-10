# Development State

Stage: decision-required
Candidate: none

## Blocking findings

None.

## Human decision needed

[最初のGUIレコード編集・保存体験RFC](rfcs/0005-first-record-authoring-experience.md)の比較案を選ぶ。

- OQ-A: 初期編集対象をRequired Primitiveの非key fieldに絞るか、Enum / Value Object / Nullable等まで含めるか。
- OQ-B: 1 record単位の明示Saveと保存前source diff、無関係なtextを保持する保存方針を採るか。
- OQ-C: invalidな変更値は保存不可、無関係な既存errorは保存を妨げない、stale拒否、dirty時の保存／破棄／キャンセルという推奨packageを採るか。

回答はRFCへ反映し、採用範囲のGUI / shared source-edit仕様をDraft化してreviewする。
この選択だけでcanonical仕様をApprovedにせず、残るobservable detailとHuman Approvalを閉じてからimplementation-readyへ進む。
