# 操作buttonと選択のfeedback

Status: Applied

Approval mode: Agent-autonomous
Objective / request basis: 2026-10-22 Humanの3点feedback。別passのreviewにBlockingがなくorchestrationが承認。

## Basis / scope

2026-10-22 Human Requirement: 「レコードを追加」「テーブルデータファイルを作成」をhover前からbuttonとして認識できるようにする。Explorerの選択背景内でicon左の余白を少し増やす。file tab部分に選択の切替を示す短いanimationを付ける。Desktop UI-003 / UI-004と、それを共有するBrowser WEB-002がcanonical owner。

## Agent Decision / behavior

- 表footerの2操作は共通の高さと常時見える輪郭を持つbuttonにする。追加・file作成のiconと文字を表示し、hover / keyboard focus / disabledを区別する。従来の追加先・無効理由・入力開始・Undoの規則は保持する。
- Explorerのiconと名前を選択の有無によらず少し（初案4px）右へ寄せる。開閉矢印と階層線も同じ量だけ寄せ、子の矢印と親iconの中心軸、28pxの行高、ellipsis、dirty mark、drop表示・入力を維持する。
- file tab選択先の緑の上辺に約180msの伸長とfadeを付ける。選択先のtab内で完結させ、wrap / scrollでも現在位置が分かるようにする。本文は直ちに切り替える。同一tabの編集・dirty更新・再clickでは再生しない。連続切替は最新の選択だけを示し、close / preview・keyboard・退出motionを維持する。editor / OSのreduced motionではこの補間を抑える。

## Acceptance / validation

Light / Darkでpointerを外した状態でも両buttonの境界が見え、無効時の追加先案内と作成・追加・Undoが保たれる。狭幅でも操作を覆わない。選択fileのicon左に数pxの内側余白があり、深い階層で親icon / 子arrow・guideの軸が合う。mouse / keyboardのtab切替に短い上辺motionがあり、入力・scroll・dirtyを失わず、wrap・close・連続操作とreduced motionでも旧選択の動きを残さない。実画面の比較とfocused UI回帰を使う。

データ / filesystem / permission boundaryの変更なし。未決定事項: None。Human gate: None。選択済みの修正範囲内で自律承認適格。

## Review

確定したProposedを別passでchallenge review。buttonの常時borderとdisabled理由、余白追加による階層軸とdrop位置、選択上辺だけのanimationによる入力保持・連続切替・退出との両立を確認。本文を動かさず、同じDOMの選択状態にだけ補間を付けるため、draft / focusモデルへ影響を追加しない。Blocking Issues / Non-blocking Issues / Questions: None。Approved as Proposed: Yes。Autonomous approval eligible: Yes（選択済みfeedback内、可逆な表示変更、検証可能）。Human gate: None。
