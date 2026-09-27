# 仕様変更 0044: Unified Table interactionとsource-local lifecycle

Status: Proposed

## Affected Specifications

- `docs/gui/table-editor/spec.md` — `GUI-UNIFIED-001/002/004/005`、通常Saveの入口。
- `docs/gui/data-editor/spec.md` — `GUI-DATA-SAVE-001/005`、`GUI-DATA-KEY-001`。
- `docs/gui/data-editor/grid-authoring.md` — `GUI-GRID-004..006`の履歴とkeyboard precedence。
- `docs/gui/data-editor/record-mutation.md` — `GUI-DATA-ROW-001/010`のAdd Row target。
- `docs/specs/field-declaration-mutation.md` — `FIELD-DECL-006..010`、inline同一sourceのlifecycle。
- `docs/specs/source-edit.md` — `SOURCE-EDIT-007..011`のfile commit境界。
- `docs/specs/schema-migration.md` — `MIGRATION-009/010/014/016`との境界確認。Migration自体の意味は変更しない。

## Source evidenceと分類

- **Human decision（最新）**: logical Tableを単一interaction surfaceとする。dirty / ordinary persistenceはactual physical source representationの変更に従う。semantic dependencyによる再解釈・diagnosticだけではdependent sourceをdirtyにしない。workspace structural operationのactual mutation targetはdirect touchの有無で狭めない。
- **Human revocation**: 直前の「Option C: Save / Cmd+Sでdirty schemaとactive record sourceをまとめる」という承認は撤回された。Table interactionとSave unitを同一視したことが理由。0044をOption CとしてApproved / Appliedにしない。
- **Human requirement**: inline / separate / mixed / no record sourceで同じUnified Table面を使う。source-local dirty、semantic feedback、operation Undo、Migration transactionを別の単位として扱う。通常Save controlとshortcutの意味を一致させ、Save Allとは分ける。
- **Human proposal（未決定）**: active authoring source documentを通常Save targetにする案。単なるfocusでtargetが変わらない定義を検討する。
- **Approved authority**: `FIELD-DECL-006..010`はschema draft、sourceを変えないrecord再解釈、schema fileだけのSaveを定義する。`SOURCE-EDIT-007`はrecord-bearing source fileを通常commit unitとする。`MIGRATION-009/010/014/016`はactual affected source setのPlan / preflight / commit / rollbackを定義する。`GUI-GRID-004..006`はfile-local draft履歴とtext-control precedenceを定義する。
- **Implementation evidence（authorityではない）**: 現行GUIはschema draftとrecord draftを別stateに保持し、Cmd/Ctrl+Sはschema dirtyならschemaを優先する一方、Table headerのSaveは選択record sourceを保存する。inlineで両draftがdirtyな場合、schema Save後にrecord editorをrefreshして別Saveする。`open_context`はinlineと複数Data sourceを同一Tableに列挙する。`apply_table_intent`はMigration Planのactual affected filesと同Tableのrecord sourceをdirty gateに含める。

## 調査結果: operation matrix

「source representation changed」はdraft中の候補bytes変更と、Migration Apply後のdisk変更を区別する。diagnosticだけではどちらも生じない。

