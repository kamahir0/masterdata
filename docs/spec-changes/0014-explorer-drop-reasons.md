# Explorerの移動拒否理由

Status: Approved

Approval mode: Agent-autonomous

## Basis / scope

2026-10-22 Human Report: 同名項目がある移動先だけ禁止cursorになり、理由が分からずbugに見える。Human Requirement: 衝突時に移動できないこと自体はよい。VSCodeの置換確認は参考例であり、上書き要求ではない。

## Behavior / Agent Decision

ドラッグ中、移動先に同名のfile / folderがある場合は、禁止cursorに加え、Explorer内に名前・移動先・理由と「名前変更または別folderを選ぶ」対処を文字で示す。自己 / 子孫への移動も理由を示す。表示はdrag先判定と同じ結果から導き、許可先へ移る・領域外へ出る・drag終了 / 取消で消す。feedbackはpointer / focusを奪わず、行の位置とscroll領域を変えない。ドロップeventが届いた拒否操作では同じ理由を通知し、draft / Undo / diskを変更しない。同じ場所は従来通り変更なし。

置換・暗黙rename・folder merge・新しい確認modalは追加しない。フォルダ中央、file間のgap、見出し / 空白のroot移動、hover展開・Undoを保持する。検索中も判定は表示対象に限定せず全draftを用いる。

## Acceptance / validation

file / folder衝突、rootとgap、自己 / 子孫、許可先への切替・drag leave / endのfeedback取消、既存内容とdirty / Undo不変、rename後の再移動とUndoをChromium / WebKitで検証する。native HTML dragの禁止先ではbrowserがdrop自体を送らない場合もあるため、主となる理由表示はhover中に行う。light / darkと狭いExplorerで文字を確認し、正常移動の回帰を行う。

## Unresolved / safety

None。上書きせず、現行の移動拒否規則とdata safetyを維持する。製品の新しい置換機能は対象外。

## Review

別passでHumanの参考例を置換要求と取り違えないこと、native dropが届かない場合のhover説明、gap / rootの実際の移動先、検索から隠れた衝突、取消と許可先への移動での表示回収、focus / pointer / Undo / disk不変をreviewした。Blocking / Non-blocking / Questions: None。Approved as Proposed: Yes。Autonomous approval eligible: Yes（選択Objective内の説明改善で拒否規則とデータを維持する）。Human gate: None。
