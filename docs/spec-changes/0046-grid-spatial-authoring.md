# 仕様変更0046: Grid Spatial Authoring

Status: Applied

## Affected Specifications

- `docs/gui/table-editor/spec.md`: viewport continuity、Column reorder / positional insert / context action
- `docs/gui/data-editor/record-mutation.md`: Row reorder / positional insertとquery制限
- `docs/gui/data-editor/grid-authoring.md`: source-local history、selection / focus / virtualization
- `docs/specs/source-record-mutation.md`: source record sequence order mutationとsource-preserving candidate
- `docs/specs/field-declaration-mutation.md`: declaration orderのschema draft
- `docs/specs/schema-migration.md`: AddFieldのposition指定。既存Plan / Apply安全境界は維持

## Source Evidence and Classification

- **Human-selected Objective / Requirement:** 長大・横長Tableで位置関係を維持し、Column / Row / Arrayをgrabして並べ替える。target contextからposition-relative Insert、keyboardからMove / Insertへ到達する。Row positional mutationはsource-order view限定。
- **Human Constraint:** Column reorderはschema declaration orderだけ、Row reorderは選択physical source内のpresentationだけ、Array reorderはvalue semantic mutation。0044 Save、0045 Complex Value commit、0042 scalar interpretationとMigration安全境界を維持する。
- **Approved authority:** `SCHEMA-TABLE-004`はfield declaration orderをGUI/C# presentationとし、source record orderをbinary semanticsにしない。`SOURCE-RECORD-005`はappend、`SOURCE-RECORD-008..010`はsource-preservingとunsafe shape拒否を定義する。`GUI-GRID-004..006`はsource-local historyとkeyboard precedenceを定義する。
- **Current implementation evidence:** headerはCSS stickyだがwide gridのbody row numberはhorizontal scrollで消える。2,201 rowではAdd Rowがsequence末尾にのみあり、viewport中間から届きにくい。Column/Row順序のauthoring requestはなく、Arrayはitem menuのMove Up/Downのみ。gridはbounded row windowを使う。

## Desktop spatial friction inventory（変更前、disposable project）

| Task | Current path / cost | Spatial friction | Direction |
| --- | --- | --- | --- |
| 2,201 rowを縦移動 | scroll | column headerは残る | 維持しstackingを検証 |
| 23 columnを横移動 | cell keyboard navigation | header `#`は残るがbody行番号は流れる | body row identityをsticky化 |
| 中間位置からAdd Row | source末尾へscrollまたはtoolbar menu | gridとの距離が大きい | grid viewport anchored append affordance |
| Column order | YAML手編集 | header内で完結しない | handle dragとmenu Move |
| Row order / positional Insert | YAML手編集 | source occurrenceから操作できない | row handle/menuとshared source patch |
| Array order | 各itemのmenu→Move Up/Down反復 | 遠いitemへの移動が長い | item handle drag、menu fallback維持 |
| 空record source | Add Rowがgrid末尾に現れる | 一操作で追加・先頭cell focus | 維持 |

## Confirmed Decisions / Agent Decisions

- **Human Decision:** source record orderはhuman-editable presentationであり、record identity、PK、binary orderではない。Row moveとInsert Above/Belowはsource-order viewでのみ可能。source間moveは対象外。
- **Agent Decision:** Searchも現在のquery implementationではrow subsetを作るため、search/filter/sortがactiveな間はRow positional actionを無効にする。query clearで再開する。visible adjacencyとsource adjacencyが違う可能性を許容しない。
- **Agent Decision:** Row move patchはrecord mapping本体を移し、item間の独立したcomment / blank separatorはそのsource位置に残す。mapping内部のcommentは本体と移す。境界を一意に分類できないshapeはcandidate生成前に拒否する。既存Deleteと同じくcomment ownershipを推測しない。
- **Agent Decision:** Column dragは独立handleからのみ開始し、cell range selection / rename / type selectionと競合させない。Row dragもrow header handleからのみ開始する。DesktopのWKWebViewでHTML native dragのdropが安定して届かなかったためpointer eventsでdropを扱い、edgeでcontrolled auto-scrollする。
- **Agent Decision:** dragはdropで一回だけcommitし、cancel / same-positionはno-op。keyboard menuは同じshared operationを呼ぶ。

## New / Changed Requirements

### GUI-SPATIAL-001 — Viewport continuity

通常Table gridの縦移動中はcolumn headerが見え、横移動中はrow occurrence番号とそのcontext actionへ到達できなければならない（MUST）。sticky層はgridのheader、popover、dropdown、Problems navigation、focusを妨げてはならない（MUST NOT）。normal state changeでgrid top edgeを動かさない。bounded row renderingと`aria-rowindex`を維持する。

### GUI-SPATIAL-002 — Append affordance

record sourceがありAdd Row supportedなら、source末尾がviewport外でもgridに属するappend actionへkeyboard/pointerで到達できなければならない（MUST）。末尾がviewport内なら末尾の通常位置と二重に強いactionを表示しない。常にselected sourceへappendし、Saveしない。record sourceがないTableで保存先を作らない。

### GUI-SPATIAL-003 — Column order and insertion

