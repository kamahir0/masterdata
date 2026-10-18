# 保存・Explorer・構造編集のfeedback

Status: Applied

Approval mode: Agent-autonomous

## Basis

2026-10-18 Humanの実機レビュー12項目とExplorer比較画像。保存状態の信頼性、一般的なファイル操作、直接操作中の位置の理解、処理結果の再確認を改善する。

## Behavior

- 保存成功後、Explorer・tab・全体の状態を同じ保存基準から直ちに更新する。保存I/O中に追加された編集は未保存として保持し、Undo / Redoも保存済み基準から判定する。
- 「保存」とCmd/Ctrl+Sは表示中のファイル、「すべて保存」とCmd/Ctrl+Shift+Sはproject内の変更全体。現在ファイル保存は他ファイルのdraft、設定、無関係な空folder作成・削除を保存しない。移動・新規の対象fileに必要な親folderは作る。複数fileへ波及する編集の他fileは未保存として残り、全保存でまとめて確定できる。対象なしの通常保存は無効。close / project切替前の「保存して続ける」は全保存し、失敗時は入力と旧fileを保持する。
- Explorerはheader以外のproject名重複を除き、28px程度の密度で、三角の隣にfolder / file icon、続いて名前を置く。名前の幅不足はellipsisとfull pathのtooltipで示し、未保存markは潰さない。Escapeはrename中ならrename取消、それ以外はExplorer選択だけを解除し、表示中のdocumentを閉じない。
- 同梱Adventureの非空map / sequenceは最初から2スペースのblock YAML。コメント、引用符、64bit値、意味を維持する。空collectionは[] / {}を使える。
- Flagsの予約member Noneは名前・値0・存在を固定して編集・削除を無効にする。先頭を保ち、他memberの並替えはNoneの後ろで行える。raw sourceの自由編集と既存の診断は維持する。
- 表の行・列、Enum member、Custom fieldの追加に短い出現motion、ボタン並替えに位置の入替motionを付ける。ドラッグでは対象がpointerに追従し、移動先に応じて他の項目が空間を空ける。表示上の仮順序と保存する順序を分け、drop時に1つのUndo単位で確定する。Esc / pointer取消ではdraftを変えない。入力中の値やfocusを失わせず、reduced motionでは補間を抑える。配列の並替えにも同じ直接操作を使う。
- 「生成結果」の入口と表示menuからBuild / Publish panelを再表示でき、再表示だけでは生成・配置を実行しない。project内で直近の成功・失敗・Publish結果を保持し、対象revision / profileの古さも分かる。

Agent Decision: 複数fileに波及した操作を「現在file保存」で暗黙に全保存しない。未保存の残りを可視化し、全保存を明示的に選べるようにする。ドラッグは編集中の途中状態へ命令を連発せず、一時previewから一度だけcommitする。

## Validation

現在file保存・全保存・保存中の追加入力・Undoと保存表示、外部変更の保護。深いExplorerのellipsis・icon配置・Escape。None固定、同梱YAMLの値保持。追加、pointer途中の追従と隣接preview、drop / cancelとUndo、ボタン移動、reduced motion。結果panel再表示でbuild / publish呼出しが増えないこと。実画面とfocused tests / CI。

## Review

Blocking Issues / Non-blocking Issues / Questions: None. Approved as Proposed: Yes. Autonomous approval eligible: Yes（選択済み要求内、部分保存の残存draftと失敗・取消を明記し、外部作用は既存の保存・明示Publish境界を維持）。Human gate: None.
