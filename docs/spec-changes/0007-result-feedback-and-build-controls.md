# 操作を待たせない結果feedbackとBuild操作

Status: Applied

Approval mode: Agent-autonomous

## Basis

2026-10-18 Humanの20:44レビューとExplorer画像。親folder iconの中心と子folder switcherの中心を縦に揃える。source種類のicon、特にGit branch流用を見直す。dropdown / folder開閉は短いまま、追加・削除後の詰め替え等は少しゆっくり見せ、未実装箇所も調べる。profileは未選択として表現し、Build / Publish / 生成結果の3 buttonを整理する。

## Behavior

- Explorerは子のswitcherと親folder iconが同じ縦軸になるindent / slot / gapにする。深い階層でもellipsisと未保存markを保持する。
- 表、分離data、Enum、Flags、Custom、Value Objectを一式の線画iconで区別する。表はgrid、dataはrecordの行、Enumは単一の候補、Flagsは複数選択、Customは複数fieldの構造、Value Objectは一つの値を包む形で示す。Git branch / tag / 組織図の意味を流用しない。file / tab / 新規menuで同じiconを使い、tooltipやaccessible nameで種類を補う。色だけに頼らない。
- 待機を生みやすい開閉・候補表示・drag追従は既存の約100〜160msを維持する。追加の出現は約280ms、並替え / 削除後に残る要素の位置移動は約260msの緩やかなease-outをdefaultとする（Agent Decision）。タブ、行・列、Custom field、Enum / Flags member、配列、Explorer新規項目とBuild / Publishの結果を対象にする。
- 結果motionの完了前でも入力・追加・close・Undo・dragが可能。短時間の連続操作では前のmotionを安全に打ち切り / 接続し、古い表示やghostを操作対象にしない。reduced motionでは補間を抑える。単なる値入力、保存ack、scroll、folder開閉に伴う既存項目のmountを「追加」と誤認して再生しない。
- 表のrecord追加は追加先を表示して新規行へ移り、その場で入力できる。virtual行や横scroll後の新規列でも、可視になった対象に結果feedbackが届く。検索中の明示追加は検索を解除して追加対象を見せる（Agent Decision）。遅れて描画された後に、既に移動したユーザーのfocusを奪わない。
- Build profile初期値は選択なし。placeholderは「プロファイルを選択」、clearで未選択へ戻せる。未選択でもBuildでき、タグで絞り込まず処理する既存規則は保持する。これは「すべてのデータ」という名のprofileではなく、panelでも未選択と絞り込みなしを説明する。選択中profileが削除されたら未選択へ戻す。
- toolbarはprofileと一つのBuild split buttonにまとめる。主buttonはBuild実行、隣のmenuにPublishと実行結果の再表示を置く。menuや結果表示だけでBuild / Publishを実行しない。Publishは結果panelで対象成果物・配置先を確認してから明示実行する。成功・失敗・古い成果物・未実行が分かる。表示menuにも実行結果の入口を残す。busy中の重複Build / Publishを防ぎ、編集は維持する。

Agent Decision: 候補選択は即応を保ち、結果の出現と位置の変化だけに時間を使う。iconは小さい形で全意味を説明しきろうとせず、同じ意味の部品と種類名を揃える。Buildを日常の主操作とし、外部配置と結果確認を同じmenu / panelに整理する。

## Validation

深いfolderの中心座標とellipsis。種類別iconの形・名称・menuとの一致。タブclose後の隣接移動、表行 / 列・Custom / Enum / Flags / 配列の追加と並替え、virtual新規行と画面外列の表示。再生中の即時入力・連続追加・Undo・drag、値入力とscrollでの不要な再生防止、reduced motion。profileの未選択 / 選択 / clear / 削除とBuild対象、結果再表示で処理回数が増えないこと、Publish確認と古い成果物表示。両themeの画面とfocused tests / native / CI。

## Review

別passで、長いmotion中の入力・drag、virtual mountと実追加の区別、遅延focus、profileなしの意味、Publishの副作用と結果再表示をchallenge review。Blocking / Questions: None。Approved as Proposed: Yes。Autonomous approval eligible: Yes（選択されたUI改善内で、出力規則と外部配置の明示境界を維持）。Human gate: None。[Desktop Authoring](../specs/desktop.md)へ適用。