Column handleのdrag/drop、header context menu、keyboard context entryから、field declaration orderのMove Left/RightとInsert Left/Rightを実行できなければならない（MUST）。Column reorderはschema physical sourceのlocal draftで一操作のUndo/Redoとし、name/type/modifier/key/PK/SK/Reference/valueを変更しない（MUST NOT）。same-position dropはno-op。field Addのposition指定はshared Application/Coreで扱い、既存structural Migrationのpreflight/commit/recoveryを維持する。通常末尾`+`は維持する。

### GUI-SPATIAL-004 — Array item drag

Array item handleのdrag/dropは現在の0045 value operationへ一回だけreorderを渡し、一つのsource-local Undo単位にする（MUST）。cancel/no-opでは履歴もdirtyも増やさない。item menuのMove Up/Down/Removeとkeyboard到達性を維持し、nested item identity、diagnostic path、focusを正しいitemへ追従させる。

### SOURCE-RECORD-016 — Positional sequence authoring

選択中の一つのrecord-bearing physical source内で、existing occurrenceとAdded draftのsource sequenceを明示順序へ並べ替え、対象occurrenceの直前/直後へ既存Added Row shapeを挿入できなければならない（MUST）。occurrenceはbase indexまたはAdded draft identityで区別し、PK valueやquery ordinalで識別しない（MUST NOT）。mutation requestはsurviving existing occurrenceとadded draftを重複/欠落なく一度ずつ含む最終順序としてshared Coreが検証する。通常Add Rowは末尾appendのまま。

Row order mutationはfile-local draftであり、Saveまでworkspace bytesを書き換えない。existing value edit、Add、Pending delete、inline schema draftと一つのphysical candidateへcompositionし、exact base identityとpostconditionを検証する。semantic record valueや他source、binary canonical orderingを変えない。source whole-file reserializationは禁止。record内部のbytesはitemと共に動き、独立したseparatorは元位置に残す。安全なblock境界を特定できない場合はsourceを書かずに失敗する。

### GUI-SPATIAL-005 — Row context and query safety

source-order viewのrow headerからdrag reorder、Insert Above/Below、Move Up/Down、Deleteへpointerとkeyboardで到達できなければならない（MUST）。queryでsearch/filter/sortのいずれかがactiveならpositional mutationを拒否し、理由に到達できなければならない（MUST）。Pending delete occurrenceへInsert/Moveを適用しない。Delete / Undo DeleteとAdded draft削除の既存意味を維持する。drag中に全rowをmountせず、offscreen targetへcontrolled scrollで移動できる。

### GUI-SPATIAL-006 — Focus / selection / history

reorder / Insert後、移動または追加したcolumn/row/itemのfocusまたはactive contextが新位置へ追従しなければならない（MUST）。Problems navigationは移動後のoccurrenceを指す。range selection、clipboard、Shift+Arrow、Enter/F2、text control local Undoを維持する。drag移動中にhistoryを積まず、drop/Insert一回を一操作とし、no-opは履歴を増やさない。

## Compatibility Impact

既存YAML schema、record、binary format、identity、MessagePack keyに新しいsyntaxは追加しない。source record順をauthoring presentationとして変更可能にするが、logical dataset / Buildの意味は維持する。AddFieldの位置指定は既存末尾追加のdefaultを保つoptional intentとする。既存のsource-preserving / lost-update / Migration契約を弱めない。

## Implementation Impact / Acceptance

shared Coreにfield declaration source block reorderとrecord sequence order candidate、AddField positionを追加する。Applicationはrequest ownershipとpostconditionを担い、GUIはintentとresultを扱う。GUIはsticky body row number、viewport anchored Add Row、handle/context menu、drag marker/scroll、focus追従を追加する。0/1/2,001+ row、23+ column、inline/separate/mixed、separator comment/CRLF、duplicate PK、query制限、Undo/Redo、Save、Migration failure、selection/clipboard、nested Array、Desktop実操作を確認する。

## Potential ADRs / Open Questions / Approval Eligibility

Potential ADRs: None。既存source mutationとMigration boundaryを拡張する。

Open Questions: None。unsupported source shapeは`SOURCE-RECORD-010`と同様に拒否する。

Autonomous approval eligible: Yes。Human gate: None。Humanがproduct directionと範囲を選択済みであり、Agent Decisionは既存source safety内のreversibleな実装選択。仕様review後にapproval/applicationする。

## Review（proposal確定後のchallenge pass）

Blocking Issues: None identified。Non-blocking Issues: 既存のquery resultはsearchもrow subsetを作るため、search中もpositionを推測しない保守的制限を採用する。Comment separatorとliteral blockの境界はfocused source testsで実証する。Questions: None identified。Approved as Proposed: Yes。Autonomous approval eligibility: Eligible Yes; Human gate None。Intent fidelity、SCHEMA-TABLE-004とSOURCE-RECORD-008..010、0044/0045、Migration境界、failure semantics、testability、compatibility、document ownerを照合した。pending deleteへのposition actionを明示して曖昧さを除いた。

## Approval Record

Approval mode: Agent-autonomous。Basis: Human-selected Grid Spatial Authoring Objective、上記fresh review、non-breaking source-preserving extension。2026-09-29にcanonicalへ適用。Proposalとreviewは決定経緯として保持する。