| Operation | Interaction target | Actual source representation changed | Semantic affected sources | Dirty ownership | Persistence / transaction | Undo model |
| --- | --- | --- | --- | --- | --- | --- |
| type change | column header | schema draft | 同Tableのinline / separate recordsと依存closure | schema physical sourceのみ | `FIELD-DECL-008`のordinary source Save | schema draft Undo。再解釈とdiagnosticも戻る |
| Nullable / Array | column header | schema draft | 同上 | schema physical sourceのみ | 同上 | 同上 |
| cell edit | selected cell | selected record source draft | Table / projectの解決結果 | selected physical sourceのみ | `SOURCE-EDIT-007`のordinary source Save | source draft Undo |
| Add Row | selected record set | selected record source draft | Table | selected physical sourceのみ | record sourceのordinary Save | source draft Undo |
| inline row edit | selected cell | schema file内records draft | Table | schema physical sourceのみ | schemaと同じfile lifecycleが必要 | source draft Undo |
| paste | selected range | preflight後の選択record source draft | Table / project | 実際に変わったphysical sourceのみ | `GUI-GRID-002/005`のdraft Save | batch一操作のUndo |
| RenameField | column header | schema、対象record、追随するReference declarationのactual patch | dependency closure | Apply前はordinary dirty draftを増やさない。成功後はaffected filesが新しいpersisted base | Migration Plan / multi-file commit / rollback | 現行は未保存Undo対象外。rollbackはcommit failure recovery |
| AddField | header末尾 | schema、recordsのある対象sourceへのactual patch | target Table records | 同上 | Migration Plan / multi-file commit / rollback | 同上 |
| DropField | column context | schema、recordsのある対象sourceへのactual patch | target Tableと依存closure | 同上 | destructive authorization付きMigration | 同上。逆操作を暗黙Undo扱いしない |
| Reference mutation | advanced Table details | Reference declarationを含むactual patch対象 | Reference dependency closure | 同上 | operation別Migration Plan / commit | 同上 |

Migration Planの`affected_files`はpatchのあるsourceだけを表す。dirty gateの照合対象には、Planの結果を安全にApplyするために読み込んだ同Table sourceも含み得る。両者を「dirty files」と呼び換えない。

## 提案するcontract差分（Save target決定前にも成立する部分）

1. interaction、dirty / persistence、semantic dependency、Undo、transactionの単位を分離する。Table headerとgridはinline / separate / mixed / emptyで同じ構造とし、複数record source時だけselectorを表示する。selected source切替では各sourceのbuffer、history、selection、queryを保持する。
2. dirtyは最後のpersisted baseからの**actual source candidateの差分**で決める。schema declarationの変更が別Data sourceにdiagnosticを作っても、そのData sourceのsource candidateが変わらなければcleanである。sourceを元へ戻せば該当dirtyと原因diagnosticは消える。
3. inline recordsとschema declarationsは同じphysical source documentを所有する。両draftが共存するなら、schema fileのbase identity、dirty判定、Save candidate、Conflict lifecycleを二重化しない。候補の安全な合成と、未保存入力を失わない失敗経路をshared Applicationで定義する。別Data sourceのcandidateは混ぜない。
4. structural operationはordinary draftへの擬似変換を行わない。shared Migration Planのactual affected filesをmutation targetとし、transaction成功後はそれらをnew persisted baseとして扱う。commit failure時のrollback / Recovery Requiredを未保存Undoと呼ばない。
5. Undoはoperationの性質に従う。text control内ではnative text Undoが優先し、schema draft Undoはschema候補だけを戻してshared semanticsを再評価し、record draft Undoは対象source候補だけを戻す。複数sourceを変更した**未保存**operationを将来導入する場合はoperation-wide Undoを検討できるが、現行Migration Applyのdisk rollbackをCmd/Ctrl+Zへ割り当てない。
6. GUIはdiagnostic発生やheader/cell focusだけからsource ownershipを推測しない。shared snapshot / mutation resultのschema source、selected record source、candidateのactual changed source、Migration affected filesを利用する。具体的wire shapeは固定しない。

## 通常Saveの未決定点

現行のApproved contractはschema Saveとactive record source Saveをfile別に定義するが、Unified Table headerの一つのSave buttonとCmd/Ctrl+Sの対象を一致させる規則はまだ定まっていない。特に別fileのschemaとactive record sourceが双方dirtyな時、次の候補はいずれも合理性がある。

