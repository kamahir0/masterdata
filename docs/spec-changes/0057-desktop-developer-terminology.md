# Desktop日本語・開発者用語の見直し計画

Status: Proposed

## Affected Specifications

- [App shell / GUI-SHELL-LANGUAGE-001](../gui/app-shell.md#gui-shell-language-001): 日本語第一言語を維持し、技術識別子だけでなく開発者の慣用的な概念名を英語で提供できるようにする。
- [Product terminology](../product/terminology.md): 採用時にDesktop表示語の対応表を追加する。意味の定義を再所有しない。
- [Presentation quality](../gui/app-shell.md#presentation-quality): 既存のcompact density / accessibility / stable geometryへ適合する文言修正。新しいRequirement ID / feature / acceptance dimensionは追加しない。

## Source Evidence and Classification

- **Decision（既存）**: 2026-10-09 Human requestは日本語中心、特に設定説明文の日本語化を求め、一般的な日本語版開発ツールで原語を保つ名称は残す。[0055](0055-japanese-desktop-language.md)が変更経緯、App shellが現行authority。
- **Proposal**: Humanが共有したセカンドオピニオンは、操作・説明と開発者の概念名を役割で使い分け、短いラベルと説明文を分離する提案。個別の英語表記案をHuman確定decisionとして扱わない。
- **Decision（今回のscope）**: Human request「その辺を全体的に見直し、修正プランを策定」。今回の成果はレビューと修正計画。canonicalへの適用とproduct実装は行わない。
- **Agent proposal**: 以下の用語選択、画面別差分、実装順序。日本語の通常操作・説明を維持し、慣用的な日本語まで一括で英語へ戻さない。

## Confirmed Decisions / 調査範囲

調査基準は`main`の`d2697653bcd466d68169a9011336b0be3664abbe`、working treeはclean。対象は`desktop/web/src`の全presentation surface、`language.ts`、`workspace.ts` / `delivery-state.ts`の表示メッセージ、既存language tests / Desktop evidence adapters。以下はsource review結果であり、今回の実機visual evidenceではない。

- 型カテゴリはcreation / empty Project / Type editor / diagnosticsへ分散し、日本語表示が重複している。
- Build / Publishは「ビルド・配布」「ビルド成果物を配布」等へ置換され、操作名とCLIの対応が弱い。
- Table declarationは主キー・副キー・参照・MessagePackキーを編集するが、「詳細」「宣言」が混在する。
- 再確認ラベルはソース一式・設定・パス・成果物で異なる。対象も実際に異なるため、機械的な一括置換は不適切。
- 一般操作の未翻訳が`table-declaration.tsx`に残る（`Edit Secondary Key` / `Edit Reference` / `Remove Reference`）。`initializer.tsx`のroot見出しも`Constant value`が残る。
- `Null` / `null`、フラグ / フラグ列挙型、カスタム値 / カスタム型が混在する。
- 通常操作、Welcome / Recent、Explorer、テーマ、Search、Problemsの日本語は概ね維持できる。長いaccessible labelには対象を識別する役割があり、visible labelと同じ長さへ揃える必要はない。

## Proposed Delta

### GUI-SHELL-LANGUAGE-001の置換案

> Desktopは日本語を第一言語とし、説明文（設定項目を含む）、確認・エラー・診断の説明、標準UI componentの一般文言を日本語で提供する（MUST）。操作名・状態・accessible labelは日本語を基本とし、コード・CLI・設定と対応する概念名には製品で統一した開発者の慣用表記を使用してよい（MAY）。短い操作ラベルと、対象・影響を説明する文を役割に応じて分け、accessible nameにもvisible labelと共通の操作名を使う（SHOULD）。対象の識別や破棄・上書き・復旧等の判断に必要な情報を短縮で失ってはならない（MUST NOT）。製品・システム名、path / filename、schemaのfield / type / enum member / identifier、入力値、コード、format、CLI command、Diagnostic Code等のtechnical identityは翻訳・変更しない（MUST NOT）。表示用語の対応はProduct terminologyへrouteする。外部tool由来のtechnical detailは原文を保持してよい。言語選択設定や追加の多言語対応は要求しない。

### 推奨する用語対応

適用時の表示語のownerはProduct terminology。本表は承認前の提案であり、現行実装authorityではない。

| 概念 | 推奨表示 | 説明・使用上の方針 |
| --- | --- | --- |
| Value Object / Enum / Flags Enum / Custom Type | `Value Object` / `Enum` / `Flags` / `Custom Type` | カテゴリ名をcreation / editor / diagnosticsで統一。必要な説明は日本語。`Flags`は既存Flags Enumへの表示上の短縮で、新カテゴリではない。 |
| Array / Nullable / Required | 配列 / `null`を許可 / 必須 | 慣用日本語を維持。Nullableは値の`null`と対応させる。Requiredの存在・非null条件は既存意味論に従う。 |
| Underlying Type | 基になる型 | 自然な日本語を維持。`int` / `long` / `ulong`等の型名は原語。 |
| Project / Schema / Table / Field / Record / Source | プロジェクト / スキーマ / テーブル / フィールド / レコード / ソース | 一般的な開発者用語。TableとTOMLテーブル、Recordと画面上の行、論理Tableとphysical sourceを混同しない。 |
| Primary Key / Secondary Key / Reference | 主キー / 副キー / 参照 | 英語へ戻さない。MessagePackキーと区別する。 |
| Table declaration surface | テーブル定義 | メニュー・Drawer・再取得・確認文を統一。「宣言」はYAML内の宣言自体を説明する文脈では残す。 |
| Build / Publish | `Build` / `Publish` | 操作名・実行状態・操作を指す説明で統一。「配布先」「成果物」「生成」は説明上の自然な日本語として残す。 |
| Build Profile | `Buildプロファイル` | 見出しと跨画面の名称を統一。同じ文脈内では「プロファイル」と短縮可。profile名とconfig keyはそのまま。 |
| Migration / Plan / Apply / Compare | 構造変更 / 変更計画 / 適用 / 比較 | 今回は英語へ広げない。既存のPlan・fresh check・authorizationの意味を維持。 |
| Save / Save All / Undo / Redo / Find | 保存 / すべて保存 / 元に戻す / やり直す / 検索 | 維持。検索・履歴をsource mutationと混同する文言へ変えない。 |
| Light / Dark / System | ライト / ダーク / システムに合わせる | 現行を維持。OS follow / 保存preferenceの意味はそのまま。 |
| Conflict / Outcome Unknown / Recovery Required | 外部変更との競合 / 結果を確認できません / 復旧が必要です | 状態説明は日本語。成功・失敗・未実行・一部完了を含め、既存の結果分類を潰さない。 |
| `null` / `true` / `false` / `None`、製品・規格・識別子 | 原語・原値 | データ値と名称は保持。technical placeholderへ翻訳を適用しない。 |

### ラベルと説明の具体案

| 現在 | 推奨visible label | 残す説明・対象識別 |
| --- | --- | --- |
| ビルド・配布… | `Build / Publish…` | 同じ既存Drawerを開く。surfaceの増設なし。 |
| ビルド | `Build` | 保存済みソースと取得した設定・プロファイルが入力。未保存変更は含めない。 |
| ビルド成果物を配布 | `Publish…` | 最後に成功した成果物を対象に確認を開く。Buildを暗黙実行しない。 |
| ビルドして配布… | `Build → Publish…` | Build成功後にPublishの確認へ進む既存操作。最終実行には別の確認がある。 |
| 配布を実行 / 配布をキャンセル | `Publish` / キャンセル | 確認画面のtitleは`Publish内容の確認`。対象・変更・削除ファイルの情報を残す。 |
| テーブルの詳細… / 詳細に戻る | テーブル定義… / 定義に戻る | キー・参照・MessagePackキーを扱う現在の画面。 |
| 現在のソース一式を再確認 | 状態を再確認 | 隣接説明で当該操作の対象ソース一式を確認すること、自動再試行しないことを明示。a11yでも対象を識別。 |
| ディスク上の設定を再確認 | 保存結果を再確認 | `masterdata.toml`の現在状態の確認であり、再保存ではない。 |
| 現在のビルド成果物を再確認 | 成果物を再確認 | レシートと全artifact hashの確認。ソース最新状態やUnity検証は含まない。 |
| 現在のソースで計画を作り直す | 計画を作り直す | 現在の対象ソースから再取得。古いPlanを適用しない。 |
| プロジェクトを再読み込み / ソースを再読み込み | 原則維持 | 対象が違う。破棄の確認文を保ち、単に「更新」へまとめない。 |
| 破棄して再読み込み… / 変更前の状態に復元… | 維持 | destructive effectをラベルから隠さない。 |
| `Edit Secondary Key` / `Edit Reference` / `Remove Reference` / `Constant value` | 副キーを編集 / 参照を編集 / 参照を削除 / 初期値 | 開発者の概念名保持ではなく、一般操作・見出しの取りこぼしを修正。 |

ellipsisは新たな待機時間を表さず、確認・入力画面を開く現在の操作に合わせる。短縮したラベルの安全上必要な説明をhover-only Tooltipへ追い出さない。icon-only controlには既存契約どおりaccessible labelとTooltipを維持する。

## Implementation Impact / 有限の修正順序

1. **方針と用語を確定**: 別pass review後、有効なapprovalがある場合だけApp shellへdelta適用、Product terminologyへ対応表を置く。0055の過去決定は書き換えない。
2. **共通表示語を整える**: `language.ts`へ型カテゴリ・操作名等の共通表示を集め、inline JSXも使用する。全文章を辞書化する基盤、言語switch、一般i18n frameworkは作らない。native messageの原文key、action ID、wire値は維持する。
3. **画面ごとの適用**: 下表のscopeを順に閉じる。単純な全置換を避け、用語と安全情報を各文脈で確認する。
4. **既存検証との整合**: accessible name / focus復帰selector / evidence adapterを同じsliceで修正。label変更によって対象操作・focus・latest intentを失わない。
5. **実機確認・Candidate**: 以下の有限チェックを通して修正Candidateを作る。追加の見栄え調整や性能stretchへ広げない。

| Surface / file | 修正・維持する範囲 |
| --- | --- |
| `app.tsx`（Welcome / Recent / Explorer / command / Problems / appearance） | 型作成・Build / Publish・テーブル定義への名称を統一。通常の開く・作成・検索・問題・テーマは維持。 |
| `creation.tsx` / `type-editor.tsx` | 型カテゴリ・メンバー説明・Custom Type fieldラベルを統一。基になる型・変換・必須・配列は日本語。 |
| `grid.tsx` / `complex.tsx` / `initializer.tsx` | `null`表記、Flags / Custom Typeの値作成ラベル、初期値の取りこぼし。field名・member名・scalar表示は不変。 |
| `table-declaration.tsx` | 定義というsurface名、英語の一般操作取りこぼし、再取得・計画・確認文を統一。 |
| `settings.tsx` / `delivery.tsx` / `delivery-state.ts` | Build / Publish / Buildプロファイルを統一。配布先・タグ・パス・設定説明は日本語。入力snapshotと成功済みartifactを明確に分ける。 |
| `app.tsx` / `type-editor.tsx` / `table-declaration.tsx`の復旧表示 | 再確認labelを短縮し、対象・実際の状態確認・書き込み再試行停止の説明を残す。 |
| `source-path.tsx` / `project.tsx` / `tags.tsx` | 名前変更と移動、作成先と保存先、タグの操作説明を確認。技術識別子を保持し、自然な既存日本語は変更しない。 |
| `workspace.ts` / `language.ts` / `main.tsx`のpresentation接続 | 確認・診断・通知の用語、Ant日本語locale、`lang`、状態表示の一貫性を確認。外部tool原文fallbackは維持。 |

## Acceptance / 完了判定

既存`GUI-SHELL-LANGUAGE-001` / presentation / authoring contractsに対応する確認に限る。

- 共通対応表と全ordinary surfaceの表示が一致し、一般操作の上記取りこぼしが解消する。ボタンは簡潔で、設定・ヘルプ・警告の説明は日本語。
- 同じnative diagnosticやcategoryがcreation / editor / Problemsで別の用語にならない。dynamic identifier・path・member・入力値・64-bit文字列はそのまま。
- visible labelをaccessible nameに含め、必要な対象情報を残す。focus復帰・keyboard / IME・menu操作の既存testsがlabel修正で壊れない。
- `language.test.cjs`の既存ケースを更新し、用語対応と実値保持を直接確認する必要なcaseだけ追加。既存creation / type / declaration / settings / delivery / focus等の影響を受けるadaptersを再実行。全labelの内部配置を固定するsnapshotや新しいharnessは作らない。
- TypeScript / production build、repository integrity、required CIを実行。product変更を含む新Candidateの証拠はfreshに取得し、過去Candidateの証拠を読み替えない。
- actual macOSでWelcome / Table menu / Type creation・editor / Complex / テーブル定義 / settings / Build・Publish / Problemsを確認。Light / Dark、長い識別子、既存long/wide Tableで折り返し・切れ・重なり・gridの移動・focusを確認する。
- Conflict / unknown / recoveryの代表状態で、短縮labelでも対象・破棄範囲・再確認と再試行の違いを理解できることを確認する。検証用projectで行い、ユーザーのソースへ障害を書き込まない。
- この文言scopeの既存target / regression evidenceが成立し、具体的Blockingがなければ終了。新機能、レイアウト再設計、motion追加、未取得のmacOS OS入力 / Windows / Unity evidenceの代替作業は行わない。

## Compatibility Impact / Potential ADRs

presentation-only。YAML / TOML bytes、CLI / config / wire / generated C# API、Rust意味論、dirty / history / Save scope、fresh write authorization、Build / Publish / Migration lifecycle、bounded projection / navigation generationを変更しない。Potential ADRs: None。React / Ant Design、shared Rust authority、既存性能architectureは維持する。

## Open Questions / Approval Eligibility

Open Questions: None（個別用語は上記Agent proposalとして解決案を提示）。Autonomous approval eligible: Yes（通常workflowのreviewと後続scope authorizationがある場合）。Human gate: None for this presentation proposal。既存の外部環境evidence gateは変わらない。今回の計画策定scopeを理由に、approval / application / implementationへ進めない。

## Review / Approval Record

同一agentの別passで、元のrequest / 現行canonical / 実際の文字列 / 既存testsへ戻ってchallenge reviewを実施。独立reviewerの証拠とはしない。

- Blocking Issues / Non-blocking Issues / Questions: None identified。
- Approved as Proposed: Yes（semantic review verdict。status approvalではない）。Eligible: Yes。Human gate: None for this proposal。
- Intent / scope / normative strength: 日本語中心を維持し、個別用語はAgent proposalとして区別。慣用表記はMAY、ラベル構成はSHOULDとし、追加のhard quality barを作らない。
- Internal / cross-spec / terminology / ambiguity: 表示名とdomain identityを区別し、FlagsとFlags Enumの対応、Table定義とsource宣言、BuildとPublish、異なる再確認scopeを確認。
- Testability / compatibility / leakage / unrequested behavior / ownership: 既存observable boundaryで検証でき、Rust意味論・wire・source・operation lifecycleを変更しない。新feature / framework / acceptance dimensionなし。用語は採用後に既存glossaryへrouteする。

Approval Record: 未承認・未適用。今回の計画策定scopeに従いProposedで終了し、canonical / product / Candidateを変更しない。
