# Rewrite Input Purification — Audit Report

この文書はPurification監査時点の分類・provenance。D1〜D6の未決定表現は0051でsuperseded。current owner / readinessは [finalization report](finalization-report.md) を参照する。

Status: Historical Evidence

## Diagnosis

source preservation、schema-directed interpretation、exact identity / Conflict / Recovery、physical source ownership、real .NET consumer、最近のDesktop UX / navigation evidenceは強い資産である。安全性は単なる旧codeの複雑さではなく、外部編集とfailureを扱うproductの条件である。

最も危険なbaggageは、canonical内に残る初期sliceの限定prose、Accepted RFCの古いroadmap、exact internal API / component / source regexを要求するtest、performance harnessのone-shot request topologyである。Approvedもtest passも次architectureの正しさを自動保証しない。

本ObjectiveのoutputはDraft監査案で、current specs/testsにpurificationを適用していない。production code / dependencies / fixture / testの変更はない。

## Fresh recovery / Git reality

開始時のtrusted `main` / upstream / remote HEADはすべて `eb2acd00af4bdd3c9751fab134ba7e46300f7e59`、working tree clean、merge/rebaseなし。remoteは `https://github.com/kamahir0/masterdata.git`。前Objectiveは `objective-complete`、Candidate `ab93c30df14fb95905ec4aaf11a59848a9be5901`、Active work / Blockingなし。conversationの値ではなくlocal Gitとremote照合を使用した。

前Work base `1194a59f9d7493b186a92cc418efbeeac449443d` → HEADの46-file navigation change、recent commits、0040〜0050と関連historical commitsを調査した。Current Objective / Development Stateだけを新監査へ更新し、current branchを維持した。

operating authorityとしてAGENTS、execution workflow、documentation policy、specification workflow、rationale guideを読んだ。`refine-spec` / `review-spec`でprovenance・material fork・Draft境界を検査し、`review-code`のfresh reviewでdelivery scope / evidenceを確認する。今回のanalysisはspec removalのapproval/applicationを含まない。

## Coverageとprovenanceの限界

| 対象 | Audit coverage / important knowledge | Route |
| --- | --- | --- |
| Product Vision / terminology / documentation ownership / workflow | daily product、Git ecosystem、extensions、semantic用語とimplementation記述の混在。repository procedureはruntime contractではない | registry A01/A28/A44/A54、constitution |
| domain canonical 29 files | 26 Approved / 1 Implemented / 1 Draft / 1 Deprecated。project layout/init/config、YAML/schema、Table/key/reference、6 type-system specs、source creation/edit/path/tag/record/field、schema/type migration、query/batch、Build/selection/preview、CLI/runtime/Unity | A01–A20、A39/A42–A45、A59–A63 |
| GUI canonical 15 files | 14 Approved / 1 Implemented。shell/project/initializer、4 data-editor specs、Explorer/Table/Type/source creation/settings/Overview/delivery/theme | A08–A10/A20–A25/A30–A35/A58–A62 |
| ADR 0001–0008 | 7 Accepted / 1 Superseded。YAML/shared Rust/.NET/location/key/source interpretation/sessionのWHY、retired host | A01/A02/A03/A14/A26/A29/A36/A49 |
| RFC 0001–0008 | 4 Accepted / 2 Proposed / 1 Draft / 1 Superseded。採用済み意味とold parser/initial edit/roadmapを区別 | A03/A39–A41/A47/A49 |
| spec-change 0001–0050 | 48 Applied / 2 Rejected。statusだけでなくsuccessor / revocationを追跡 | 下記history pass |
| tests | GUI `.test.*` 15 files、Rust integration 27 files、inline core/app/Tauri、Desktop scripts、.NET/Unity static checks。重要oracleとsource-text mechanismを調査 | test-purification / corpus |
| fixtures / golden / evidence | minimal/full/showcase/invalid、inline byte strings、actual .NET loader、fixed12k /100k /10k、Desktop geometry / rapid evidence | corpus / performance |
| current implementation | core parse / source patch / type / migration、app authoring / workspace / Save / Build / Publish / path、CLI、Tauri dispatch/thread/locks、frontend navigation/state/value/grid/user preference、codegen/.NET/Unity boundary | evidenceとして参照。module mapはrewrite constraintにしない |

coverageは全familyを横断した重要項目の監査であり、全行・全branch・全test caseの独立審査や再実行ではない。特にpublic undocumented CLI出力、全library/platform差、Unity actual runtimeは完全なconsumer corpusとして未確定。provenanceがない場合にcommit author名からHuman intentを作っていない。

## Pass 1 — Archaeology

spec-changeとGit historyの照合で、Human-selected direction、Human package approval、Agent Decision、bug evidenceを分離した。package承認は各pixel / exact APIをHumanが要求したという意味ではない。

