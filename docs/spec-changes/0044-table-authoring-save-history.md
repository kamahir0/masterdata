# 仕様変更 0044: Table authoring contextのSave / Undo

Status: Proposed

## Affected Specifications

- `docs/gui/table-editor/spec.md` — `GUI-UNIFIED-004/005`。
- `docs/gui/data-editor/spec.md` — active record file Saveとshortcut。
- `docs/gui/data-editor/grid-authoring.md` — file-local historyとkeyboard precedence。
- `docs/specs/field-declaration-mutation.md` — `FIELD-DECL-008/010`。

## 根拠と分類

- **Human Requirement**: Tableを編集しているmental modelにSave / Undoを合わせ、Save Allとは区別する。Option A/B/Cを比較し、Cを真剣に検討する。partial resultとsource safetyを維持する。breaking / persistence semantics変更ならHuman gateに従う。
- **Human Preference / Proposal**: Table-level Save、chronological Undo、単一Save controlは強い候補であり、承認済み決定ではない。
- **Approved authority**: 現行Saveはschema fileとactive record-bearing fileをそれぞれ独立対象とし、一方が他方を暗黙保存しない。各fileのhistory/base identityも独立。Cmd/Ctrl+Sはactive record sourceのみと定義される一方、実装はschema dirty時にschema Saveを優先する。両者のずれは修正対象。

## Friction inventory（Desktop観察＋コード・test確認）

| Task | Current path / input | 常設chrome・遷移・摩擦 | 改善候補 |
| --- | --- | --- | --- |
| type change | header Select、1選択 | draft barが出現しgrid下降 | header状態へ統合 |
| Nullable / Array | header `?` / `[]`、1 click | 同上 | 同上 |
| add column | header末尾 `+`、1 click | Plan由来の例外時は対話、通常は直接 | primary path維持 |
| rename column | header input、入力＋Enter | field errorはheader付近 | 維持、focusを確認 |
| add row | grid末尾 `+ Add Row`、1 click／More→Add Row、2 clicks | primary path重複、長いgridでは末尾が遠い | grid境界＋keyboard path、More重複削減 |
| cell edit | double clickまたはEnter/F2、入力＋確定 | 通常は直接 | 維持 |
| copy / paste | Cmd/Ctrl+C/V | shared preflight後、bufferへ直接適用済み。Batch toolsには別のpreviewが残る | normal path維持、advancedを補助面に |
| Undo / Redo | grid Cmd/Ctrl+Z/Shift+Z、schemaはheader focusかdraft bar | historyがfile/category別、直前のschema変更をgrid focusから戻せない | Table文脈の時系列intentを検討 |
| Save | header Save／draft bar Save schema／Cmd/Ctrl+S | 同じTableに別の対象、shortcutとbuttonも不一致 | Option比較、Table context Save候補 |
| search | input＋EnterまたはSearch button | 同等経路が2つ常設 | input中心、buttonの必要性を再評価 |
| filter / sort | Filter & sort→5 controls→Apply | 展開でgrid下降、フォームが常設flowへ入る | 重畳する補助面、active条件のみchip |
| complex value | cell→Popover→値操作→Apply | Array item操作は既に各itemの`…`に収納、Popoverの確定が必要 | 値優先の実機検証、不要な再設計を避ける |
| diagnostic repair | Problems→対象cell/header→修正 | draft barと件数が重複、bar挿入で視点喪失 | local marker＋Problems、位置固定 |
| Diff | header Data/Diff切替 | source単位の補助viewへ遷移 | 低頻度導線の密度を再検討 |
| new record source | More→New data file | 作成flowへ遷移 | Table contextから到達維持 |
| advanced Table details | More→Table details | 展開でgrid下降、低頻度metadata | secondary surface維持、normal header操作と重複しない |

Clicks / keysは現行実装の通常最短path。未保存入力のあるDesktopでは破壊的経路を実行せず、keyboard・failure pathはコードとtestで補った。

## Saveモデルの比較

| 案 | 利点 | 問題 |
| --- | --- | --- |
| A UIだけ統合 | 小変更、現行file-local contractに近い | 両方dirty時に追加判断が残り、Cmd/Ctrl+Sの意味が不安定 |
| B split menu | 個別対象を明示できる | 保存対象のfile分類を通常操作へ露出し、primary actionが曖昧 |
| C Table-level Save | Tableという編集文脈へ一致、schema＋active recordを一度に扱える | multi-file partial resultとsame-file inline compositionの設計・実装が必要。現行Approved Save contractを変更 |

**推奨するAgent Proposal: C。** Cmd/Ctrl+Sとheader Saveは、現在表示するTableのdirty schemaとactive record sourceだけを対象にする。他Table、非active record source、Project全体を暗黙保存しない。個別file Saveは高度な明示操作として残せるが通常導線にしない。Save AllはProject全体の別command。

## 提案する差分（Human decision待ち）

1. GUIはTable authoring contextをSave intentとしてshared Applicationへ渡す。frontendが複数の低レベルSaveをdomain authorityとして順序付けない。
2. Applicationは対象physical file集合とそれぞれのexact base identityを明示し、inline schema+recordsが同一fileなら単一candidateとして安全に保存する。分離sourceならfile別結果を返す。成功したfileだけnew baseへ進め、失敗・Conflict・Outcome Unknownのfileのdraft/historyは保持する。一部成功を全体Successと表示しない。
3. semantic-invalidはsource safetyを満たせばSave可能。Buildabilityやvalidation resultはSave successから推定しない。
4. Undo/Redoは現在Tableで最後に確定した未保存authoring操作へ作用する。schemaとactive record sourceの時系列順を持つ一方、per-file history/base identityを保持し、Save成功したfileの履歴だけを再基準化する。text control内のUndo、disk rollback、別Tableの履歴とは区別する。focusとshortcut precedenceは既存のtext edit契約を維持する。

## 互換性

source format / CLI / binaryは不変。ただしGUIのCmd/Ctrl+Sとheader Saveのpersisted対象が変わる。schemaとrecordの双方がdirtyなとき、従来は片方だけ保存する契約から両方の試行へ変わる。保存後のpartial stateも新しいobservable behaviorである。

## 受け入れと実装への影響

- inlineとseparateでschema＋active record dirtyからSaveし、対象集合と結果を確認。他Tableや別record sourceはdirty維持。
- schema success＋record Conflict、schema Conflict＋record未試行、Outcome Unknown、I/O failureをそれぞれ正確に表示し、source safetyとdraft保持を確認。
- schema→cell→Undo→Undo→Redo、file Saveをまたぐ履歴、text control内Undo、focus切替を確認。
- shared application test、GUI focused test、Desktop Golden Path、repository checksとCI reconciliationを実施。

## 未解決事項

Human decision: A/B/CのどれをTableの通常Save modelにするか。C採用なら同一file candidateの合成と分離fileの失敗順序・部分結果を実装前にreviewで固定する。

## レビュー

Blocking Issues: None identified for decision presentation。Non-blocking Issues: same-file compositionのAPI詳細は決定後のrefinementで確定する。Questions: Save modelのHuman choice。Approved as Proposed: Yes（採用可能な選択肢としてのsemantic review verdict）。Autonomous approval eligibility: Eligible: No。Human gate: Material product fork。Rationale: A/B/Cには合理的trade-offが残り、ObjectiveはCを強く示唆するが必須決定としていない。Cmd/Ctrl+Sのpersisted対象変更は既存Approved contractを改める。

## 承認記録

Human decision待ち。
