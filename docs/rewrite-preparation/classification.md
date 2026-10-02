# Classification Registry

この文書はPurification監査時点の分類・provenance。D1〜D6の未決定表現は0051でsuperseded。current owner / readinessは [finalization report](finalization-report.md) を参照する。

Status: Historical Evidence

重要なcontract / decision / test群63項目の監査。current ownerのStatusは基準HEAD時点。`DELETE`はfuture rewrite inputから除外する候補で、今のfile/testを削除する指示ではない。[適用境界](README.md) に従う。Human由来は明示記録がある場合だけ使用し、Agent commitのauthor名から推定しない。package approvalは全widgetの個別Human要求を意味しない。

## A01 — YAML source authority

Classification: **KEEP**

- Artifact / Current owner / Current status: [source edit](../specs/source-edit.md) / ADR0001; Approved / Accepted。
- Provenance: Vision・初期Human direction。Protects / Why it exists: 人が読み、外部編集・Gitで扱えるsourceを守る。
- Reason: cacheをauthorityにすると外部変更とreview可能性が失われる。Risk if retained: 派生read modelまで禁止と誤読すること。Risk if removed: opaque storageへ置換しsourceを失うこと。
- Proposed rewrite contract: [sourceと意味論](domain-safety.md#sourceと意味論)。Acceptance evidence to retain: source/config bytes、external edit cases。Human decision required: No。

## A02 — Shared semantics / native MasterMemory delegation

Classification: **KEEP**

- Artifact / Current owner / Current status: ADR0002 / ADR0003; Accepted。
- Provenance: Human-selected architecture。具体的crate構成はAgent実装。Protects / Why it exists: GUI/CLIの同じ意味と実MasterMemory consumerを守る。
- Reason: frontend/domain fork・binary internals再実装を避ける。Risk if retained: 同じlifecycle / transportまで要求する誤読。Risk if removed: 型・reference・binary semanticsの分裂。
- Proposed rewrite contract: 共有意味論のみを制約、internal API自由。Acceptance evidence to retain: type tests、.NET compile/load、generated consumer。Human decision required: No。

## A03 — Table / Type / Key / Reference semantics

Classification: **KEEP**

- Artifact / Current owner / Current status: [types](../specs/type-system/README.md), table-and-keys / index-and-reference; Approved。
- Provenance: 0001/0003/0023 Human decision、詳細はcanonical refinement。Protects / Why it exists: nominal identity、modifier、PK/SK、Required/Nullable、unique/non-unique。
- Reason: 意味論と公開C# APIは利用者が依存する。Risk if retained: 現Rust enumやresolver topologyまで固定すること。Risk if removed: 異なるdata / consumer意味を生成すること。
- Proposed rewrite contract: [domain](domain-safety.md)、canonical valid/invalid matrix。Acceptance evidence to retain: table/type/reference/generation tests。Human decision required: No。

## A04 — Source-preserving mutation / lexical preservation

Classification: **KEEP**

- Artifact / Current owner / Current status: source-edit / field-declaration-mutation; Approved。
- Provenance: 0015 Human preservation choice、0042 Human source semantics、bug regressions。Protects / Why it exists: 対象外bytes、quotes、comments、CRLF、ambiguous location拒否。
- Reason: 編集で無関係なdiffやvalue lossを作らない。Risk if retained: patch scanner / ASTの固定。Risk if removed: 全体serializationでsource破壊。
- Proposed rewrite contract: 局所intent・safe failure・byte oracle。Acceptance evidence to retain: core source preservation / config / literal scalar cases。Human decision required: No。

## A05 — Semantic invalidity vs Saveability

Classification: **KEEP**

- Artifact / Current owner / Current status: [source edit](../specs/source-edit.md); Approved。
- Provenance: 0042 Human-approved semantics、初期authoring decision。Protects / Why it exists: invalid途中状態の編集・保存とstrict Buildを分離。
- Reason: 修復前にdraftを失わず作業できる。Risk if retained: 全syntax failureもSave可と誤読。Risk if removed: duplicate / mismatchのdraftを保存不能にする。
- Proposed rewrite contract: representable mutationとsemantic validityを別判定。Acceptance evidence to retain: domain-invalid scalar、duplicate key、schema mismatch tests。Human decision required: No。

## A06 — Fresh identity / Conflict / overwrite

Classification: **KEEP**

- Artifact / Current owner / Current status: source-edit / project-config-edit; Approved。
- Provenance: lost-update safety decisionとrace regression。Protects / Why it exists: actual bytesとexpected identity、local保持、明示確認後もfresh recheck。
- Reason: 外部editor/Gitと同時使用できる。Risk if retained: read cacheをcommit authorizationに代用。Risk if removed: silent lost update。
- Proposed rewrite contract: [authoritative writes](domain-safety.md#authoritative-writes)。Acceptance evidence to retain: cached identity後external mutation、backup bytes race。Human decision required: No。

## A07 — Outcome Unknown / Recovery Required

Classification: **KEEP**

- Artifact / Current owner / Current status: schema/type migration / source-edit; Approved。
- Provenance: 0014 Human mutation gate、fault regressions。Protects / Why it exists: uncertain commit、OLD/NEW/mixed、no automatic retry。
- Reason: I/O failureを成功/失敗の二値へ潰さない。Risk if retained: general recovery database / 全file crash atomicityまで追加。Risk if removed: double apply、mixed sourceでwritesを継続。
- Proposed rewrite contract: canonical outcome / recovery gate、mechanism自由。Acceptance evidence to retain: migration_commit、Save Unknown、Recovery GUI。Human decision required: No。

## A08 — Current Table Save / Save All scope

Classification: **KEEP**

- Artifact / Current owner / Current status: [0044](../spec-changes/0044-table-authoring-save-history.md), Table editor; Applied / Approved。
- Provenance: 最新Human fourth-option decision。旧Option Cはrevoked。Protects / Why it exists: dirty schema+selected record、inline composition、inactive保持。
- Reason: 画面contextとphysical lifecycleを両立。Risk if retained: project-wide Saveへ安易統合。Risk if removed: 意図しない別source保存・draft消失。
- Proposed rewrite contract: current scope / preflight / per-file result。Acceptance evidence to retain: app context save 5 cases、GUI Save/All。Human decision required: No。

## A09 — Inline / separate / mixed / empty

Classification: **KEEP**

- Artifact / Current owner / Current status: table-and-keys / source creation; Approved / Implemented。
- Provenance: 0037 Human-selected inline、Agent mixed/empty refinement。Protects / Why it exists: source topologyとphysical composition。
- Reason: 既存projectを勝手に別形式へ変換しない。Risk if retained: topologyごとに別UIを再生成。Risk if removed: inline records欠落・duplicate commit。
- Proposed rewrite contract: 同じauthoring、別physical ownership。Acceptance evidence to retain: inline_records、mixed save、creation fixtures。Human decision required: No。

## A10 — Dirty / local history / query preservation

Classification: **KEEP**

- Artifact / Current owner / Current status: grid-authoring / source-edit / Table editor; Approved。
- Provenance: 0016 package Human approval、0044 / 0050、regressions。Protects / Why it exists: source-local draft/history、Added/Pending、native input Undo。
- Reason: navigationやrefreshは保存・破棄の意図ではない。Risk if retained: exact store / capacityを永久固定。Risk if removed: draft / Undo / searchを失う。
- Proposed rewrite contract: local overlayとbaseを分離、disk rollbackではない。Acceptance evidence to retain: editor-state、interactive-navigation、schema draft tests。Human decision required: No。

## A11 — Lossless frontend 64-bit boundary

Classification: **KEEP**

- Artifact / Current owner / Current status: primitives / GUI data editor; Approved。
- Provenance: domain/public consumer + GUI regression。Protects / Why it exists: int64/uint64のexact value。
- Reason: JS Number丸めはsilent data corruption。Risk if retained: string DTOだけを唯一手段にする。Risk if removed: 値の不可逆丸め。
- Proposed rewrite contract: end-to-end exact transport / edit / Save。Acceptance evidence to retain: ulong max、negative int64、nested integer cases。Human decision required: No。

## A12 — Occurrence identity != PK

Classification: **KEEP**

- Artifact / Current owner / Current status: source-record-mutation / source-edit; Approved。
- Provenance: 0019 Human PK editing、duplicate/Array regressions。Protects / Why it exists: duplicate PK / 同値itemの正しい対象選択。
- Reason: 編集中key変更やduplicateを安全に扱う。Risk if retained: 現locator / row ID構造固定。Risk if removed: 別occurrenceを変更する。
- Proposed rewrite contract: source occurrenceを識別、stale locator拒否。Acceptance evidence to retain: duplicate-key edit/delete、lexical Array reorder。Human decision required: No。

## A13 — Row / Column / Array orderの区別

Classification: **KEEP**

- Artifact / Current owner / Current status: record/field mutation / grid authoring; Approved。
- Provenance: 0046 Human spatial editing、Agent semantic refinement。Protects / Why it exists: presentationとserialization key / value orderを分離。
- Reason: 見た目の移動でconsumer意味を変えない。Risk if retained: source orderをBuild順へ固定。Risk if removed: PK/Reference/MessagePack keyの意図しない変更。
- Proposed rewrite contract: [local authoring](domain-safety.md#local-authoringとread-lifetime)。Acceptance evidence to retain: record/field/Array reorder、canonical Build order。Human decision required: No。

## A14 — Project/config/path identityとuser preference scope

Classification: **KEEP**

- Artifact / Current owner / Current status: project-layout / config-edit / source-path-mutation / GUI color-theme; Approved / Implemented。
- Provenance: 0005/0006/0012/0020/0038/0039 Human/Agent記録。Protects / Why it exists: logical/physical identity、configured roots、symlink、persistence scope、Light/Dark/Systemとnative user authority。
- Reason: 別fileへのwrite・machine-local意味の混入を防ぐ。Risk if retained: path layoutやcurrent resolver方式まで固定。Risk if removed: escape / wrong destination / config lost update。
- Proposed rewrite contract: fresh binding/path safety、project/local/user分離。Acceptance evidence to retain: config patch、case-only rename、alias / external root tests。Human decision required: No。

## A15 — Migration authorization / closure freshness

Classification: **KEEP**

- Artifact / Current owner / Current status: schema-migration / type-migration; Approved。
- Provenance: 0011 Human、RFC0006 Human option、0025 Agent derived。Protects / Why it exists: explicit Plan / authorization、changed dependencies拒否。
- Reason: 構造変更の影響とstale planを扱う。Risk if retained: ordinary editにもPlan/Applyを強制。Risk if removed: destructive effectを隠す・stale patch適用。
- Proposed rewrite contract: [Migration boundary](domain-safety.md#migration--build--publish)。Acceptance evidence to retain: migration families、stale config / source membership tests。Human decision required: No。

## A16 — Deterministic Build / coherent artifact set

Classification: **KEEP**

- Artifact / Current owner / Current status: build-pipeline / build-selection; Approved。
- Provenance: 0004/0007 Human artifact choice、production regressions。Protects / Why it exists: saved inputs、canonical order、C#/binary/receipt coherence。
- Reason: 同じ意味から再現可能delivery、failure時旧set保持。Risk if retained: exact internal bridge reportをpublic化。Risk if removed: 部分artifact更新・dirty source暗黙保存。
- Proposed rewrite contract: strict Build、native .NET、Publish非連鎖。Acceptance evidence to retain: production_build、build_pipeline、full fixture loader。Human decision required: No。

## A17 — Publish boundary / namespace / Unity metadata

Classification: **KEEP**

- Artifact / Current owner / Current status: build request preview / Unity integration; Approved。
- Provenance: 0006/0008 Human safety、0028 Human Unity Option B。Protects / Why it exists: last receipt、all-target preflight、unmanaged/.meta、partial result。
- Reason: 配布先の他assetを壊さずBuildと独立に配布。Risk if retained: 現transaction directory shape固定。Risk if removed: unmanaged削除・namespace overwrite・GUID破壊。
- Proposed rewrite contract: fresh destination / receipt確認、target-local outcomes。Acceptance evidence to retain: receipt、publish_preflight / execution、Desktop stale Publish。Human decision required: No。

## A18 — Public CLI / generated C# / Unity surface

Classification: **KEEP**

- Artifact / Current owner / Current status: CLI / C# naming / Unity integration; Approved。
- Provenance: 0011/0012/0023/0028 Human choices。Protects / Why it exists: documented automationとconsumer API compatibility。
- Reason: private topologyとpublic利用者境界は別。Risk if retained: undocumented全stdout fieldも永久固定。Risk if removed: automation / consumerを無断break。
- Proposed rewrite contract: documented public surface保持、specific breakはgate。Acceptance evidence to retain: CLI integration、codegen compile/load、Unity package inputs。Human decision required: No。

## A19 — Tags / profile inclusion semantics

Classification: **KEEP**

- Artifact / Current owner / Current status: source-tag-edit / build-selection; Approved。
- Provenance: 0017 package approval、Build selection domain。Protects / Why it exists: tagsはmetadata、selectionはshared意味、source bytes保持。
- Reason: Build inclusionがGUI独自判断で変わらない。Risk if retained: tags UI placementを固定。Risk if removed: different binary set / metadata破壊。
- Proposed rewrite contract: semantic setとlexical source orderを区別。Acceptance evidence to retain: tag tests / prod-dev profile / no source mutation。Human decision required: No。

## A20 — Typed source creation / initializer

Classification: **KEEP**

- Artifact / Current owner / Current status: source-creation / typed-initializer / project-init; Implemented / Approved。
- Provenance: 0013/0015/0034/0037 Human + Agent refinement。Protects / Why it exists: unset/default/null、lossless、exclusive creation。
- Reason: GUI自由入力でdomain意味を複製しない。Risk if retained: exact creation dialog path固定。Risk if removed: wrong type / source collision overwrite。
- Proposed rewrite contract: shared typed capability、safe destination。Acceptance evidence to retain: core/app creation、GUI creation/request tests。Human decision required: No。

## A21 — Diagnostics generation / Problems navigation

Classification: **KEEP**

- Artifact / Current owner / Current status: GUI data validation / Explorer nav; Approved。
- Provenance: 0050 Human direction、stale diagnostic regressions。Protects / Why it exists: pending/current distinction、right source occurrenceへ移動。
- Reason: old resultをnew sourceの問題にしない。Risk if retained: 全project validateをfirst paintに要求。Risk if removed: stale診断と誤target。
- Proposed rewrite contract: local shapeとbackground project validation分離。Acceptance evidence to retain: interactive-navigation stale validation / Problems。Human decision required: No。

## A22 — Direct Complex Value / focus / native Undo

Classification: **KEEP**

- Artifact / Current owner / Current status: [0045](../spec-changes/0045-direct-complex-value-authoring.md); Applied。
- Provenance: Human direct operation、commit detailsはAgent Decision。Protects / Why it exists: openはno-op、operation即local、closeで既commitを撤回しない。
- Reason: 普通のvalue editにtransaction儀式を要求しない。Risk if retained: 全部のcommit key / widgetをpixel固定。Risk if removed: 旧footer workflow復活・値消失。
- Proposed rewrite contract: [acceptance matrix](acceptance-matrix.md) Complex scenario。Acceptance evidence to retain: value-editor / authoring / nested focus cases。Human decision required: No。

## A23 — Sticky spatial context / position editing

Classification: **KEEP**

- Artifact / Current owner / Current status: [0046](../spec-changes/0046-grid-spatial-authoring.md); Applied。
- Provenance: Human long/wide usability要求。Protects / Why it exists: row/header context、offscreen append、grab、keyboard/focus。
- Reason: large Tableでも対象を見失わない。Risk if retained: specific virtualizer / PointerEvent implementation固定。Risk if removed: long/wideで編集不能。
- Proposed rewrite contract: sticky context / contextual action / bounded projection。Acceptance evidence to retain: Desktop geometry、grid reorder、source order tests。Human decision required: No。

## A24 — Whole target drag preview / cancellation

Classification: **KEEP**

- Artifact / Current owner / Current status: [0048](../spec-changes/0048-grid-drag-preview.md); Applied。
- Provenance: Human whole-column/row追従、Agent axis/ghost/cancel。Protects / Why it exists: neighbor relocation、Drop一回、cancel無変更。
- Reason: ドラッグ結果を予測し誤操作を戻せる。Risk if retained: transform-only DOMやexact animation固定。Risk if removed: 二重commit、誤順序、旧insertion-line復活。
- Proposed rewrite contract: observable geometryと操作安全性。Acceptance evidence to retain: grid-reorder / Desktop cancel / Array source tests。Human decision required: No。

## A25 — Compact chrome / non-overlap / overscroll

Classification: **KEEP**

- Artifact / Current owner / Current status: [0049](../spec-changes/0049-table-chrome-density.md); Applied。
- Provenance: Human screenshot feedback、Legacy reference許可。Protects / Why it exists: 名前/handle非重複、外側gapなし、toolbar一列。
- Reason: editor spaceとpointer対象を守る。Risk if retained: exact20/40pxへ昇格。Risk if removed: 重なり・空白・無駄なheaderの再発。
- Proposed rewrite contract: [UX principles](constitution.md#desktop-ux-principles)。Acceptance evidence to retain: Desktop header/toolbar geometry、screenshots由来acceptance。Human decision required: No。

## A26 — Interactive read / strict write分離

Classification: **KEEP**

- Artifact / Current owner / Current status: ADR0008 / 0050; Accepted / Applied。
- Provenance: Human navigation redesign、phase/actual Desktop evidence。Protects / Why it exists: view selection、read generation、latest wins、no warm full rebuild。
- Reason: one-shot lifecycleがinteractive latencyを阻害した。Risk if retained: exact session / command / lockをKEEP扱い。Risk if removed: 再parse・UI freeze・cacheによるunsafe Save。
- Proposed rewrite contract: [Performance constitution](performance.md)。Acceptance evidence to retain: workspace phase/safety、rapid Desktop、external mutation。Human decision required: No。

## A27 — Fixed regression / performance inputs

Classification: **KEEP**

- Artifact / Current owner / Current status: fixtures / examples / evidence; fixed assets / Historical Evidence。
- Provenance: bug history、100k/10k、0050 measured fixture。Protects / Why it exists: 同じ入力でsemantic/capacity/latencyを比較する。
- Reason: rewriteの都合で簡単なinputへ変えない。Risk if retained: current harness API / all outputsを無審査固定。Risk if removed: edge caseと比較可能性を失う。
- Proposed rewrite contract: [corpus](compatibility-corpus.md) input + explicit oracle。Acceptance evidence to retain: 12k/2k×20、100k/10k、inline/separate/mixed。Human decision required: No。

## A28 — VisionのCore / CLI-first記述

Classification: **REFINE**

- Artifact / Current owner / Current status: [Vision](../product/vision.md); Draft。
- Provenance: 初期product decomposition + 最新Human Desktop direction。Protects / Why it exists: YAML/Git/Unity価値とautomation。
- Reason: shared semanticsは良いがdaily productをcrate導入順で説明しない。Risk if retained: CLI frontendを再生成。Risk if removed: automation価値まで捨てる。
- Proposed rewrite contract: [Product Constitution](constitution.md)。Acceptance evidence to retain: Vision + 0040/0041/0050 intent。Human decision required: No。

## A29 — Exact crate / service / internal command API

Classification: **REFINE**

- Artifact / Current owner / Current status: ADR0002/0003、CLI shared application記述; Accepted / Approved。
- Provenance: Agent architecture/API naming。Protects / Why it exists: 共有domain boundaryとin-process利用。
- Reason: exact型名はproduct authorityではない。Risk if retained: NativeApplicationServiceのone-shotをDesktopへ強制。Risk if removed: shared semanticsの保護を失う。
- Proposed rewrite contract: boundaryだけ保持、lifecycle / transport自由。Acceptance evidence to retain: shared derivation / source semantics integration。Human decision required: No。

## A30 — Initial slice Non-goalsの残留

Classification: **REFINE**

- Artifact / Current owner / Current status: GUI record-mutation / data-editor / Explorer; Approved文書内prose。
- Provenance: 初期限定範囲、後続0019/0040/0044/source creation。Protects / Why it exists: 段階的deliveryの経緯。
- Reason: no general Undo / key edit / creation等の旧説明は最新ownerと分ける。Risk if retained: 現行機能を禁止する古い範囲を復活。Risk if removed: current Undo/save/creation契約まで削除。
- Proposed rewrite contract: superseded proseをhistoryへrouteする適用案。Acceptance evidence to retain: later grid history / existing key / source creation tests。Human decision required: No。

## A31 — App shell / workbench placement固定

Classification: **REFINE**

- Artifact / Current owner / Current status: GUI app-shell / project-workflow; Approved。
- Provenance: 0035/0036 Agent UX refinement、0041+後続Human。Protects / Why it exists: 見つける/編集するcontext。
- Reason: logical list capabilityとleft source tree placementを混同しない。Risk if retained: old hierarchy / parallel editing pages再生成。Risk if removed: Find / contextを失う。
- Proposed rewrite contract: one Tableとquiet surface、入口はobservable intentで評価。Acceptance evidence to retain: project/navigation/creation tests。Human decision required: No。

## A32 — Exact pixel / geometry calibration

Classification: **REFINE**

- Artifact / Current owner / Current status: GUI specs/tests / 0049; Approved / Applied。
- Provenance: Human non-overlap、exact数値はAgent calibration。Protects / Why it exists: 名前・handle・sticky/toolbarの関係。
- Reason: 20px / 40pxをHuman explicit要求にしない。Risk if retained: 次frameworkのlayout自由度を奪う。Risk if removed: 実geometry regression knowledgeを失う。
- Proposed rewrite contract: relative geometry / no overlap / stable surface。Acceptance evidence to retain: Desktop測定方法・uneven width cases。Human decision required: No。

## A33 — GUI source-text boundary tests

Classification: **REFINE**

- Artifact / Current owner / Current status: apps/gui/tests/app.test.mjs; current tests。
- Provenance: Agent smoke mechanism、safety意図。Protects / Why it exists: dirty / lossless / shared boundary / least privilege。
- Reason: regexは意味を充分証明せずrenameでfalse failure。Risk if retained: old App.tsx topology強制。Risk if removed: architecture safety checkを全廃。
- Proposed rewrite contract: 意味のboundary audit + observable testsへport。Acceptance evidence to retain: 13 test意図、actual permission audit。Human decision required: No。

## A34 — GUI exact component / mocked DTO tests

Classification: **REFINE**

- Artifact / Current owner / Current status: 15 GUI test files; current。
- Provenance: feature/bug tests + internal adapter convenience。Protects / Why it exists: observable authoringとfailure UX。
- Reason: test wiringはpublic compatibilityではない。Risk if retained: component/DTO構成再現。Risk if removed: UX regression oracle喪失。
- Proposed rewrite contract: [test proposal](test-purification.md) role分解。Acceptance evidence to retain: intent / rendered interaction / outcome assertions。Human decision required: No。

## A35 — Reorder transform / unchanged DOM order

Classification: **REFINE**

- Artifact / Current owner / Current status: grid-reorder.test.ts; current。
- Provenance: 0048 preview実装のcalibration。Protects / Why it exists: drag geometry、ghost非interactive、cancel。
- Reason: 固定DOM順序はmechanism。Risk if retained: transform-only旧algorithm強制。Risk if removed: preview/cancel edge casesを失う。
- Proposed rewrite contract: geometryとone commitのみport。Acceptance evidence to retain: uneven widths、virtual unmount、context loss。Human decision required: No。

## A36 — Navigation session types / lock / generation fields

Classification: **REFINE**

- Artifact / Current owner / Current status: ADR0008 / app+Tauri+frontend tests; Accepted / current。
- Provenance: 0050 Agent synchronization decision。Protects / Why it exists: reusable read、short publication、latest wins。
- Reason: immutable principleとexact implementationを分ける。Risk if retained: global lock/API/state treeを再生成。Risk if removed: ownership・freshnessの知識喪失。
- Proposed rewrite contract: generation semanticsとbounded work、exact型自由。Acceptance evidence to retain: phase / cached save / stale response tests。Human decision required: No。

## A37 — Rapid stress testの弱いread-count上限

Classification: **REFINE**

- Artifact / Current owner / Current status: navigation-desktop.mjs; current。
- Provenance: 0050 actual stress instrumentation。Protects / Why it exists: obsolete work bounded / latest target。
- Reason: reads<=clicksだけでは1click1readを許す。Risk if retained: coalescing無しもgreen。Risk if removed: real Desktop stress evidenceを捨てる。
- Proposed rewrite contract: controlled pending drainとwork count oracle。Acceptance evidence to retain: 40 synthetic clicks sample、deterministic latest-pending tests。Human decision required: No。

## A38 — 100k one-shot performance harness API

Classification: **REFINE**

- Artifact / Current owner / Current status: desktop_v1_performance.rs; current。
- Provenance: Desktop v1 capacity evidence。Protects / Why it exists: fixed dimensions、RSS、bulk cost。
- Reason: one-shot load/queryはwarm interactionのoracleではない。Risk if retained: full reloadを互換APIとして再生成。Risk if removed: capacity / paste regression入力喪失。
- Proposed rewrite contract: input維持、結果oracleとend-to-endを追加。Acceptance evidence to retain: 100k/10files/20cols/10k cells。Human decision required: No。

## A39 — YAML parser library RFCの未解決事項

Classification: **REFINE**

- Artifact / Current owner / Current status: [RFC0002](../rfcs/0002-yaml-parser-library.md); Proposed。
- Provenance: 初期library survey / Agent recommendation。Protects / Why it exists: subset遵守・source preservationの判断。
- Reason: dated library recommendationと既解決preservation課題をcurrent gateにしない。Risk if retained: serde_yaml version固定・古いopen question復活。Risk if removed: parserに必要なcompatibility条件喪失。
- Proposed rewrite contract: library自由、subset/bytes oracleを優先。Acceptance evidence to retain: canonical YAML / source edit / D6。Human decision required: No（D6は別項目）。

## A40 — RFC0005初期readiness / 限定edit範囲

Classification: **REFINE**

- Artifact / Current owner / Current status: [RFC0005](../rfcs/0005-first-record-authoring-experience.md); Draft。
- Provenance: 記録された初期Human primitive非key/file-save decision。Protects / Why it exists: direct editing出発点と安全性議論。
- Reason: 後続key/complex/history/Table Saveで範囲が拡張。Risk if retained: Required Primitive non-key限定を復活。Risk if removed: 最初のsafety rationaleを失う。
- Proposed rewrite contract: 最新canonical範囲へroute、古いreadinessはhistory。Acceptance evidence to retain: source identity / preservation / current edit cases。Human decision required: No。

## A41 — RFC0008 P1/P2/P3とlater roadmap

Classification: **REFINE**

- Artifact / Current owner / Current status: [RFC0008](../rfcs/0008-authoring-system-v1.md); Accepted。
- Provenance: Human package approval、後段計画。Protects / Why it exists: daily Desktopを一体で考えたintent。
- Reason: later key/source move実装・Web/Computed退役後に旧roadmapを使わない。Risk if retained: obsolete future機能を再生成。Risk if removed: workflow integration知識喪失。
- Proposed rewrite contract: current acceptanceへ抽出、段階計画はhistory。Acceptance evidence to retain: 0016–0018とlater retirements。Human decision required: No。

## A42 — Combined invalid fixture oracle

Classification: **REFINE**

- Artifact / Current owner / Current status: fixtures/invalid / discovery tests; fixed/current。
- Provenance: negative domain regressions。Protects / Why it exists: malformed / duplicate / invalid config。
- Reason: first config failureだけでは各sourceの拒否を証明しない。Risk if retained: false confidence・error masking。Risk if removed: negative source corpus喪失。
- Proposed rewrite contract: isolated input + phase/diagnostic/no-write oracle。Acceptance evidence to retain: invalid YAML / unknown kind / invalid names。Human decision required: No。

## A43 — Build golden / internal builder wire assertions

Classification: **REFINE**

- Artifact / Current owner / Current status: production_build / generation / bridge tests; current。
- Provenance: consumer integration + internal adapter design。Protects / Why it exists: public C#、loaded binary、coherent receipt。
- Reason: 全JSON/formattingとpublic APIを分離。Risk if retained: private protocolを永久互換化。Risk if removed: public API・binary semanticsを壊す。
- Proposed rewrite contract: public compile/load + semantic oracle。Acceptance evidence to retain: full fixture actual .NET、canonical ordering。Human decision required: No。

## A44 — Glossary / Draft schemaのAST・hash-cache記述

Classification: **REFINE**

- Artifact / Current owner / Current status: product terminology / schema-language; product prose / Draft。
- Provenance: 初期typed AST / build-plan design。Protects / Why it exists: 語彙とschemaの意味。
- Reason: future cache/hashやRust ASTはproduct contractでない。Risk if retained: 旧data model構成を必須化。Risk if removed: semantic用語も曖昧になる。
- Proposed rewrite contract: semantic termsを保持、design例はhistory。Acceptance evidence to retain: canonical Table/Type/Build semantics。Human decision required: No。

## A45 — Unity static checker / evidence strength

Classification: **REFINE**

- Artifact / Current owner / Current status: xtask unity_package / Unity package; current。
- Provenance: 0028 integration implementation。Protects / Why it exists: public package境界・metadata ownership。
- Reason: source string scanはcompile/runtime証明ではない。Risk if retained: static greenをactual Unity互換性と誤認。Risk if removed: package identity / APIを見落とす。
- Proposed rewrite contract: consumer compile/runtime oracleを追加する提案。Acceptance evidence to retain: package source / .NET generated API、現static checks。Human decision required: No。

## A46 — 0047 insertion-line refinement

Classification: **DEMOTE TO HISTORY**

- Artifact / Current owner / Current status: [0047](../spec-changes/0047-grid-drop-boundaries.md); Applied history。
- Provenance: Human Legacy reference・duplicate境界line feedback。Protects / Why it exists: 境界の一意性・geometry bug knowledge。
- Reason: 0048でline presentationを置換。Risk if retained: lineとwhole previewを同時要求。Risk if removed: boundary ambiguityの教訓を失う。
- Proposed rewrite contract: current presentationは0048、historyを保存。Acceptance evidence to retain: duplicate boundaries / header extent bugの由来。Human decision required: No。

## A47 — Complex Value旧transaction footer

Classification: **DEMOTE TO HISTORY**

- Artifact / Current owner / Current status: 0015 / RFC0007 / earlier UI; Applied / Accepted history。
- Provenance: Human初期Option C、後続0045 Human。Protects / Why it exists: typed nested編集の導入。
- Reason: typed semanticsはKEEP、旧Apply/Cancel topologyはsuperseded。Risk if retained: ordinary editにtransaction儀式を再導入。Risk if removed: typed/null/unknown preservationまで失う。
- Proposed rewrite contract: 0045 direct operationへroute。Acceptance evidence to retain: old edge inputs、current direct operation tests。Human decision required: No。

## A48 — 0044 revoked Option C

Classification: **DEMOTE TO HISTORY**

- Artifact / Current owner / Current status: [0044 history](../spec-changes/0044-table-authoring-save-history.md); revoked intermediate。
- Provenance: d09ba38→d7fbb67→4193b6e Human fourth-option。Protects / Why it exists: Save/history選択の理由。
- Reason: 最新scopeと旧physical-only案を同時currentにしない。Risk if retained: wrong Save scopeを復活。Risk if removed: physical ownershipの理由喪失。
- Proposed rewrite contract: A08だけcurrent候補、revocation chain保持。Acceptance evidence to retain: Table Save mixed/inline/selected cases。Human decision required: No。

## A49 — Web / Native Host / WASM product direction

Classification: **DEMOTE TO HISTORY**

- Artifact / Current owner / Current status: ADR0006 / RFC0004 / 0009–0010 / runtime-hosts; Superseded / Deprecated。
- Provenance: Human初期選択→0022 Human retirement、0021 Rejected。Protects / Why it exists: 退役したruntime選択経緯。
- Reason: 現在Desktop productに戻さない。Risk if retained: native-host handshake / Web lifecycle再生成。Risk if removed: 退役理由を忘れる。
- Proposed rewrite contract: non-goal境界のみcurrent候補。Acceptance evidence to retain: 0022 / rejected0021。Human decision required: No。

## A50 — Released Compatibility / impact subset

Classification: **DEMOTE TO HISTORY**

- Artifact / Current owner / Current status: 0024 / 0026; Applied history、0030 retirement。
- Provenance: Human retirement。Protects / Why it exists: 互換性提案の経緯。
- Reason: source safetyやpublic互換性とretired release comparison engineは別。Risk if retained: retiredimpact engine再生成。Risk if removed: 正当なpublic compatibilityまで捨てる。
- Proposed rewrite contract: public corpusはKEEP、retired engineは除外。Acceptance evidence to retain: 0030 explicit full retirement。Human decision required: No。

## A51 — Computed View DSL v1

Classification: **DEMOTE TO HISTORY**

- Artifact / Current owner / Current status: 0027; Applied history、0031 retirement。
- Provenance: Human retirement。Protects / Why it exists: programmable intentの歴史。
- Reason: 将来可能性とcurrent acceptanceを分離。Risk if retained: 未依頼DSL / compiler再生成。Risk if removed: 将来user intent経緯喪失。
- Proposed rewrite contract: non-goal、具体的new need時のみ再検討。Acceptance evidence to retain: 0031 no v1 fallback。Human decision required: No。

## A52 — Git collaboration / automation expansion

Classification: **DEMOTE TO HISTORY**

- Artifact / Current owner / Current status: 0029; Rejected。
- Provenance: Human simplification decision。Protects / Why it exists: Git ecosystemへの拡張候補。
- Reason: Rejectedをfuture roadmapと扱わない。Risk if retained: Git client / issue integration creep。Risk if removed: Gitとの協調価値を全否定。
- Proposed rewrite contract: YAML+Git benefits KEEP、独自client default外。Acceptance evidence to retain: Vision / 0029 rejected outcome。Human decision required: No。

## A53 — 0035/0036の中間workbench placement

Classification: **DEMOTE TO HISTORY**

- Artifact / Current owner / Current status: 0035/0036; Applied history。
- Provenance: Agent delegation下のUX refinement、later0041–0049。Protects / Why it exists: densityとLegacy参考の経緯。
- Reason: 最新One Table / compact toolbarを優先。Risk if retained: 初期pane / page hierarchy再生成。Risk if removed: recent density / source tree lessons喪失。
- Proposed rewrite contract: observablesだけA31/A25へ抽出。Acceptance evidence to retain: later screenshots / Desktop geometry。Human decision required: No。

## A54 — Historical performance numbers / workflow stages

Classification: **DEMOTE TO HISTORY**

- Artifact / Current owner / Current status: evidence documents / old execution checkpoint; Historical。
- Provenance: 複数hardware/toolchain、historical workflow。Protects / Why it exists: 測定過程と失敗/修正の記録。
- Reason: 平均・異環境・過去Stageをcurrent product contractにしない。Risk if retained: 速かったsampleだけ採用・workflowをruntimeへ写す。Risk if removed: baselineの再現条件喪失。
- Proposed rewrite contract: [performance protocol](performance.md) / repository governanceを分離。Acceptance evidence to retain: exact Candidate reconciliation、hardware metadata。Human decision required: No。

## A55 — Exact App.tsx regex literalのrewrite constraint

Classification: **DELETE**

- Artifact / Current owner / Current status: app.test.mjs内literal条件; implementation-specific current test。
- Provenance: Agent smoke convenience。Protects / Why it exists: 旧function / invoke / state名。
- Reason: 意味はA33へportでき、literal自体に外部互換性価値なし。Risk if retained: 旧App.tsx/request topologyを再生成。Risk if removed: literal以外のsafety意味まで捨てる危険。
- Proposed rewrite contract: future corpusからliteral constraintを除外（現test削除しない）。Acceptance evidence to retain: A33のobservable / boundary evidence。Human decision required: No。

## A56 — Legacy read API mock compatibility shim

Classification: **DELETE**

- Artifact / Current owner / Current status: workspace-fixtures.ts旧openDataFile/openTableContext adapter; test helper。
- Provenance: test移行のconvenience。Protects / Why it exists: 旧mockと新navigation payloadの接続。
- Reason: GUI internal adapterはpublic stable protocolでない。Risk if retained: 廃止read pathを新実装にも併設。Risk if removed: fixtureのsemantic expected値まで失う危険。
- Proposed rewrite contract: input/oracleのみ移植、shim API除外。Acceptance evidence to retain: creation / table / authoring observable cases。Human decision required: No。

## A57 — Implementation mapをrewrite設計図として投入

Classification: **DELETE**

- Artifact / Current owner / Current status: docs/technical source/table implementation notes; implementation history。
- Provenance: 各sliceのmodule/type/command説明。Protects / Why it exists: 当時の実装場所・技術検証。
- Reason: 構成を再現させるinputとして不要。事実調査用には保存。Risk if retained: 旧module decomposition / adapter再生成。Risk if removed: .NET技術constraintsやbug理由まで捨てる危険。
- Proposed rewrite contract: rewrite inputからtopology mapを除外（document削除しない）。Acceptance evidence to retain: contract/corpusへ抽出したconsumer / safety evidence。Human decision required: No。

## A58 — Persistent independent Diff view

Classification: **HUMAN DECISION NEEDED**

- Artifact / Current owner / Current status: GUI-SHELL-LAYOUT / GUI-DATA-DIFF; Approved。
- Provenance: P1 package + latest quiet/Git direction。Protects / Why it exists: unsaved candidate理解とview入口。
- Reason: capabilityとpersistent surfaceを区別するproduct choice。Risk if retained: 不要な常設tab。Risk if removed: 比較発見性・Save理解低下。
- Proposed rewrite contract: [D1](human-decisions.md#d1--独立diff-view)。Acceptance evidence to retain: unsaved/Conflict/Migration compare evidence。Human decision required: Yes: D1。

## A59 — Typed Filter scope

Classification: **HUMAN DECISION NEEDED**

- Artifact / Current owner / Current status: authoring-query / GUI query; Approved。
- Provenance: 0016/0017 package Human承認、個別daily価値provenance限定。Protects / Why it exists: read-only subsetによるtargeting。
- Reason: Searchと別product価値で判断。Risk if retained: operator UI / subset mode creep。Risk if removed: 長いTableのtargeted authoring低下。
- Proposed rewrite contract: [D2](human-decisions.md#d2--typed-filter)。Acceptance evidence to retain: typed predicates/no mutation/query state。Human decision required: Yes: D2。

## A60 — View Sort scope

Classification: **HUMAN DECISION NEEDED**

- Artifact / Current owner / Current status: authoring-query / GUI query; Approved。
- Provenance: package承認、typed ordering Agent refinement。Protects / Why it exists: 値の探索とstable read-only順序。
- Reason: Filterやsource reorderと独立に選ぶ。Risk if retained: sort mode / source-order friction。Risk if removed: 比較・探索効率低下。
- Proposed rewrite contract: [D3](human-decisions.md#d3--view-sort)。Acceptance evidence to retain: invalid/null/stable ties/no bytes mutation。Human decision required: Yes: D3。

## A61 — Advanced Batch / preview surface

Classification: **HUMAN DECISION NEEDED**

- Artifact / Current owner / Current status: authoring-batch / grid authoring; Approved。
- Provenance: P1 package、0040 direct paste Human direction。Protects / Why it exists: 反復入力を減らす高度range操作。
- Reason: direct clipboard基盤と高度UIを分離。Risk if retained: preview/Fill control creep。Risk if removed: 反復修正が高コスト。
- Proposed rewrite contract: [D4](human-decisions.md#d4--advanced-batch)。Acceptance evidence to retain: all-or-none/typed codec/one Undo/10k paste KEEP。Human decision required: Yes: D4。

## A62 — Saved Overview GUI capability

Classification: **HUMAN DECISION NEEDED**

- Artifact / Current owner / Current status: Table Overview / build selection; Approved。
- Provenance: P2 package Human approval、current One Table direction。Protects / Why it exists: saved cross-source / profile理解。
- Reason: Build semanticsと独立GUI surfaceを分離。Risk if retained: dual saved/dirty UI complexity。Risk if removed: profile inclusion / cross-source理解低下。
- Proposed rewrite contract: [D5](human-decisions.md#d5--saved-table-overview)。Acceptance evidence to retain: saved snapshot/no dirty mutation/profile oracle。Human decision required: Yes: D5。

## A63 — Flow mapping spec/runtime discrepancy

Classification: **HUMAN DECISION NEEDED**

- Artifact / Current owner / Current status: YAML-SUBSET-007 Approved vs source_edit inline current test。
- Provenance: fee882c / bc0a28c implementation。明示Human syntax permissionはUnknown。Protects / Why it exists: 現在受理・source preservationとsubset禁止が衝突。
- Reason: test passをformat approvalにしない。Risk if retained: 未承認syntax拡張を継承。Risk if removed: 既存受理sourceをrejectする互換性risk。
- Proposed rewrite contract: [D6](human-decisions.md#d6--yaml-flow-mapping)。Acceptance evidence to retain: audit focused test pass、before/after flow bytes。Human decision required: Yes: D6。

## 件数

| Classification | 重要項目群 |
| --- | ---: |
| KEEP | 27 |
| REFINE | 18 |
| DEMOTE TO HISTORY | 9 |
| DELETE | 3 |
| HUMAN DECISION NEEDED | 6 |

これは全Requirementや全test caseの個別審査済み件数ではない。subscopeを区別して分類しており、同じfileにKEEP意味とDELETE候補のmechanismが共存する。全体coverageとunknown provenanceは [audit report](audit-report.md) が所有する。
