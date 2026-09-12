# Source Creation実装とself-review

本書は実装根拠と検証範囲を記録するnon-normativeなメモである。
behaviorの正本は[Source Artifact Creation](../specs/source-creation.md)と
[GUI Source Creation](../gui/source-creation/spec.md)であり、両仕様のStatusはApprovedのまま維持する。

## Scope / Specification Conformance

Current Objectiveのsingle-artifact creationを実装した。Table / Data / Value Object /
Enum / Flags / Custom Typeのtyped requestとYAML構築はcore、filesystem操作と既存生成名との
衝突検査はapplication、formとExplorerの更新はGUIが担当する。
Folderも1回に1つだけ作成し、既存sourceの編集、record追加、Build / Publish / Gitは開始しない。
作成後のschema/type専用editorは対象外であり、既存unsupported表示へ渡す。

## Tests and Regression Evidence

| 対象 | 実装 / 回帰evidence |
| --- | --- |
| SOURCE-CREATE-004〜011: 各category、deterministic rendering、参照解決、無関係な既存errorの分離 | `crates/masterdata-core/tests/source_creation.rs`。全categoryのroundtrip、64bit enum値、複合キー順序、modifier、invalid dependencyを確認 |
| SOURCE-CREATE-001〜003、012〜017: 保存先、安全な作成、競合、再確認、既存source保持 | `crates/masterdata-app/tests/source_creation.rs`。folder/file作成、path escape、symlink、既存entry、並行作成、read-only recheckを確認 |
| SOURCE-CREATE-012〜014: preflight後の競合と失敗時のpartial防止 | `creation.rs`内の`target_created_after_staging_is_never_overwritten`、`staged_io_failure_leaves_no_partial_destination` |
| GUI-CREATE-ERR-001: 不正なtyped inputの分類 | Tauriの`malformed_creation_input_is_a_structured_preflight_rejection`。空欄keyをtransport failureにしない |
| GUI-CREATE-INT / STATE / KEY / FOCUS / ERR | `apps/gui/tests/creation.test.tsx`、`authoring.test.tsx`。guided input、並べ替え、64bit値、Conflict入力保持、二重submit防止、Unknown再確認、Cancel/reopen、write capability、dirty buffer保持 |

## Rationale Freshness

- **Fresh — 排他的なfile公開**: 同一parentにcomplete sourceをstageしてからexclusive hard linkで公開する。
  destinationへ直接writeするとpartialを露出し、通常のrenameでは競合entryを置換し得るため、この形を維持する。
  `cap-std` / `cap-fs-ext`のdirectory capabilityとcomponentごとのno-follow openでI/O境界を保持する。
- **Fresh — candidateの依存範囲だけをvalidation**: Project全体のdiagnosticをgateにすると無関係な既存errorまで作成を妨げる。
  必要なdependencyは重複宣言も含めて取り込み、解決不能や衝突を隠さない。
- **Fresh — 作成後のworkspace refresh**: Project Reloadと別にExplorerを更新し、既存dirty bufferを保持する。
- **Fresh — uncertain requestの保持**: submit時のdestinationを記録し、form変更やCancel/reopenでrecheckを迂回させない。
  これは現在のGUI session内のrecoveryであり、process crashを越えるtransaction保証ではない。
- TableのC#名導出は既存処理を共通helperへ移した。名称変換規則自体は変更していない。

## Evidence Integrity / Architecture

Requirement参照は上記canonical ownerに対応する。architectureはADR-0001 / ADR-0002 / ADR-0006の
YAML Source of Truth、shared core、host capability境界を維持する。frontendはYAMLを構築せず、
型候補を共有層から取得する。C#生成名のpreflightは既存codegen validatorを再利用し、生成物を書き込まない。
benchmarkによる性能改善は主張しない。

## Findings / 検証の限界

self-reviewで確認したsymlink check/open間の差し替え、Cancel/reopenによるUnknown retry、
IPC decode errorの分類は同じwork package内で修正した。残るBlocking / Specification Gapは確認されていない。
これはimplementation self-reviewであり、確定Candidateに対する別passのfinal verificationではない。

ブラウザの描画・keyboard smokeはTauri境界をmockして実施した。native filesystemはRustテストで検証したが、
native GUIの手動操作、Windows/Linux実機、OS crash・電源断時の動作は検証していない。

## Validation / Verdict

- `cargo xtask check-all`: 成功。fmt、clippy、Rust/native GUI tests、frontend lint/test/build、integration smokeを含む。
- `cargo xtask check-rationale`: 成功。参照の存在確認と上記rationaleの意味確認を別々に行った。
- browser smoke: 1440px / 1024pxでform描画、keyboard reorder、Cancelを確認した。
- 既存MessagePackのNuGet advisory warningとfrontend bundle size warningは残る。

Ready to merge: Yes（implementation self-review）。仕様StatusはApprovedのままであり、
Objective完了判定はexact Candidateに対するfinal verificationで行う。