| History | 復元した由来 / successor | 継承上の意味 |
| --- | --- | --- |
| 0001–0003 | Human Table identity / subset / serialization key。ADR0005 | filename意味論・旧FieldIDモデルを再導入しない |
| 0004–0008 | Human artifact set / legacy hard cut / external paths / receipt / multi-target | delivery safety KEEP。legacy build pathsをcompatibilityとして復活させない |
| 0009–0010 → 0021/0022 | Human初期Web方向、0021 Rejected、0022 Human retirement | Web authorityをhistoryへ |
| 0011–0012 | Human CLI/migration、project conventions、generate removal / persistence scopes | public automation KEEP、kind-first foldersをmandatoryにしない |
| 0013–0015 | Human Added-key / Recovery / Complex strategyとpreservation | type/safety KEEP、later0045前のfooter UIをhistoryへ |
| 0016–0018 | Human Desktop P1–P3 package approval | daily integrationのintent KEEP、query/batch/Overviewのscopeは今回再審査 |
| 0019–0020 | Human existing-key direct edit / source rename-move | 初期非key・非creation説明はobsolete。batch PK制約とは別scope |
| 0023 | Human Reference Option B、required/nullable、helperの選択 | consumer semantics KEEP |
| 0024–0027 → 0030/0031 | Released / compatibility-impact / computed proposalsとHuman full retirement。0025 reference-aware renameはAgent derived safety | retired engine/DSLは継承しない。rename closureの意味はKEEP |
| 0028–0029 | Human Unity Option B / metadata ecosystem。Git expansionはHuman Rejected | delivery consumer KEEP、general Git productをdefaultにしない |
| 0032–0036 | welcome / close / quit / create / workbench / source tree、主に委任下Agent refinement | dirty guard / findability KEEP、intermediate hierarchyはhistory |
| 0037–0039 | Human inline、Light/Dark/System、3 persistence scopes。Agent exact storage/provider | inline KEEP、latest OS-side user authority KEEP、localStorage例は歴史 |
| 0040–0042 | Human daily direct editing / unified Table / source interpretation、Agent keyboard/virtualizer/adapter | direct product・shared semantics KEEP、exact topology自由 |
| 0043 | Agent stable-surface refinement（Human daily authoring direction内） | routine barでgridを動かさない。AgentをHuman explicitへ昇格しない |
| 0044 | initial Option C `d09ba38` → revocation `d7fbb67` → final `4193b6e` | 最新Human fourth-optionのSave scopeだけを継承 |
| 0045–0046 | Human Complex直接操作 / spatial context、Agent text commit・cancel・source-order rules | useful acceptance knowledge KEEP、mechanism自由 |
| 0047 → 0048 | `81e1fc9` boundary line → `4a3746b` whole-target preview | line presentationはDEMOTE。duplicate boundary lessonは残す |
| 0049 | `88a3bb9`、Human screenshots / Legacy grip / compact chrome / overscroll | non-overlap・geometry関係を残しexact pixelはREFINE |
| 0050 | Human bold read architecture permission、measured duplicate parse / native blocking | recent session/read-generation lessonsをCore/CLI baggageと一緒に捨てない |
| final navigation fixes | `d811361` closed primitive Popover cost、`bfeb00c` configured physical identity、`ab93c30` creation focus replay | render cost、alias safety、one-shot focus intentをacceptanceへ |

flow mappingのpositive testは `fee882c`、scannerは `bc0a28c` を調査。syntax permissionの明示Human provenanceは見つからず、UnknownをD6として残した。

## Pass 2 — Product critic

1. ApprovedだからKEEPせず、Diff / Filter / Sort / advanced Batch / saved Overviewを別々のproduct choiceへ分解した。Search / clipboard基盤 / contextual compareまで一括削除しない。
2. GUI record-mutationにgeneral Undo/Redo導入しないprose、Added draftだけkey edit例外の説明、data-editorの初期Undo non-goal、Explorerの旧creation範囲が残る。later canonicalへのowner routingを修正する候補にした。typed-initializerの `GUI-TABLE-INT-001` はcurrent Table specにdefinitionが見つからず、stale参照候補として後続spec適用で確認する。
3. source-regex testはexact command / function / state / dependency-arrayを固定し、boundary証明として弱い。observableへREFINEしliteral条件をfuture corpusから除外する。
4. Plan / Diff / Apply capabilityはmigration safetyとして必要でも、ordinary direct editのpersistent UIを正当化しない。
5. single-row toolbar / handle non-overlap / sticky contextはHuman UX knowledge。testのcalibration pixelはarchitecture constraintへ昇格しない。

これはcurrent productを変更するfindingではなく、purification applicationの入力である。

## Pass 3 — Rewrite architect

architecture説明より先にProduct Constitutionを置き、domain/safety、performance、observable workflow、corpusへownerを分けた。current canonical全文を複製せず、詳細oracleの所在を残す。

新実装は、同一Projectのviewを高速に選ぶlifetime、baseとdirtyのcomposition、latest selection rejection、background validation、fresh authoritative writeを成立させる義務を持つ。型・module・framework・transport・request名・lock・component treeの自由は持つ。CLIとのsemantic sharingを同じlifecycleの要求にしない。

