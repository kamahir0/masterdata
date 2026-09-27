# GUI仕様: Unified Table Editor

Status: Approved

この文書は、Table schemaと選択record setを同じ編集面で扱うGUI interactionのownerである。field/value/type/key semantics、source-preserving mutation、lost-updateとrollbackは[Field Declaration Mutation](../../specs/field-declaration-mutation.md)、[Schema Migration v1](../../specs/schema-migration.md)、[Source Record Edit](../../specs/source-edit.md)へ委譲する。[仕様変更0041](../../spec-changes/0041-unified-table-editor.md)で旧schema-first Table Editor contractを置換した。

### GUI-UNIFIED-001

#### Table context

Workspace ExplorerでTable schemaまたはData sourceを選ぶと、main areaはlogical Tableの同じ編集面を表示しなければならない（MUST）。schema fieldsをcolumn header、選択sourceのrecord occurrenceをrowとして扱う。file path / filename / folderをTable identityとして扱ってはならない（MUST NOT）。schema選択時はinline recordsがあればそれを、なければ最初のData sourceを初期record setに選ぶ。record-bearing sourceがない場合もTable headerと空gridを表示し、Data source作成へ到達できなければならない（MUST）。

### GUI-UNIFIED-002

#### Record sets

同じTableに複数のrecord-bearing sourceがある場合、Table面のrecord-set selectorで切替えられなければならない（MUST）。選択元fileは初期record setを指定するだけであり、headerのTable contextは共通である。切替えで他fileのdirty buffer、history、selection、query入力を破棄、保存、混同してはならない（MUST NOT）。fileごとのSaveとConflict lifecycleは[Data Editor](../data-editor/spec.md)に従う。

### GUI-UNIFIED-003

#### Direct columns

field名はheaderそのものからpointerとkeyboardでinline renameでき、typeはheaderから選択でき、Nullable / Arrayは同じcolumn文脈で切替えられなければならない（MUST）。column末尾からAddできなければならない（MUST）。通常成功時にPlan / Diff / Applyの必須操作、rename/type変更modal、別のSchema Editorへの遷移を要求してはならない（MUST NOT）。field名の未確定入力は失敗時に保持し、対象headerに理由を示す。headerの値はshared Table snapshotに由来し、frontendでYAML parseやType Systemの推論を行ってはならない（MUST NOT）。

右端 `+` のdefaultはNullable string fieldとし、未使用MessagePack key候補はshared Applicationが選ぶ。existing recordsには明示的な`null` initializerをMigration commandへ渡す。候補keyとfield declarationはshared Planで最終検証しなければならない（MUST）。defaultを変更したい利用者は作成後にheaderから変更できる。

### GUI-UNIFIED-004

#### Schema draftと通常Save

type / Nullable / Arrayのheader操作は[Field Declaration Mutation](../../specs/field-declaration-mutation.md)の`FIELD-DECL-006..010`に従う可逆的なschema draftでなければならない（MUST）。headerは直ちに新宣言を表示し、shared Coreの再解釈によるcell/column/Problems diagnosticを更新する。既存valueが新宣言でinvalidでも操作を拒否せず、record source bytesを変換しない。Ctrl/Cmd+Zで未保存のtype変更を戻すと、sourceを変更せず該当diagnosticも消える。通常のschema Saveは対象schema fileだけを安全にpersistする。

rename/add/drop、MessagePack key、Key、Reference等のstructural / destructive / dependency operationは各既存Plan・dirty・authorization契約へ従う。frontendはPlan resultを自動承認する権限を持たず、stale画面から新しいsnapshotへ黙ってplanし直してはならない（MUST NOT）。schema draftとrecord draftは同時に存在でき、片方のSaveやsource切替えで他方を失ってはならない（MUST NOT）。

Save/operation失敗時は対象、理由、可能な修正またはDetailsを対象付近に表示し、Conflict、commit failure、Recovery Required、Outcome UnknownをSuccessと混同してはならない（MUST NOT）。Recovery Required中のcross-surface gateは[App shell](../app-shell.md)に従う。stale Planをsilent re-planしてcommitしてはならない（MUST NOT）。semantic diagnosticは操作失敗dialogではなく関連cell/column/Problemsで示す。

Drop FieldとReference Remove等のdestructive operationは明示confirmationとbackend authorizationを維持する。Reference add/edit/removeはshared source-preserving mutationを通し、normal column headerには常設しない。advanced Table detailから名前、source/target fields、resolved cardinality/optionality、C# helper name、diagnosticsを確認・編集できるようにする。frontendでReference semantic resolutionを再実装してはならない（MUST NOT）。

### GUI-UNIFIED-005

#### Grid composition

同じTable面のgridは[Data Editor](../data-editor/spec.md)と[Grid Authoring](../data-editor/grid-authoring.md)のcell / complex value / row / TSV / Undo/Redo / Save / Problems / virtualization contractを維持する。schema draftとrecord file-local dirty bufferは同じ編集面で区別して保持する。Undo/Redoを確定済みdisk変更のrollbackに見せかけてはならない（MUST NOT）。Build / Publish / Gitをauthoringに暗黙結合しない（MUST NOT）。

Key、Reference、MessagePack key、低頻度metadataは補助面へprogressively discloseする。通常時はTable名とcolumn / row dataを最も強く表示し、正常状態の説明を繰り返し常設しない。gridのkeyboard selection、field headerのtab順、operation失敗後のfocus復元はpointerなしでも成立しなければならない（MUST）。状態は色だけに依存せずassistive technologyへ伝える。

## Adapter boundary

Tauri frontendはschema Plan derivation、YAML patch、dependency resolution、lost-update preflight、rollbackを実装してはならない（MUST NOT）。shared Native Application ServiceをTauri command経由で呼び、frontendはTable contextとserialized intent / resultを扱う。complex value、record mutationも既存shared Application boundaryに従う。

## 検証

inline sourceと分離Data sourceの同等操作、複数source切替え、空Table、header直接操作、keyboard、dirty gate、stale、失敗・復旧、source-preserving、2千件以上のbounded row DOM、Problems移動をfocused testとDesktop実操作で確認する。
