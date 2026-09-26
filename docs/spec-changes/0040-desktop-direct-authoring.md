# 仕様変更: Desktopの直接操作と静かな編集面

Status: Applied

## Affected Specifications

- `docs/gui/source-creation/spec.md`: `GUI-CREATE-LAYOUT-002`–`003`、`GUI-CREATE-STATE-001`、`GUI-CREATE-INT-002`–`006`、`GUI-CREATE-KEY-001`。
- `docs/gui/data-editor/spec.md`: `GUI-DATA-LAYOUT-006`、`GUI-DATA-LAYOUT-007`、`GUI-DATA-KEY-002`。
- `docs/gui/data-editor/grid-authoring.md`: `GUI-GRID-001`–`003`、`GUI-GRID-006`。
- `docs/gui/table-editor/spec.md`: `GUI-TABLE-LAYOUT-004`、`GUI-TABLE-INT-004`–`006`のPlan / Diff / Apply interaction。
- `docs/gui/type-editor/spec.md`: `GUI-TYPE-LAYOUT-004`とPlan / Diff / Apply interaction。
- `docs/gui/app-shell.md`: command surfaceとnormal / exceptional stateの視覚的優先順位。
- `docs/specs/source-creation.md`: `SOURCE-CREATE-001` / `003`のpathとcreation-time default suggestionの区別を明示する。

`docs/specs/source-creation.md`のcomplete declaration、`docs/specs/authoring-batch.md`のall-or-none buffer mutation / codec、`docs/specs/schema-migration.md`と`docs/specs/type-migration.md`のPlan / preflight / source-preserving commit / recoveryは変更しない。Applicationの追加入口はこれらの契約へ委譲する。

## 根拠と分類（Source Evidence and Classification）

- Human Requirement: Desktopで日常的な編集を短くし、データを主役にする。sourceの左ペインはfile treeとする。
- Human Constraint: safety、YAML source authority、shared Rust boundaryを弱めず、複雑性をApplication/Core側で引き受ける。
- Human-selected direction: 2026-09-26のGUI診断・再設計案に対する「その方向で進める」。これを個々のcontrol配置の厳密承認とは解釈しない。
- Agent Decision: 通常のpasteは保存前bufferへのall-or-none変更として直接適用し、Undo可能にする。変更前previewは明示commandとして残す。理由はsource commitを伴わず、失敗時buffer不変、保存前Undoが成立するため。backend preview / revision checkを省略するdecisionではない。
- Agent Decision: schema/type structural editは対象行・列から開始し、Planを自動生成するが、Migration commitには文脈内の明示Applyを要求する。複数file・destructive・dirty conflictでは影響詳細を強める。理由は現行のfile Saveとの違いとmulti-file mutationを利用者が認識できるため。
- Agent Decision: 大量recordのgridではsource occurrence / draft IDとview ordinalを分離し、visible windowのDOMだけを描画する。具体的なvirtualizer libraryはimplementation detail。

## 提案する差分（Proposed Delta）

### Source creation

`GUI-CREATE-LAYOUT-002`と`GUI-CREATE-INT-002`–`006`の「Create前に完全なdeclarationをフォーム入力する」義務を置き換える。ExplorerのNewでkindを選択すると、そのfolderに未commitの仮nodeを出し、filenameをinline入力できなければならない（MUST）。Enterでshared Application層のdefault intent operationを開始し、Escapeではmutationなしで仮nodeを消す（MUST）。Success時はnew fileを選択しtyped editorを開く（MUST）。Conflict / Failureでは入力を維持して修正・retryでき、Outcome Unknownでは既存のrecheck lifecycleを使う（MUST）。

Application層はpathからdomain identityを後続解釈するのではなく、作成時に一回だけ候補identityを生成し、complete typed `SourceCreation`へ変換して既存のpreflight / exclusive creationを通す（MUST）。この一回限りの候補生成は`SOURCE-CREATE-001`のpath-independent source interpretationと区別し、生成identityを作成前に確認できるようにする（MUST）。候補を安全に作れないnameでは短いidentity入力を求める。TableのdefaultはRequired `id: int`、MessagePack key `0`、Primary Key `id`、empty inline recordsとする（SHOULD）。fileとschemaだけを作る選択もNewの文脈から可能にする（MUST）。Dataはcurrent logical Tableが一意ならそれを提案し、一意でなければ作成前にTable選択を要求する（MUST）。Value Object / Enum / Flags / Custom Typeのdefaultはshared layerが現行type systemでvalidな最小宣言を選ぶ（MUST）。defaultでは足りないunderlyingや初期field/value等を作成前に指定するAdvanced作成導線を保持する（MUST）。作成後に現行Type Editorでサポートされないdeclaration操作を可能であるかのように見せてはならない（MUST NOT）。filenameとdomain identityの独立、path safety、project-local collision、Outcome Unknownを維持する。

仮nodeのfilenameを変更している間だけ、表示する候補domain identityも更新してよい（MAY）。Success後のrename / moveはidentityを変えない（MUST）。`SOURCE-CREATE-001` / `003`はsource interpretationのpath独立性を保持し、creation-time suggestionだけを区別して許す。

### Data / Table editor

`GUI-DATA-LAYOUT-006`を、gridが定常時の最大の視覚領域を占め、正常なvalidation / Saved / selection countを同強度の常設button列として表示しない形へ改める。未保存、validation pending / error、Conflict、Recovery Requiredは対象fileと関係付けて識別可能にし、screen readerに伝わる状態を省かない（MUST）。Searchは直接到達可能とし、filter / sortは条件があるときに状態を識別できる（MUST）。secondary/batch/project actionはcontextual commandへ置いてよい（MAY）。

