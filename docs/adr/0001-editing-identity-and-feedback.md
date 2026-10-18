# 編集対象の識別と操作後のfeedback

Status: Accepted

関連behavior: [EDIT-005 / UI-003](../specs/desktop.md)、[変更0008](../spec-changes/0008-editing-foundation-and-explorer.md)。

## 判断の理由

表示indexは並替えで変わり、値は編集中に変わる。同値のprimitive配列は値から区別できない。PKも編集可能なユーザーデータであり、入力コンポーネントのidentityには使えない。

objectにはsession内のidentity、配列には各slotのidentityをWeakMapで関連付ける。値更新ではidentityを継承し、挿入・削除・移動ではslot対応を明示して渡す。Undoは変更前の配列そのものを保持するため、別のindex推測による復元を要しない。定義変更に伴う再帰的な値の書換えでも同じ対象であることを継承する。YAMLに編集用IDは混ぜず、再読込後に同じIDが続くことも要求しない。

## 表示と入力の完了を一箇所で扱う

追加後のfocusを画面ごとのindexや一定回数のrAFで実装すると、filter解除、virtual mount、遅い描画のたびに例外が増える。編集commandの結果として安定IDを返し、共通controllerが対象の表示、feedback開始、入力開始を処理する。各viewは表のvirtual scrollやTreeのscrollToなど、表示位置の調整だけを提供する。後続のユーザー操作や対象画面の変更は古いfocus要求を終わらせる。

表示位置の調整後、入力へ渡す前にfeedbackを開始する。スクロールイベント後のrAFにだけ委ねると、WebKitの大きな表では最初のキー入力が新規項目のfeedbackを終了させ、出現が一度も見えない競合が起きた。`large additions` UI回帰シナリオはframe配信を遅らせ、表示と即時入力の両立を確認する。

## 削除とタブ

退出アニメーションのために古いeditorをmodelやReact treeへ残すと、保存、Undo、編集対象が二重になる。削除は即時確定し、可視DOMの非対話snapshotだけを退出させる。snapshotはinert / aria-hiddenで、対象復元、連続操作、画面破棄時に除去する。virtual model全体や画面外columnをsnapshotしない。

ファイルタブには独立したpresenceを用いる。タブ全体へ二次元の位置補間を適用すると、幅変更・折返し・active tabの表示が干渉して、他のタブまで上下に動く。該当タブだけの幅とopacityを変え、残りは通常のflex配置へ任せる。active tabのscrollもタブ帯の中で完結させる。
