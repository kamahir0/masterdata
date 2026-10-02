# Rewrite Acceptance Matrix

Status: Approved

architecture-freeなobservable acceptance。Sは [domain / safety](domain-safety.md)、Pは [performance budgets / protocol](performance.md) を参照する。詳細のsource形式 / public output oracleは [compatibility corpus](compatibility-corpus.md)。内部command名を期待結果にしない。[適用境界](README.md) に従う。

| Scenario / User intent | Observable expected behavior | Safety / performance | 既存evidence・移植するtest | 移植不要な旧拘束 |
| --- | --- | --- | --- | --- |
| Project open / 作業対象を開く | configured sourceを見つけ、Project identityを明示。invalid configを別Projectとして開かない | S config/path。P cold、初回projectionを分離 | `project_discovery.rs`、GUI `project-surfaces` / `creation`、showcase | CLIのdiscover/load順序、exact service constructor |
| Project close / switch / quit / 作業を終える | dirty stateにSave / Don't Save / Cancelを提示し、Cancelや保存失敗で作業を失わない。quitとbackground refreshを混同しない | S dirty / outcome / scope | GUI `project-surfaces`、0032/0033、Tauri quit guard tests | old dialog component名、全部のdirtyを暗黙保存 |
| Appearance / keyboard / IME / 作業しやすくする | Light/Dark/System、OS追従、user preference復帰、識別可能focus、色以外のstate表現。IME Enterをcommit移動に誤解釈しない | S user scope / lossless text。P 状態変更でgridを移動しない | `color-theme` / `user-state`、authoring keyboard、0038/0039 | localStorageをdurable authorityへ戻すこと、exact CSS token / provider |
| Explorer navigation / 対象を選ぶ | selected state / new targetが即時変化。pending時にold gridをnew targetとして出さない | P next-frame / warm。S read freshness | GUI `interactive-navigation` / `workspace-navigation`、app `workspace_session`、navigation Desktop | exact internal command / hook名 |
| Rapid A→B→C→D / 最後を編集する | Dへ到達。old result、diagnostics、focus intentで巻き戻らない | P bounded obsolete work。S dirty保持 | 同上、40-click Desktop、Tauri coalescing tests | exact11 reads / microtask / lock type |
| Table schema edit / 定義を直接直す | Table surfaceでname / type / modifierを編集。recordsを勝手にcoerceしない。依存diagnosticを更新 | S schema-directed / physical dirty | `field_declaration.rs`、`table_authoring.rs`、GUI `table-editor`、0041/0042 | separate schema page強制、exact draft DTO |
| Scalar edit / 値を直す | cell commitをbufferへ反映。既存PKもexact occurrenceで編集。invalidityは診断し、representable Saveを許可 | S lossless / occurrence / preservation。P local commit | core `source_edit` inline、GUI `authoring` / `editor-state` | batchのPK禁止をdirect editへ流用 |
| Complex Value / nested値を操作 | 開くこと自体でdirtyにしない。leaf / Array / nullable / unknown enumを直接操作し、closed surfaceで既存値を失わない | S lossless、unknown保持、Undo。P bounded controls | GUI `value-editor`、core complex/Array regressions、0045 | transaction footer Apply/Cancel必須、closed per-cell heavy mount |
| Add / Delete / Undo / 誤操作を戻す | Added Rows / Pending deleteと既存recordを区別。source-localでrevertできる。native text Undoを奪わない | S history / occurrence、disk rollbackではない | `source-record` inline tests、GUI `authoring` / `editor-state`、0044 | file全体Undo、exact store / capacity calibration |
| Row / Column / Array reorder / 順序を直す | 掴んだ対象が追従、neighborが移動。Dropで一操作、Escape / cancel / context lossで無変更 | S 三種類のorderの意味を区別。P stable geometry | core order tests、GUI `grid-reorder` / `value-editor`、Desktop geometry、0046/0048 | 挿入線表示、固定DOM順序、固定transform式 |
| Long / wide Table / 位置を見失わず編集 | header / row contextがsticky、名前とhandle非重複、外側overscroll gapなし。visible controlsは操作できる | P bounded DOM / frame、S focus / cancel | Desktop `desktop-e2e.mjs`、grid geometry、0049 | 20px grip / 40px toolbarをproduct SLAにすること |
| Inline / separate / mixed / 同じ意味を編集 | 保存先を正しく示し、inline schema+recordsを一physical candidateへ合成。inactive separate draftを混ぜない | S source topology / dirty scope | `inline_records.rs`、app `table_authoring`、GUI `table-editor` / navigation | topologyごとに別authoring product |
| Source switch / 作業を並行する | dirty/history/query/selectionをsourceごとに復帰。unsaved search draftも混同しない | S dirty overlay。P dirty再訪budget | GUI `interactive-navigation`、app workspace overlay tests | navigation cacheでdraftをreset |
| Clipboard paste / 手入力を減らす | TSVをlosslessly解釈しbufferへ一操作で反映。失敗はall-or-none、formulaを実行しない | S typed codec / no-op / history。P 10k cells | `authoring_batch` inline、GUI `authoring`、100k/10k input | direct pasteにもpreview Applyを必須化 |
| Save / 現Tableを確定 | current Table scopeを保存。既知Conflictはcommit前に止め、開始後はfileごとの結果を正確に示す | S fresh actual identity / partial outcome。P Save stage | `table_authoring` context save tests、GUI `project-surfaces`、0044 | 全record sourceを一括Save、revoked Option C |
| Save All / 全draftを確定 | 明示されたdirty全体を扱い、成功分だけbase更新。他draftやhistoryを誤消去しない | S config/binding / per-file outcomes | GUI `authoring` / `project-surfaces`、app source save tests | 全file crash-atomic保証 |
| Conflict / 外部更新と比較する | local draftを保持。比較対象identityを明示。reload / overwrite等の許可された選択をfreshに確認 | S lost-update、config overwrite禁止 | core/app save external mutation tests、GUI conflict tests | cache一致だけでoverwrite、source/config同一扱い |
| External clean / dirty change / 外部編集を取り込む | cleanはrefresh、dirtyはConflict。invalid / deleted / dependency missingはstale editable viewを出さない | S identity/generation。P targeted freshness | app `workspace_session`、GUI `interactive-navigation` | watcher framework必須、mtimeのみ |
| Invalid YAML / 壊れたsourceを理解する | parse diagnosticと対象を示し、古いcontentをcurrent editableにしない。unrelated draftを保持 | S parse/semantic区別 | `fixtures/invalid`をisolated case化、workspace invalid/delete tests | combined invalid fixtureでfirst errorだけをoracleにすること |
| Validation / 問題を理解する | pending / current / staleを区別。Problemsから正しいsource occurrenceへ移動し、diagnosticでdirtyを作らない | S shared semantics / generation。P background completion | core `validation.rs`、GUI Problems/navigation / stale validation tests | selectionごとのproject-wide validate |
| Source creation / rename / 移動する | typed initializer、collision / scope / physical identityを確認。成功後focus intentを一度だけ適用 | S exclusive / path / Unknown / dirty guard | source creation/core/app tests、GUI `creation`、path mutation inline、ab93c30 focus regression | exact old read API fixture adapter |
| Migration / 構造を変える | Plan / consequence / explicit authorization。changed input拒否、failure / Recovery Requiredを明示 | S closure fresh / recovery。ordinary editingへ強制しない | core migration families、app `type_authoring`、GUI workflow/type editor | exact Plan DTO / multi-file transaction実装 |
| Build / deliveryを生成する | saved source/config/profileからC# + binary + receipt。strict validation、再現可能、失敗時旧set保持 | S shared semantics / .NET delegation | `production_build.rs`、`build_pipeline.rs`、`generation.rs`、full fixture / .NET loader | Rust binary再実装、exact internal JSON report |
| Publish / 成功artifactを配布 | last receiptを使用。current dirty YAMLと独立。全destination preflight、unmanaged / .meta保持、partial failure明示 | S namespace / target outcome / fresh preview | app receipt/preflight/execution、CLI build/publish、Desktop delivery | implicit Build、Git client、Unity GUID生成 |
| Build profile / 保存済み範囲を理解 | profile / tagsのshared意味を保つ。必要なTable / Build文脈からphysical composition / inclusionへ到達 | S selection / no source mutation | build selection tests、GUI `project-surfaces` / tags | permanent Overview surface無条件維持 |