| 候補 | focusだけ移動した場合 | 両source dirtyでのheader Save | 利点 / 摩擦 |
| --- | --- | --- | --- |
| 最後に確定したauthoring operationのsource | targetは変えない | 最後に編集したsourceをSave | 編集意図に近い。履歴・切替・Reload後にtargetを明示する必要がある |
| 明示したactive source context | UI上のcontextが変わるまで固定 | 表示中のsource contextをSave | 結果を予測しやすい。schema headerとrecord gridを同時に見る面でcontext選択UIが増える |
| 両方dirtyの時だけtarget選択 | focusだけでは変えない | Save時に対象を選ぶ | 誤保存を避ける。日常pathに判断を挿入する |

`active authoring source`は有力だが、単なるfocusの移動でSave targetを変えない点以外は未確定。Case A（schema edit後のcell focusのみ）、B（両方dirty）、C（schema→record→schema edit）、D（inlineで同一file）をDesktopで比較し、buttonとshortcutの意味、targetの事前可視性、Save後の選択遷移を同じ規則で決める必要がある。Save AllはProject全体の別operationに維持する。Option Cを再導入しない。

**Agent recommendation**: 「最後に確定したauthoring operationのphysical source」を保持し、focusだけでは変えず、Save buttonのsecondary detailで対象を確認できる案。表面を増やさずCase A/Cを説明しやすい。一方、両方dirtyでheader Saveの対象が表示中のrecord setと異なる可能性があり、他の選択も合理的なのでHuman gateと判定する。

## Acceptance / compatibility / implementation impact

- inline only: headerとrow editは同一schema fileをdirtyにし、Undoでbase一致ならclean。両draft共存時のSaveは同一source candidate、別Saveによるlost updateやdraft喪失を許さない。
- separate only: schema type変更はschemaだけdirty、Data sourceはclean + diagnostic。Undoでdiagnosticが消え、record sourceはcleanを維持する。cell editはselected Data sourceだけdirty。
- mixed: schema inline recordsと各Data sourceをrecord-set selectorで切替え、schema候補・各record候補と履歴を混同しない。schema変更のdiagnosticだけではData sourceをdirtyにしない。
- no record source: 共通schema header、空grid、Add Column、Create record source導線を維持する。Add Rowで保存先を暗黙生成しない。
- Rename/Add/Drop/Reference: actual patch対象はdirect touchの有無に依存せずPlanが返す。stale / dirty gate、commit rollback、Recovery Requiredを維持し、Apply成功を未保存dirtyとして扱わない。
- source format、scalar interpretation、CLI、binaryは変更しない。通常Saveの対象と同一file合成はGUI / shared Applicationのobservable behavior変更であり、未決定target ruleをImplementation convenienceで選ばない。
- focused Core / GUI regression、Desktopのschema→diagnostic→Undo、record edit→Undo、inline/mixed切替、keyboard focus / Save、repository checkとrequired CIを確認する。既存のstable grid位置を後退させない。

## Review / approval eligibility

Blocking Issues: None identified for decision presentation。Non-blocking Issues: 同一fileの候補合成APIと履歴再基準化はSave target決定後のrefinementが必要。Questions: schemaとselected Data sourceが双方dirtyな時、単一Save / Cmd/Ctrl+Sはどのphysical sourceを対象にするか。Approved as Proposed: Yes（Human decisionへ提示可能というsemantic review verdictであり、status transitionではない）。Autonomous approval eligibility: Eligible: No。Human gate: Material product fork。Rationale: latest Human decisionはsource-local ownershipを確定するが、通常Save targetの複数案を一意化しない。Review dimensions: intent fidelity、ID参照、Migration / source-edit境界、failure safety、compatibility、testability、documentation ownerは整合。未決定targetをimplementationで発明しない。

## Approval Record

2026-09-28: HumanはOption Cを一度承認したが、同日の最新Human requirementで撤回した。撤回理由はlogical Table interactionとphysical source lifecycleを同一視していたため。Option CはApproved / Appliedではなく、現行canonical仕様と実装は変更していない。最新の確定原則は「One Table surface. Independent source lifecycles. Shared semantic feedback. Operation-scoped Undo where appropriate.」である。通常Save targetの選択は未承認。
