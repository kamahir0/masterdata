# 編集対象・操作結果の共通基盤とExplorer

Status: Applied

Approval mode: Agent-autonomous

## Basis

2026-10-19 Humanは前turnの基盤改善（編集操作後の表示・focus・feedbackと、値やindexに依存しない対象識別）を実装するよう指示。追加レビューは高精細でgreen accentを取り入れたapp icon、削除motion、tab単体の出入り、Explorerの可変幅・種類色・作成focus・folder drop、種類badgeとTable Dataの区別、PK / SK / Key表示、参照値と名前の区別。

## Behavior

- 行・列・Custom field・Enum / Flags member・配列要素は、追加・複製した対象を表示して入力を開始する。表示とfocusは対象の安定IDで対応させ、画面ごとのindex推測や固定frame数に依存しない。後続のユーザー操作・navigationは古いfocus要求を取り消す。通常の値変更、同値の配列要素、並替え、Undo / Redoでも対象の識別を保つ。IDはeditor session内で管理し、YAMLへ加えない。
- 共通の編集操作結果を表示・focus・motionへ渡す。データ変更は即時に一つのUndoへ反映する。削除では可視対象に約200msの退出feedback、残る項目に約260msの位置移動を示す。表示用の残像は入力・selection・accessibility対象にしない。大量選択でも画面外の項目をcloneしない。失敗・Undo・連続操作でghostを残さない。
- tabは当該tabの幅・opacityで約200msの出現・退出を示す。既存tab全体へ縦方向の補間を適用しない。横scrollを保ち、active tabの表示もtab帯内に限定する。wrapは通常の折返しに従う。closeは既存の未保存保護を保持し、退出するtabへ入力できない。reduced motionでは即時表示する。
- Explorerの境界をdragで幅変更でき、keyboard矢印 / Home / Endとdouble clickによるdefault復帰を用意する。幅はPC内設定として保持する。最小180px、最大480pxをdefault範囲とし、狭いwindowでも編集領域を確保する（Agent Decision）。折畳み状態と分離し、project dirtyへ影響しない。
- 作成時は検索を解除し、全祖先と作成先folderを開き、新規項目をscroll表示してrenameへfocusする。menu操作 / rename入力はfolderの開閉を誘発しない。新規folder自身を選択する。Escのrename取消は既存規則を保持する。
- folder node本体へのdropはそのfolderへ入れる。folder全体をhighlightし、node間の隙間は親folderへのdropを線で示す。指示と実際の移動先を一致させ、自己 / 子孫 / 衝突は実行前に拒否する。必要ならhoverで展開する。可逆なdraft moveとし、確認modalを追加しない。
- source種類の形は維持し、Table=blue、Table Data=cyan、Enum=amber、Flags=orange、Custom=violet、Value Object=rose等の種類色を両themeで識別できるようにする（Agent Decision）。green accentは操作状態に使い、種類名 / 形も残す。header badgeをiconの左へ置き、TABLE / TABLE DATA / ENUM / FLAGS / CUSTOM TYPE / VALUE OBJECTで区別し、iconと同じ種類色を使う。
- recordsだけを持つ分離fileのUI名称はTable Data（テーブルデータファイル）。「テーブル定義ファイル内」を統一し、作成入口を「テーブルデータファイルを作成」とする。これは空fileの作成であり、既存recordの移動とは説明しない。
- 表の列headerへPK、SK n（secondary keyの0始まりgroup番号）、Key n（fieldのMessagePack key）を文字で示す。複合keyは各対象列に所属と順序を示し、複数所属も保持する。secondaryの一意 / 重複可もtooltipで識別する（Agent Decision）。編集と詳細設定への既存入口を保持する。
- 参照選択は実値を先、表示名を後に置く。実値と補助名を独立したtextとして表示し、補助名の弱い色 / 小さい文字で階層化する。検索は両方を対象とし、保存する値と参照先navigationを保持する。
- app iconはgreen accent系のvector原稿から高精細の各OSサイズを生成する。macOSは16〜1024pxのretina slot、Windowsは16〜256pxを含み、小さいサイズでも線が読めるようにする。native bundleと画面内brandで同じ意匠を使う。

## Validation

primitive配列の同値・編集・複製・moveとUndoでID / inputを保持。全collection種別の追加・削除・並替えと即時入力、遅延描画、次の操作によるfocus取消、削除snapshotの非対話化とcleanup。tabの上下座標と単体enter / exit、wrap / close / unsaved。Explorerのmouse / keyboard resizeと復元、検索中・閉じたfolder内作成、folder中央 / gapへのnative drop、取消とUndo。種類badgeの順序・色・label、複合key所属、参照の実値 / 補助名と選択 / navigation。iconの全resolution slotと実画面。focused tests、3 OS package / startup / installとself-review。

## Scope and review

保存format、出力処理、security boundaryを変更しない。ユーザーdraftへ暗黙保存・破棄を行わない。Objective内のagent判断として承認適格。別passで安定IDとUndo、即時削除と非対話snapshot、遅延focusの取消、tab帯内scroll、native dropの判定とhighlight、key番号の根拠をchallenge review。Key nはfieldのMessagePack keyであることをlegacy2の旧UI例と現在のbuilderで確認し、提案を修正した。Blocking / Questions: None。Approved as Proposed: Yes。Autonomous approval eligible: Yes（選択された改善内で、format / data safety / 外部作用を保持）。Human gate: None。
