# Clean-room Rewrite — Candidate review evidence

Status: Evidence / Human-gated

Review Candidate: `f7cf90d857124602a918bdcad1dda2ef2e031850`（2026-10-07、実IME correction）

基準Candidate: `f57102e2d098a4a3385081dc111982e5c065d4ff`。既存のperformance raw samplesはこの基準binaryの実測として保持し、新SHAへ読み替えない。

**Not cutover-ready — macOS dense input / held drag、Windows actual Desktop、Unity actual環境のrequired evidence gaps。** `f57102e`の実IME違反は`f7cf90d`で修正し、scalar / nestedのactual IMEとrequired CIで再検証した。2026-10-09にHumanがPR #16をmainへmergeしたが、未取得の外部証拠は免除していない。以下の旧Candidate evidenceは当時の記録として保持する。

## GUI磨き込み追加scope（2026-10-10）

Candidate: `03d77b60263a0418e62d78ccc684dba0b425d0b7`、Work base: `edc633a03b3406a7ba5741ee8f63b3b540a1a1c4`。Humanが比較を依頼したfront-end-sample / codex-schedulerから、paneの連続性とAnt標準feedbackを既存`GUI-SHELL-PRESENTATION-001 / 002`へ反映した。新しい機能・常設surface・acceptance dimensionは追加していない。

