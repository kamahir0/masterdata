# Explorer新規作成をEscapeで取り消す

Status: Applied

Approval mode: Agent-autonomous

## Basis / scope

2026-10-23 Human Requirement: 新規ファイル / フォルダの名前入力中にEscapeを押すと作成自体をキャンセルする。既存項目のrename取消とblur競合の防止も含む変更依頼。

Canonical owner: `docs/specs/desktop.md` EDIT-005（Desktop / Browser共通Explorer）。保存形式、CLI、通常の削除、テーブルデータファイル作成は対象外。

## Behavior / Agent Decision

新規作成では作成先に仮のnodeを表示・選択して名前入力へfocusする。有効な名前をEnterまたは外側clickによるblurで確定するまではsessionへ追加せず、file tabも開かない。確定時に一つの可逆な作成操作として追加し、fileなら開く。Escapeでは入力値・初期名のどちらでも項目を残さず、以前のExplorer選択へ戻し、編集中file・既存draft・dirty・Undo / Redo履歴を保つ。仮nodeだけを取消対象とするため、既存データの削除は行わない。

既存項目のrenameはEscapeで入力だけを取り消し、元のpathと内容を保つ。Enter / Escape後のblurで二重確定しない。IME変換中のEnter / Escapeは編集確定 / 取消へ流用しない。

新規名が空または無効・衝突なら理由を通知し、仮nodeと入力を残して再入力またはEscapeで回復できる。既存renameの無効名は従来通り元のpathを保つ。検索解除・祖先展開・入力focusの保持・確定後に次のfocusを奪わない規則は維持する。

## Acceptance / validation

- 新規folderと各file種類を、未入力・入力変更後・無効名通知後にEscapeで取消でき、保存しても実体がない。
- 既存file / folderのrenameをEscapeで取消し元のpath・内容が保たれる。
- 既存dirtyとUndo / Redoを新規取消が壊さず、取消した項目が履歴から復活しない。
- Enter / 外側clickで新規作成を確定でき、Undo一回で作成前、Redoで確定名へ戻る。深いfolderでも入力と次操作へのfocusが維持される。
- Chromium / WebKitのUI回帰、coreの準備・作成・履歴のfocused test、workspace checkと型検査で検証する。

## Unresolved / safety

None。Agent Decisionは名前確定までの仮表示と履歴の一体化、取消後の選択復帰、無効新規名の再入力導線。いずれも依頼内の可逆なUI判断で、実データ削除・互換性破壊・Human gateはない。

## Review

別passで取消対象の限定、既存draft・履歴・file tabの保持、名前確定の単一操作化、無効名からの回復、IMEとblurの順序を確認。Blocking Issues / Non-blocking Issues / Questions: None。Approved as Proposed: Yes。Autonomous approval eligible: Yes（Humanの変更依頼内で、仮表示だけを取り消すため実データ削除に当たらない）。Human gate: None。