移植先はboundaryごとに最小のtestを選ぶ。domainの全edge caseをDesktop E2Eで再現せず、desktopはcross-layer wiring / focus / geometry / actual interactionを証明する。GUI testがmockする内部commandやDTOはsource compatibilityではない。

D1〜D6は [Human decision record](human-decisions.md) でResolved。legacy testのgreen自体をrewrite gateにしない。portable期待値と現行adapterの検証率は別であり、未実行のscenarioを合格扱いしない。

## Independent oracle binding

この表は上のworkflowを [portable v1 corpus](../../fixtures/rewrite-oracle/v1) の具体的input / intent / expectedへ結び付ける。Historical tests欄は参考であり、旧APIを呼べることはacceptanceではない。

| Scenario | 独立oracle | 検証境界 |
| --- | --- | --- |
| scalar / quote / comments / CRLF / block scalar / occurrence / nested64-bit | manifest byteScenarios | exact candidate bytes、拒否時input不変 |
| schema-directed null / number / string | interpretation.json、workflows schema-reinterpretation | 解釈・validity、record bytes無変更 |
| flow mapping compatibility | flow-parse / flow-unrelated / flow-local / flow-unsafe / new-mapping-block | D6の5条件、writer gapを免除しない |
| inline / separate / mixed / Save / cached identity Conflict | manifest saveScenarios | 全physical output bytes、committed paths、inactive除外 |
| no-record-source / schema / Complex / history / reorder / sticky / focus | workflows.json | visible event/state、pixel / component固定なし |
| Search / ordinary paste / Problems / Save All / external change / recovery | workflows.json | observable result、dirty / no mutation / outcome。fault seam別adapterが必要 |
| rapid navigation | workflows rapid-navigation | 50 selections、controlled obsolete response、last target、bounded work / parse0 |
| Migration | manifest structuralScenarios / workflows migration-stale-plan | Rename / Add / Dropのexact multi-source byte、closure identity・authorization・no mutation on stale |
| Build / Publish / public consumer | consumer/scenario.json、Consumer.cs、workflows build-publish | canonical fixture + reference input、compile / actual load / PK / SK / nested値 |
| large dataset | capacity.json | 100k row式、query result、10k paste全target / 非target、diagnostics、RSS / bounded viewport |

corpusのdeclarative scenarioはtest frameworkを指定しない。現adapterの実行結果と未接続範囲を [report](finalization-report.md) に明示する。production code無しでexpectedを理解できても、全Tier1 oracleが抽出・実行済みとは限らない。