performanceを「高速」とせず、zero project-wide warm work、next-frame feedback、candidate p95 budgets、stage trace、fixed input、Tier1 / hardware variance、first usable oracleへ分解した。100k capacityと2k navigationを別contractにした。

## Pass 4 — Adversarial reviewer

draft作成後、次の反証を別passで行い、成果物を補強した。

| Challenge | 結果 / correction |
| --- | --- |
| 独立DiffをGit clientと誤認して削除していないか | 現Diffはunsaved candidate compare。D1はcapabilityを保持しpersistent入口を選ぶquestionへ限定 |
| Generated C# / Unity exact APIまで実装詳細として捨てていないか | public consumerはKEEP。private DTO / formatting / static checkerを分離 |
| old preference authorityを戻していないか | 0039のOS per-user authorityをdomainへ補記。localStorageは移行sourceのみ |
| future UIの自由を理由にdirty/Undo/Save scopeを薄めていないか | current scope、per-file base/history、eviction notice、Unknownとconfig overwrite境界を保持 |
| direct PK editとbatch禁止を誤って矛盾としたか | 異なるoperation scope。両oracleを分けた |
| 0047と0048を両方currentにしたか | insertion lineはhistory、whole previewだけcurrent候補 |
| source test存在だけでsyntaxを承認したか | flow mappingをfocused実行で確認しD6へ。production/specは変更しない |
| native26msをDesktop p95として報告したか | single sample / native / rAF / WebDriverを分離。現UI target未達を明記 |
| render最適化の方法を新MUSTにしたか | bounded render / measured latencyだけ残す。exact DOM/Popover/virtualizerは自由 |
| fixture inventoryだけでclean-room corpus完成と主張したか | 未抽出inline oracleとconsumer/p95 gapをreadinessへ残した |

Draft監査成果物のmergeを阻むfindingは、link/表現の修正後には残っていない。D1〜D6は将来contract applicationのmaterial gateであり、この監査Objectiveを途中で止めるlow-level質問ではない。

## Classification summary / obsolete inventory

[registry](classification.md) は **KEEP 27 / REFINE 18 / DEMOTE TO HISTORY 9 / DELETE candidate 3 / HUMAN DECISION NEEDED 6**、合計63重要項目群。全Requirementや全test case数ではない。

obsolete / legacyの詳細はregistry A30/A39–A41/A46–A57が所有する。特にWeb・Released Compatibility・Computed v1・Rejected Git expansion、revoked Save案、旧Complex footer、insertion line、中間workbench配置をcurrent authorityとして再投入しない。DELETE candidateはexact regex literals / legacy read mock shim / implementation topology mapのfuture input除外であり、今のassetを消していない。

## Readiness verdict

**Not ready — missing decisions/evidence.** 監査とDraft精製案は準備できたが、これだけをfinal authorityとしてcodeblind production rewriteを始める状態ではない。

- D1〜D5のcapability / persistent surface選択、D6のsource syntax矛盾をHumanが決定する必要がある。
- Human-approved purification applicationでowner conflict / stale proseを解消し、inline bytes / expected outcomesをlegacy helper非依存のcorpusへ抽出する必要がある。
- Tier1 end-to-end p95、100k output oracle / capacity budget、Unity actual consumer evidenceを補う必要がある。性能は最初のvertical sliceで検証し、機能完成後へ延期しない。

この案は **purification applicationのreview inputとして利用可能**。その適用は次のHuman-selected Objectiveで行う。今回rewrite、skeleton、production refactorを開始していない。missing evidenceを理由に既存architectureを新実装のtemplateとして固定しない。

## Verification / delivery scope

このObjectiveのfocused verificationはDraft link / classification count / scope check、`cargo xtask check-specs`、`cargo xtask check-rationale`、`cargo test -p xtask --test execution_state`、`git diff --check`。flow mapping findingについては `cargo test -p masterdata-core flow_custom_member_insertion_preserves_existing_mapping_bytes` が1 focused case pass（他caseはfiltered out）。full product / Desktop / performanceを今回再実行したという意味ではない。

local結果: spec integrity（29 domain / 15 GUI / 520 Requirement IDs / 8 ADR / 8 RFC / 50 proposals / 656 relative links）pass、rationale integrity（164 files / 106 blocks / 200 references）pass、state integrity 3 tests pass。state checkでCandidate未設定値のcaseを`none`へ修正して再実行した。新Draft10 filesのlocal link欠落0、classification63項目の件数一致、全fileのDraft境界を確認した。確定diffのfresh reviewでD6のfragment参照を修正し、Draft内のlocal heading linkも確認した。

production/test/configを変えていないためdoc/state checksと自動Fast CIをdelivery gateにする。既存Deep / Desktop successは基準Candidateの歴史的evidenceであり、このDraftの新実行結果と混同しない。fresh確定Candidate reviewとrequired CI reconciliationはGit / Development Stateが所有する。