Tableのfield name / type / keyはgrid headerで読め、field名変更・field追加をheader文脈からkeyboard / pointerで開始できなければならない（MUST）。record-bearing sourceに対するTable structureはgridから離れた別の設定画面だけにしてはならない（MUST NOT）。schema-only documentも同じfield list interactionを使う。操作入力が確定したらshared Migration Planを自動生成して表示する（MUST）。利用者が文脈内のApplyを実行するまでsourceは変更しない（MUST）。Plan概要は対象、destructive性、affected file / record数、diagnosticsを示し、全fileのbefore/after Diffへの明確な導線を提供する（MUST）。affected dirty file、stale、destructive authorization、rollback / Recovery Requiredの既存contractを維持する。

gridはview ordinalではなくstable source occurrenceまたはdraft IDで編集対象を保持し、scrollでrow DOMが消えてもactive selection、dirty buffer、range anchor、editing valueを失ってはならない（MUST）。query / sort後は既存`GUI-GRID-003`のselection ruleを適用し、Problemsから画面外のcellへ移動できなければならない（MUST）。visible row countに比例するbounded renderingを行い、large fileで全rowのDOMを同時生成してはならない（MUST NOT）。accessible row/column countとvirtual row位置を提供する（MUST）。

### Paste / complex values / types

`GUI-GRID-002`のpasteに関する常時明示preview / Apply義務を置き換える。grid navigation modeのCmd/Ctrl+Vは、shared codecとshared batch preflightによりbase / buffer / schema / query / selection revisionを一度のintentとして照合し、成功時は全targetを一つのUndo単位でlocal bufferへ直接反映する（MUST）。failure、read-only混入、stale、shape不一致ではbuffer不変で対象と理由を示す（MUST）。これはSave / Build / Migrationを起動しない（MUST NOT）。明示的なPaste preview / Fill previewとbefore/after / diagnosticsの確認導線は残す（MUST）。Fillとrange Set Nullは現行の明示preview / Applyを維持する。existing keyのbatch禁止等は変更しない。

Arrayなどの複合値では、各itemに同一のsecondary action群を常時繰り返して値より強く表示してはならない（MUST NOT）。move / remove等は対象itemのfocus、menu、keyboardから到達できなければならない（MUST）。値shapeとsource edit semanticsは変更しない。Type Editorのmember / fieldも同じ対象文脈の操作を基本とし、Migration Plan / Apply safety contractを維持する（MUST）。

## 互換性（Compatibility）

YAML / binary / CLI / project config / source identityは不変。GUIのcreation、paste、migration UI操作は意図的なobservable changeであり、既存のGUI手順testを更新する。Appのdefault-intent APIは追加のみとし、complete typed creation APIを削除しない。pasteは保存前buffer変更のtimingが変わるが、disk mutationとsource safetyは不変。

## 受け入れと実装への影響（Acceptance and Implementation Impact）

- New→Table→filename Enterでvalidなinline-records schemaが一つだけ作られ、gridが開く。Escape / collision / Outcome Unknownでworkspace stateと未保存bufferを守る。
- Field renameはheaderからPlanへ到達し、Apply前はfile不変。dirty affected file、stale plan、destructive operation、multi-file rollbackを回帰確認する。
- TSV pasteはgridから直接bufferへall-or-noneで入り、Undo / Redo / Save / Conflictで現行契約を守る。invalid value、existing key、stale selection、IME precedenceを確認する。
- 1万 / 10万record相当の固定inputでrendered rowがwindow内に限られ、遠いrowへのkeyboard / Problems navigation、sort/filter、added/deleted、range選択が成立することを確認する。具体的なlatency SLAは実測なしに規範化しない。
- Complex valueとType listはpointerに加えkeyboard / assistive technologyからsecondary actionへ到達できる。

## 未解決事項（Open Questions）

None blocking. 完全なTable schema/data統合dirty sessionとserver-side windowed snapshotは計測・運用結果を得て別Objectiveで検討する。今回のfield migrationは既存の明示Apply boundaryを維持する。

## レビュー（Review）

2026-09-26、Proposedをauthoring passから離してGUI / domain ownerと照合した。

- Blocking Issues: None identified。
- Non-blocking Issues: default-intent APIの具体的なwire shapeとvirtualizer libraryは実装時に選択する。
- Questions: None blocking。
- Approved as Proposed: Yes。
- Autonomous approval eligibility: Eligible: Yes。Human gate: None。HumanがGUI再設計の方向を選択済みで、source format / stable identity / CLI / security boundaryを変更しない。pasteは保存前bufferのUndo可能なlocal操作、schema/typeのcommitは明示Applyを保持する。
- Review dimensions: intent fidelity、cross-spec safety、testability、compatibility、documentation ownerを確認。既存のsource creation / batch / migration contractに責務を残す。

## 承認記録（Approval Record）

Approval mode: Agent-autonomous。Basis: Human-selected Desktop GUI redesign objectiveと上記review。Canonical application: same commit as this record (`docs/gui/source-creation/spec.md`、`docs/gui/data-editor/spec.md`、`docs/gui/data-editor/grid-authoring.md`、`docs/gui/table-editor/spec.md`、`docs/gui/type-editor/spec.md`、`docs/gui/app-shell.md`、`docs/specs/source-creation.md`)。