| Finite追加evidence | 結果 |
| --- | --- |
| pane / standard feedback | content幅を保つ開閉、即時inert / focus退避、rapid reversalのobsolete focus拒否、標準Tooltip、100 / 160 / 200ms token、Explorer / Welcome / Problemsの階層、pending / disabled feedback。Project adapterの既存開閉checkで途中frame / selection保持も確認 |
| focused回帰 | local native Project14 / geometry9 / authoring13 / focus6 / creation6 / delivery7の計55 checks PASS。表示境界3 tests / TypeScript / Vite / Rust fmt / clippy / integrity PASS。local rawは実行時HEAD / dirty / binary hashを保持 |
| actual macOS | 専用bundle28でWelcome / Recent Open、Dark Table、⌘B、menu / Tooltip、Problems、Light / Settings、invalid scalar→Problems focus→Escape / Undo、Build/Publish Drawerを確認。全9 filesのSHA-256一致。今回は既存surfaceへの変更に絞ったreview |
| correction / actual再確認 | Darkの選択済みfilenameが背景に近いaccent色となるregressionを発見し、foreground tokenへ修正。clean Candidateのbundle29でDark / Light、System復帰、設定focus復帰を再確認。全9 filesのSHA-256一致、33 rows / 264 cells / active editor0 / dirty0 |
| Reduced Motion | OS設定は変更していない。今回の実機prefはOFF。reduced tokenの0sとCSSのduration / delay除去をfresh reviewし、旧Candidateのactual ON / OFF結果は当時の証拠として保持する。ONの新実機測定へ読み替えない |
| Candidate CI | [implementation run 38031936820](https://github.com/kamahir0/masterdata/actions/runs/38031936820): SUCCESS、全17 jobs。両Tier1でfocused55 checksずつ、全15 native Desktop categoryとcontrolled navigation / 100k capacityを通過。[integrity run 38031937025](https://github.com/kamahir0/masterdata/actions/runs/38031937025): SUCCESS |
| local app / cleanup | `node scripts/local-app/app.mjs --no-launch`のproduction build / staged smoke / install PASS。`~/Applications/masterdata-local.app`をCandidateへ更新。専用bundle28 / 29は通常Quit、対象process残存0。Unity Editorは今回起動していない |

bundle28は`ba2f545`のproduct / Development Stateだけdirty、binary SHA-256 `36a6160a70576adcf31ce262c36a04d08c1e98aace7e30fa96a6420eb01fd0c6`。bundle29はclean correction Candidate、binary SHA-256 `c85b4c04ca3d62b00ed89fd6fd0c3e20cf51ccca9d58b4415961223ad864a638`。production installed binaryは`4636e59221274b3a0df3b68405b9a67877970d43fa2426cab7bef2b1a146ab65`でpackageと同一。CUA native pointer / keyboardでの実画面reviewであり、physical human input / CGEvent helper / dense OS performanceの証拠へ改称しない。

local controlled navigationは`ba2f545`（Development Stateだけdirty）で実測。3 Tables / 8 sources / 12,000 records / selected 2,000×20、全warm discovery / enumeration / YAML parse / validation = 0、mounted rows最大33。修正Candidateへのlocal測定の読み替えは行わない。

| case | n | accepted median / p95 / max ms | backend p95 ms |
| --- | ---: | ---: | ---: |
| first source | 1 | 97（single） | 3.76（single） |
| revisit | 300 | 61 / 67 / 73 | 3.38 |
| same Table | 300 | 64 / 71 / 181 | 3.39 |
| cross Table | 300 | 61 / 67 / 132 | 3.26 |
| schema | 300 | 64 / 72 / 96 | 3.36 |
| dirty revisit | 100 | 45 / 67 / 78 | 4.53 |
| rapid | 100 | 47 / 53 / 56 | 3.33 |

cold Open→inventory publicationは793ms、boot readyは99ms（各n1）。non-obsolete invalid0、obsolete300は意図どおり拒否。境界はcontrolled input / selection publication / backend / freshness / projection / serialization / IPC / React commit / layout / target-filtered rAF paint opportunity / controlled first acceptedを分離する。rAFをGPU presentationと呼ばず、controlled <150ms target通過を未取得のdense OS target証明の代わりにしない。passing metricの追加optimizationは行っていない。

修正CandidateのCI controlled accepted p95は、macOSでrevisit111 / same110 / cross104 / schema107 / dirty113 / rapid72ms、Windowsで95.8 / 95.1 / 91.1 / 97.0 / 94.9 / 61.8ms。全warm project-wide countsは0。100k Desktop warm p95はmacOS122 / Windows90.2ms（各n100）、bounded rendering / 10k paste / source exact checks PASS。これはnative WebView / controlled handlersの証拠で、既存Windows actual OS gateを解消するものではない。全samples / RSS / stage latency / environmentはmachine evidenceへ保持する。

Scope: base→Candidateのpresentation diffを同じagentの別passでfresh review。Specification Conformance: Pass。Tests and Regression Evidence: 上記のfocused / exact source / latency / CI。Rationale Freshness: Still accurate（固定幅contentのclip、closing時の即時inert、context / input intentを持つfocus取消、幅変更時はvisible列boundaryが変わる時だけReactへ公開）。Evidence Integrity: current canonicalとADR 0009へ照合、fixtures / engine / native builderは不変。Architecture / Scope: Rust authority、long-lived Workspace、physical draft / history、fresh write authorization、bounded projectionを保持。Findings: contrast regression修正・再確認済み、今回scopeのBlockingなし。Verdict: GUI磨き込みscope complete、required CI reconciliation完了。Clean-room全体の既存28項目ledger / 4 external gatesは不変。[machine evidence](clean-room-candidate-data.json.gz)の`guiPolish`へ今回scopeを分離する。

## Desktop操作feedback追加scope（2026-10-10）

Candidate: `dc7a0eb3e29930684e9ec608cd457208fec6a3cf`、Work base: `8191c05c29e6adf2074d64da9bc683a498d6e2f3`（HumanによるPR #16 merge）。authorityは[0056](../spec-changes/0056-desktop-authoring-feedback.md)とそのcanonical owners。今回の9点以外へcompletion boundaryを拡張しない。

| Finite追加evidence | 状態 / exact evidence |
| --- | --- |
| Welcome / Explorer / appearance | Recent 0件の空状態、再起動後の履歴と明示Open、全paneの開閉 / ⌘B / 選択保持、単一root省略、WelcomeからLight / Dark / Systemへ到達。macOS専用bundle25 / 26で確認 |
| inline / menu / viewport Add | 通常表示と入力の同じ書体。row menuは外側click / 再click / Escapeで閉鎖。横長Table最終列へのArrow移動でも列Addはviewport右端、行Addは左下。actual Tableで確認 |
| drag / source preservation | actual row上下 / column左右dropと各Undoでcleanへ復帰。local native authoring 13 checksで両方向の途中frame補間を確認。bundle25 / 26の全10 Project filesのSHA-256はbefore / after一致 |
| focused回帰 / warm navigation | local native geometry 9、authoring 13、Project 14、focus 6、creation 6の計48 checks。12k corpusの全warm project-wide discovery / enumeration / YAML parse / validation = 0。controlled accepted p95はrevisit 64 / same 68 / cross 64 / schema 68 / dirty 64 / rapid 50ms。actual OS timingの代替ではない |
| required Candidate CI | [implementation run 38016344291](https://github.com/kamahir0/masterdata/actions/runs/38016344291) / [integrity run 38016344363](https://github.com/kamahir0/masterdata/actions/runs/38016344363): SUCCESS、全17 jobs。両Tier1 native Desktopでfocused geometry / authoring / Project / focus / creation / delivery計55 checksずつがPASS |

actual binary SHA-256は`d303d5122dd0f9e457a9d51da42543f3d934442eac7ed5a1d3b3eebac692c2d0`、presentation Candidate `ffeeff67`。bundle25はclean、26はDevelopment State metadataだけdirty。native pickerのCUA入力は完了できなかったため、このrunのOpen成功はRecent buttonによるものだけとする。CUA横scrollも変化を観測できなかったため、実機の右端確認にはArrow navigationを使った。OS設定やCGEvent helperは操作していない。

製品修正Candidate `0114d09`のbundle27（binary SHA-256 `1f64d54c470e41142e8a370f7e146dc81d9e175338af8cc276b48602c3973722`、evidence / state文書だけdirty）でもRecent Open、root省略、System復帰 / Dark、設定focus復帰、inline / Escape、row menu閉鎖、⌘B、最終列での固定Addを再確認。全10 file hashes一致。production build / staged smokeを通した`~/Applications/masterdata-local.app`を`0114d09`へ更新した。後続Candidateは検証codeのみの変更。専用bundle25 / 26 / 27の対象process終了も確認済み。Unity Editorは今回起動していない。

CIのmacOS / Windows geometryは、overlay入力が先にmountし仮想表示cellの描画前に書体比較して失敗した。`fa280b0`でobservable待機へ修正しlocal geometry 9 checksを再実行した。別pass reviewで新root表示の`strip_prefix().unwrap()`がshared engineの受理する空の外部rootでpanicすることも確認し、`0114d09`でconfigured identityを保持するfallbackへ修正した。Desktop Open integration 2 tests / clippy / integrityがPASS。source scopeやengineの受理規則は変えていない。設定確認を追加したProject adapterでも、programmatic clickが作成ボタンへfocusを移さず、設定ボタンへの正しい復帰を作成の復帰先と誤認していた。`b070388`で設定のfocus復帰完了とCreateのkeyboard originを明示し、local Project 14 checksを再実行した。製品codeは`0114d09`から不変。WindowsのDelivery adapterでは検証用ack fileのtruncate / write間をnative readerが読み、EOFで中断した（Build / actual reloadは成功済み）。`dc7a0eb`で完成JSONをsame-directory replaceで公開し、local Delivery 7 checksを再実行した。

Scope review: Specification Conformance: Pass。Rationale Freshness: Still accurate（bounded virtual cellの描画待機、rc-dropdownの遅延focus取消、visible neighbourのCSS補間保持）。Architecture / Scope: long-lived Workspace / physical draft / fresh write authorityは保持し、Rustが解決したroot表示pathのみinventoryへ追加。Evidence Integrity: local raw HEAD / dirty / hash、初回failureを保持し、actual observationsをdense OS performanceやheld-drag Escapeへ流用しない。同じagentの別passで、独立reviewerを主張しない。Findings: known defects corrected、今回scopeのBlockingなし。Verdict: feedback追加scope complete、required CI reconciliation完了。Clean-room全体はNot cutover-ready。[machine evidence](clean-room-candidate-data.json.gz)の`desktopFeedback`にraw metadata / checks / hashesを分離する。既存28項目ledgerと4 external gatesは不変。

## 日本語Desktop追加scope（2026-10-09）

日本語Candidate: `20b3a3a304adbe3a61bd535ac9400de792a4a21a`、Work base: `b919fa5e4ec2120b523cabca5d8a0a31cf8cd8da`。Human-selected `GUI-SHELL-LANGUAGE-001`により、操作・説明・状態・確認・診断・accessible labelsを日本語化した。Ant Design日本語localeとnative menuを使い、technical identity / wire / CLI / YAML / TOMLは保持する。外部toolのtechnical detailとOSが追加するnative機能はOS側の言語を保つ。独立oracleやengineの意味論は変更していない。

| Finite追加evidence | 状態 / exact evidence |
| --- | --- |
| 日本語surface / technical identity | 既存全Desktop surfaceを静的review。表示境界3 tests PASS。補間template37件でtechnical capture保持を確認 |
| authoring / focus / write回帰 | local native 13 contextual categories / 106 checksとnavigation PASS。各rawは実行時HEAD / dirty / binary hashを保持し、最終Candidate測定へ読み替えない |
| macOS actual日本語UI | clean Candidateの専用bundle24、binary SHA-256 `d9f088e468018b0fa1a41181f6fc1bd2b8dce047b59cc92dd7977f2cf47660c9`。CUA native pointer / keyboardでTable・要約・Problems・Build/Publish・native Editを確認。source hashes全て不変。bundle23（05da5d6）では設定 / Complex / Light / Darkと全source hash不変も確認 |
| source safety / scope | engine / fixture / source mutation / native builder変更なし。scalarの`[3 items]` / `{2 fields}` / `(missing)`は表示値として保持。actualで発見したgrid要約の英語残存だけを修正 |
| Candidate required CI | [implementation run 37812769520](https://github.com/kamahir0/masterdata/actions/runs/37812769520)の全17 jobs / [integrity run 37812769539](https://github.com/kamahir0/masterdata/actions/runs/37812769539): SUCCESS。shared engine / oracle、native consumer、Unity portable、capacityは3OS、native Desktop / package smokeは両Tier1で通過 |

Scope: b919fa5→日本語Candidateのpresentation / native label diffを別passでreview。Specification Conformance: Pass。Tests and Regression Evidence: 上記の表示・focus・source exact回帰。Rationale Freshness: Still accurate（Rustのbounded projectionではcomplex payloadを省くためvalueの有無を表示要約の区別に使い、frontendでschemaを再解釈しない。native item ID / selectorは保持）。Evidence Integrity: GUI-SHELL-LANGUAGE-001 / 0055のauthorityと既存adaptersを照合、未取得の旧OS-input証拠を新Candidateへ流用しない。Architecture: presentationのみ、read/write authority・lifecycle・projection境界の変更なし。Findings: None identified。Verdict: 日本語追加scopeはReady to merge: Yes、required CI reconciliation完了。Clean-room全体は既存4 evidence gapsによりNot cutover-ready。

下記の28項目ledgerと旧conformance / performance / OS-input結果はf7cf90dまでの記録。日本語の追加Human scopeは上記の有限evidenceに限定し、既存external gateやstretchの扱いを変更しない。[machine evidence](clean-room-candidate-data.json.gz)の`japaneseLanguage`に追加scopeのraw metadata / checks / reviewを分離する。

## Scope / completion boundary

Work base `e5c2f3df82b8e5ac033a894cd8ae400b4e312c56`からReview Candidateまでの新実装を対象とした。同じagentが実装passと分けてfresh / adversarial reviewを行った。別reviewerによる独立審査を主張しない。旧production source / history / generatorは参照していない。

2026-10-07 Human completion decisionに従い、[Current Objective](../current-objective.md)のApproved canonical / oracle / matrix / Human decisionsへ境界を固定した。有限ledgerは28項目で、required、target、stretch / non-blockingを区別した。[machine evidence](clean-room-candidate-data.json.gz)にledger、raw OS-input samples、metadata、stage distributions、byte goldensのhash、CI / capacity / consumer結果を保持する。新しいrequirementsやacceptance dimensionsは追加していない。

## Architecture / implementation

[ADR 0009](../adr/0009-clean-room-desktop-workspace.md)のownershipをcontractから確認した。YAML bytesと位置付きsyntax tree、schema-directed semantics、mutation derivationはRust engineが所有する。long-lived Workspaceはimmutable read generationとphysical-source別draft / historyを分離する。read cacheは理解を速めるもので、native write boundaryのfresh file / parent / content identity確認を代行しない。

Desktopにはbounded projection、resolved descriptor、lossless value、operation resultだけを渡す。Reactはselection / focus / scroll / active input / overlay / drag previewを所有する。Ant Designをshell / controls / overlays / theme / motionのsystemに使い、gridはsingle active editorとbounded custom React renderingで成立させる。selectionは最新pending一件へcoalesceし、diagnostics / result / focusをgenerationとProject epochで拒否する。validationとdeliveryは別workerで、queue lock中にparse / I/Oを行わない。CLIは同じRust engineを独立した短いlifetimeで使う。

実装済みscopeはdaily loop、schema / scalar / Complex / nullable / Enum / Flags / Custom / Value Object、Add / Delete / Undo / Redo、row / column / Array reorder、copy / typed paste、source-local Find / Search、Problems、source creation / move、外部変更 / Conflict / Save / Save All / unknown outcome、schema / type Migration / Recovery、Project / Settings / tags / Key / Reference authoring、Build / receipt / Publish、required CLI、Unity package。standalone Diff / Typed Filter / View Sort / Advanced Batch / Overview、general Git clientは追加していない。

## Oracle / source preservation

| Category | Review Candidate evidence |
| --- | --- |
| byte scenarios / flow mapping | 固定17 casesのexact UTF-8 bytes一致。D6のflow read / unrelated / local / unsafe / new-blockを含む |
| interpretation | 固定9 cases、schema変更時record bytes保持、64-bit limits、null / quoted-null |
| Save / topology / empty | 固定7 Save cases、inline / separate / mixed / no-source、current Table physical scope |
| paste | 固定4 cases、lossless / invalid-safe / ragged / PK unsafe、all-or-none、一logical Undo |
| faults | precommit failure / postcommit unknown、no automatic retry、draft / history / disk outcome保持 |
| migration | 固定Rename / Add / Dropのmulti-source exact bytes、stale closure、authorization、rollback / Recovery |
| navigation / workflows | dirty/history/query/selection保持、obsolete publication拒否、bounded queue、nested Problems、source lifetime |
| consumer | generated C# compile、actual MasterMemory reload、独立query全項目 |
| capacity | 固定100k / 10 files / 20 columns / 10k paste、2M valuesをbefore / after双方で確認 |

[byte adapter](../../engine/tests/byte_oracle.rs)はcandidateとfrozen expectedを`as_bytes()`で比較し、拒否時はinput bytesの保持をassertする。comments / quotes / lexical representation / CRLF / blank lines / block scalar / order / unrelated bytesを含む。whole-document YAML serializerはmutation authorityに使っていない。golden毎のlength / SHA-256 / CRLF countをmachine evidenceに収録した。

[Source Save](../../engine/tests/workspace_oracle.rs)、[fault](../../engine/tests/fault_oracle.rs)、[Migration](../../engine/tests/migration_commit.rs)もexact disk outcomesを確認する。Desktop raw CompareのCRLFはnative payloadとHTML TextArea表示を区別している。corpus integrityは621 frozen assets / 186 retired assets、freeze `c13d5a7da32acbd2f2c1927dc6d572648512fb6e`のまま。

## Consumer / Build / Publish

[independent Consumer.cs](../../fixtures/rewrite-oracle/v1/consumer/Consumer.cs)をgenerated C#と実binaryに接続した。[native consumer CI](https://github.com/kamahir0/masterdata/actions/runs/37540442659)はmacOS / Windows / Linuxそれぞれで8 delivery testsを実行し、PK / SK / nonunique / composite / reference helper / arrays / 64-bit / nested Value Objectをactual reload後にassertした。`Reward.ItemId.Value == 2001`はfrozen consumerのassertを通している。builderはMasterMemory 3.0.4 / MessagePack 3.1.11、独立consumerはpinned MessagePack 3.1.3で検証した。

.NETはRustのvalidated typed requestとC#を受け、YAMLを再parseしない。determinism、empty Table API、profile capture、failed Buildによるprevious artifact set保持、receipt-only Publish、unmanaged / `.meta`保持、all-target preflight / partial / unknown / recoveryを既存testsで確認した。Unity packageのportable / .NET Standard 2.1 / C# 8検証は成功したが、実Unity成功とは扱わない。

## Actual Desktop / presentation

macOS arm64ではComputer Useのnative keyboard / pointerでWelcome / Open、通常 / dirty Table、Problems、Complex、menu、Conflict / Compare、Migration review、Build / Publish、Dark / Light / System、long / wide / sticky、drag drop / Undo、focus / Arrow操作を確認した。System preferenceはnative storageへ保存され、再起動後のradio selectionとRecent Projects復帰を確認した。各検証用projectのsource hashesは一致し、Humanが編集中の別windowは変更していない。

既存presentation checklistに対し、Ant Design system / tokens、compact density、overlay / feedbackの一貫性、custom gridとの統合、stable geometryをreviewし、identified visual Blockingは未検出。追加のaesthetic polishはnon-blockingで、ここから新しい最適化を始めない。actual IMEとOS reduced-motion ON / OFF復元は下記follow-upで確認した。held-drag中のEscape観測とWindows OS interactionはgateに残す。native WebViewのcontrolled composition / drag preview / cancelは成功している。

## Performance

reference実測はApple M1 / 8 logical cores / 16GiB / macOS arm64、release binary SHA-256 `ae171524b4b1b20eb35f72b4aed59848df2befbd92370e59d6a8b6ad7cb9abbe`。2runは同じReview Candidate、working tree clean、3 Tables / 8 sources / 12,000 records、selected 2,000 × 20。warm時はsource bytes不変、mounted rows 33 / cells 264 / active editors 0。heavy build / testとの並走はしていない。

各cellは**median / p95 / max（ms）**。全warmでproject-wide discovery / enumeration / YAML parse / validation = **0 / 0 / 0 / 0**、invalid = 0、event clock欠落 = 0。

| Case | accepted n / runs | backend | OS event→selection publication | OS event→first accepted observed upper bound |
| --- | --- | --- | --- | --- |
| revisit A1 | 100 / 2 | 3.34 / 4.48 / 6.23 | 19.5 / 23 / 25 | 98 / 199 / 320 |
| same Table A2 | 100 / 2 | 3.33 / 4.63 / 15.09 | 12 / 22 / 24 | 97 / 183 / 301 |
| cross Table B schema | 100 / 2 | 3.16 / 3.72 / 7.57 | 19 / 23 / 32 | 97 / 166 / 318 |
| schema C schema→C1 | 100 / 2 | 3.35 / 3.79 / 7.02 | 19.5 / 23 / 26 | 93.5 / 216 / 369 |
| dirty revisit A1 | 100 / 2 | 3.25 / 3.60 / 4.88 | 12 / 22 / 23 | 93.5 / 198 / 307 |
| rapid final C2 | 100 / 2 | 3.34 / 3.97 / 8.01 | 18 / 23 / 25 | 91 / 217 / 287 |

rapidは4-target sequenceを100回、total 400 selection samples。backend / publicationは400件、first acceptedはfinal C2の100件。入力ツールの間隔ではbackendが各selectionの間に完了したため、これはcontrolled obsolete-responseの代替証拠ではない。意図的に遅延した50-selection publicationはnative Desktop testsが別途確認する。

cold Open→inventory publicationは737 / 725ms（2runのindividual values）、native backendは693.79 / 689.37ms。first sourceのOS keyboard試行は139ms（n=1）。前runのpointer試行はtoolの遅いprobeを含み893msであり、参考captureへ分離した。single sampleをp95と呼ばない。

serialization p95は0.12–0.17ms、queue p95は0.01–0.02ms、React commit p95は8–11ms、layout p95は14–16ms。paint opportunity p95は79–83ms。input / publication / backend / freshness / parse / projection / serialization / IPC / state / React / layout / paint / acceptedのraw boundariesをmachine evidenceに収録した。rAFをGPU presentationやusable completionとは呼ばない。

**backend <50ms targetはPASS。actual first accepted <150msは未証明。** probe gap p95が118–159msあり、上限値は150msを超える。timely subsetのp95 93–110ms（n=49–64）を全分布の合格へ置換しない。真のlatency失敗と断定する証拠もないため、より密なOS入力を取得できる環境へgateする。<100ms stretchをBlockingへ昇格しない。参考試行の誤ったcase arrangementはraw hashを残してwarm verdictから除外した。

Rust critical pathはreference scalar edit / Undo各100件で35.64 / 36.16 / 42.93msと0.12 / 0.14 / 0.15ms。full fixture（2 records）のComplex Array reorder / nested value / Flags / nullableを各100回+Undoで測り、operation p95は1.44 / 0.67 / 0.68 / 0.73ms、Undo p95 ≤0.12ms。source candidate / disk bytesはUndo後に完全一致。core measurementをactual Desktop latencyの代替にしていない。

Windows native WebView CIのcontrolled accepted p95はrevisit 79.0 / same 85.5 / cross 78.2 / schema 85.2 / dirty 94.0 / rapid 50.3ms。これはOS keyboard / IMEによる分布ではなく、Windows actual-input gateを閉じない。

## Capacity

100,000 records / 10 data files / 20 columns / 10,000 paste cells。全2,000,000 valuesのbefore / after、全paste targetsと非target、exact candidate bytes、Search、diagnostics 0、disk unchangedを3OSで確認した。memoryのabsolute capは現contractにない。

| Platform | native cold / paste / Find (ms) | native peak RSS (bytes) | Desktop peak RSS (bytes) | mounted rows / cells / editor | Desktop warm n / median / p95 / max (ms) |
| --- | --- | --- | --- | --- | --- |
| macOS CI | 10342 / 618 / 46 | 3015327744 | 3196420096 | 33 / 297 / 1 | 100 / 99 / 135 / 144 |
| Windows CI | 13383 / 571 / 44 | 3649945600 | 3676594176 | 30 / 240 / 1 | 100 / 64.35 / 82.2 / 94.6 |
| Linux CI | 9691 / 339 / 32 | 3743338496 | 未測定 | native projectionのみ | native stagesを収録 |

RSSはisolated runnerのOS child-lifetime maximumまたはWindows PeakWorkingSetSizeで、native+WebView subprocessの合算ではない。hardware間の因果的改善率は主張しない。capacityのpassing metricを追加最適化しない。

## Fresh / adversarial review

Specification Conformance: source / semantics / workspace / write / structural / delivery / presentationのreview範囲で既知のcontract違反を未検出。未取得のfinal evidenceは下記Blockingとして保持した。

Tests and Regression Evidence: [implementation CI](https://github.com/kamahir0/masterdata/actions/runs/37595928336)全14 jobs、[integrity CI](https://github.com/kamahir0/masterdata/actions/runs/37595928491)が修正Review CandidateでSUCCESS。shared workspace testsはmacOS / Linux 168、Windows 164（platform-specific範囲が異なる）、native deliveryは各OS 8。両Tier1 native Desktopは14 contextual categories / 各111 checksとnavigationを通過し、compositionend後のIME process Enter保持も両OSの既存authoring adapterで確認した。Rust fmt / clippy、TypeScript / Vite、corpus / docs integrityを含む。

Rationale Freshness: `Still accurate`。equal-byte read syntax reuseはfresh actual capture後だけで、write前flightを省略しない。Windows handle保持を避ける理由と128-bit file identity、source-set journalのowned-object rollback、Custom formatterのpublic declaration order、bounded projection / separate workers、Publish completion後のReact focus復帰はcurrent codeと既存regressionに一致する。

Evidence Integrity: Requirement referencesは[canonical owners](../specs/README.md) / [GUI](../gui/README.md) / [matrix](../rewrite-preparation/acceptance-matrix.md)へ戻した。ADR 0009はAcceptedでHuman React decisionを保持する。goldensは変更していない。benchmarkはnative / controlled WebView / OS-trusted inputを区別し、Candidate attestationはraw metadataの`finalCandidate: false`のまま。今回のreview SHAと同一codeであることはhead / product tree / binary hashで示すが、全final conformance成功へ改称しない。

Architecture / Scope / Performance / Cross-platform: CLI one-shot lifecycleのDesktop流入、frontend YAML/type/write authority、whole-document serializer、read-cache write authorization、persistent feature bloat、hidden project-wide navigation workはreview範囲で未検出。contractとADRだけに基づくownership reviewも行った。新しいquality barは作らず、任意refactor / aesthetic / stretchをBlockingにしなかった。

## macOS actual-input follow-up

2026-10-07 Humanは固定Review CandidateのmacOS actual-input、Reduced Motion一時変更と復元、既存contract違反だけの修正をauthorizationした。元のrelease binary `ae1715…`を独立した検証bundleへcopyし、実key入力で`にほんご`→`日本語`を変換した。変換確定の最初のEnterでscalar editorが閉じ、次のrowへ移動してdirty / Undoが発生した。Complex Nullable noteでも同じEnterがnon-null operationを誤確定した。`GUI-GRID-006`のconcrete Blockingである。両試行のraw captureを保持し、Undo後の全source hashesは不変。

WebKitではcompositionendがIME確定Enterより先になる場合がある。[WebKit issue](https://bugs.webkit.org/show_bug.cgi?id=165004)と実機failureに基づき、cell / nested inputの既存composition guardへprocess key `229`を加えた。既存authoring adapter内でこのevent orderingを再現し、native WebView 11 checksとsource / schema exact復帰がPASS。feature / polish / completion boundaryは変更していない。

修正binary SHA-256 `ffb2e20f4639d32b28b3bbfaded233c95f05ea1442533e0223f5d8e46ab6560d`でactual Japanese IMEを再実行した。scalarは最初のEnter後も`name row 1`のinput / focus / cleanを維持し、次の通常Enterでのみ`日本語`が確定してrow 2へ移動した。nested Nullable noteも最初のEnterでinput / 未確定seed / cleanを保ち、通常Enterでだけoperationを確定した。両操作は一Undoで復帰し、全source hashesが一致した。修正diffのfresh reviewでRust semantics / workspace / write / navigation boundaryの変更はなく、他の既知製品Blockingは未検出。

OS Reduced Motionは元のOFFを記録してONへ切り替え、アプリの`reducedMotion=true`、Complex / menu開閉、row reorder / Undo、clean復帰を実機確認した。その後OSのOFFとアプリの`false`への復元を確認した。

documented Computer Use APIで`drag`とEscapeを並行送信してもdropが先に完了し、held cancelを観測できなかった。referenceのbatch key入力10 cycles / 各4caseでも、遅い受付値にはpaint後120–190ms程度の無入力区間がある。既存100 samples / 2runのtarget判定を変更せず、これらを合格証拠に数えない。

Humanは修正Candidate `f7cf90d`上でtemporary CGEvent helperによるdense input / held-drag Escapeを明示authorizationした。権限設定を変更しない条件に従い、compile済み`/tmp/masterdata-os-input`を専用bundle `dev.masterdata.clean-input-review.20261007.19`のzero-step planで実行した。`CGPreflightPostEventAccess()`がfalseを返し、event生成・送信前に`permissionUnavailable`（exit 133）で停止した。送信CGEvent / accepted sampleは共に0。permission request、Security / Privacy等の設定変更、persistent hook、再試行は行っていない。product codeとCandidateは不変で、専用projectの全source hashesも一致した。

この試行は**CGEvent入力のpermission preflight失敗**であり、physical human inputやperformance measurementではない。injection timestamp / application receipt / selection / React / layout / paint / first acceptedの新しい値はない。helper起動時間や過去のprobe遅延をproduct latencyへ加減してtarget達成を主張しない。plan / exit / stderr / helper source・binary hashをmachine evidenceに保持した。直前のevidence / state commit `4d785dd`のimplementation CI / integrity CIもSUCCESSへreconcileした。Windows / Unityのgateは維持する。

## Human gates / verdict

1. **macOS actual performance**: dense OS probesによる全分布の<150ms target証明が必要。CGEvent helperはauthorization済みだが、現在の`CGPreflightPostEventAccess()`がfalseで実行不能。permission設定変更を禁じたHuman条件に従って停止した。過去のtool-limited upper boundsで合格を主張しない。
2. **Windows x64 actual Desktop**: native CIは成功したが、Computer Useから操作できるWindows環境がない。actual keyboard / IME / pointer / drag / visual / accepted-interaction distributionsが必要。
3. **Unity actual Editor / runtime**: 利用できるlicensed Editor / project / Player環境がない。package import / compile / Editor observationとruntime loadの実行証拠が必要。[Unity contract](../specs/unity-integration.md)のportable CI成功を実Unityへ置換しない。
4. **macOS held-drag Escape**: actual IMEのBlockingは修正・実機再検証済み。Reduced Motion ON / OFF復元も確認した。held drag中のEscapeはatomic APIで観測できず、authorization済みCGEvent helperも同じpermission preflightで停止した。入力は一件も送っていない。

[workflow Human gate 7](../execution-workflow.md#human-gate)とHuman completion rule 7により`decision-required`を維持する。推奨はpermission変更を要しない承認済み実行環境で同じCandidateの残証拠を取得すること。提供できない場合のevidence exceptionはHuman-selected boundary decisionが必要で、自動免除しない。追加の操作authorization自体は取得済みで、再確認は不要。decision後のscopeはこの有限ledgerの残証拠と、そこで実際に発見したcontract defectの修正だけである。

Verdict: **Ready to merge: No。Not cutover-ready。** Review用Candidateは固定したがfinal cutover Candidate / objective-completeにはしていない。main置換、merge、releaseは行っていない。
