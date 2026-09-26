# Current Objective

## Role

この文書はHuman-selectedなcurrent priorityとDONE boundaryのownerである。observable semanticsはcanonical specification、resume地点は[Development State](execution-state.md)、implementation realityはcode / tests / Gitで確認する。

## Objective

**Desktop GUI authoringの第一マイルストーンとして、backend operation / guided form中心の操作モデルをdirect manipulation / spreadsheet-firstへ転換する。shared Rust core/applicationのvalidation、migration、lost-update protection、rollback / recoveryは堅牢なauthorityとして維持しつつ、その慎重さ・重さを通常のGUI手順へ露出させず、日常操作をExcel / Google SheetsやIDE Explorerに近い軽さで完結させる。**

## Completion slices

- Workspace Explorerの新規artifact作成を、kind選択 -> provisional node -> filename inline edit -> Enter commit / Escape cancelへ置換し、shared applicationがvalid starter sourceを構成する。
- Table / Data authoringをspreadsheet-firstへ再構成し、column headerでfield name / typeをdirect edit、右端`+`でfield追加、grid最下部`+`でrecord追加できる。
- record gridへrow virtualization / windowingを導入し、大規模MasterDataでもDOM量をrecord総数へ比例させない。selection、keyboard navigation、range、focus restorationはvirtualization下でも維持する。
- scalar / enum / bool等の日常値はinline、Array / Custom Type / Flags等の複雑値はcell anchoredな一時editorへprogressive disclosureする。
- routineなschema Migration Plan / Diff / Applyおよびbatch previewをmandatory UI stepにせず、shared application内部のsafety mechanismとして利用する。destructive、Conflict、Outcome Unknown、stale / blocked operation等、利用者判断が必要な場合だけblocking UIを前景化する。
- GUIのdirect操作に不足するshared capabilityとしてstarter source creationと`ChangeFieldType`をapplication/core boundaryへ実装し、frontendへYAML/domain semanticsを複製しない。
- 旧`SourceCreation` modal、Plan-first schema操作、preview-first paste等のdaily workflowを置換し、focused React / Rust regression evidenceとrepository checksを完了する。

## Canonical requirements

- [Source Creation](gui/source-creation/spec.md)
- [Workspace Explorer](gui/explorer/spec.md)
- [Data Editor](gui/data-editor/spec.md)
- [Data Editor Grid Authoring](gui/data-editor/grid-authoring.md)
- [Data Editor Record Mutation](gui/data-editor/record-mutation.md)
- [Table Editor](gui/table-editor/spec.md)
- [Schema Migration v1](specs/schema-migration.md)

## Explicit non-scope

- Build / Publish / Unity delivery workflowの再設計。
- YAML source format、Table / Type identity、MessagePack binary contractの変更。
- field reorder、MessagePack key direct editing、Primary / Secondary Keyの全面的direct manipulation。
- Type Editor全体のdirect-manipulation再設計。新規Type artifactはinline creation対象だが、作成後の全面UX再設計は後続Objectiveで扱ってよい。
- value coercion / conversionを伴う`ChangeFieldType`。
- Programmable View / Computed / Annotation column、Git collaboration feature。
