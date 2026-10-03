# Rewrite Readiness Gap Closure

Status: Gap evidence captured / exact Candidate verificationへroute

## Consumer first-loss boundary

旧Candidate a09e37d（main a5da23dのproduction tree）で、full fixtureと2field最小fixtureを再現した。YAML → resolved / normalized request → .NET nested conversion → Custom構築 → Append前後までは2001を保持。MessagePack wireにも2001が存在する。**最初の誤った境界はCustomのMessagePack deserialization**。binary reload後はItemId.Value=0、Amount=0、Array=defaultとなる。

MessagePack3.1.3の生成formatterは、integer keyに対応するconstructor bindingが宣言順constructorへ適合しないとparameterless structを構築し、get-only fieldsをSkipしていた。SerializationConstructor属性だけではtype mismatch診断となり解決しない。単一のsparse fieldだけでは再現せず、異なる型の2fields・keys[9,0]で再現した。[full before trace](consumer-before-full.jsonl)、[minimal before trace](consumer-before-minimal.jsonl)は一時opt-in .NET instrumentationによる実値。診断用Rust example / builder instrumentationはproductionへ残していない。

修正はC# generatorのCustom型に限定し、公開MessagePack formatter extensionを介してpersisted keyで読み、宣言順の既存public constructorへ渡す。readonly / property / constructor API、key metadata、wire slots、Rust正規化、Build / Publish lifecycleは変えない。再ロード後の実値は[after trace](consumer-after-full.jsonl)。この具体formatter構成はrewrite architecture requirementではない。

Focused actual consumer regressionでは、direct / nested / 2records / declaration-key order / Array / nullable null・non-null / uintを検証し、generated C#とbinaryのrepeat Build一致を確認した。CLI public Build → generated C# compile → actual MemoryDatabase loadの全10checkもpass。repeat generated C# / binary determinism、Build does not publish、standalone Publish eligibility / receipt rejection / caller-owned metadata / Build --publishもpass。final Candidateでのfresh再実行とrequired CI結果はDevelopment State / CIへreconcileする。


## Controlled macOS actual Desktop baseline

Apple M1 / arm64 / RAM16GiB、macOS26.6.2 (25G83 / Darwin25.6.0)、release Tauri / WKWebView、rustc1.96.0、Node25.9.0。固定3Tables / 8sources / 12,000records、2,000×20、inline / separate / mixed。fixture SHA-256は`c62d22df13d97abe1fb6c74ada13380bdbba9a2c1274af766374482c2350b953`。

専用identifierの実Tauri appをLaunchServicesからforeground起動。caffeinateでsleepを抑止し、native focused / visible / minimizedとdocument focus / visibilityをsample前後に確認。OS foreground / onscreen windowを別processから250msごとに観測した。測定期間中のlocal build / testは停止。cleanとdirtyを別processで起動し、各case100 samples×3連続batches。3batchesを3fresh Project opensとは呼ばない。inputはactual WebView内のsynthetic DOM click / keyboardでありOS pointerの物理入力ではない。

| Case | completed | first accepted interaction median ms | p95 ms (pooled / 各batch範囲) | max ms | 採用run invalid |
| --- | --- | --- | --- | --- | --- |
| clean revisit | 300 | 139 | 160 / 155–163 | 223 | 0 |
| same Table | 300 | 138 | 159 / 155–160 | 205 | 0 |
| cross Table | 300 | 139 | 156 / 155–157 | 185 | 0 |
| schema selection / redirect | 300 | 139 | 159 / 155–163 | 275 | 0 |
| dirty revisit | 300 | 139 | 155 / 154–156 | 170 | 0 |

nearest-rank p95（ceil(.95×N)-1）、medianもnearest rank。全stage・各batchのmedian / p95 / maxは[summary](desktop-summary.json)。採用runの無効sampleは0。初回dirty attemptはforeground喪失1回でfail closed、57partial samplesを全て不採用。初回clean attemptはno-op準備selectionにcommitを要求したadapter bugで停止し100partial samplesを不採用。後者はproduct latencyやforeground discardへ数えない。理由とraw partialはsummaryに保持。rAF starvationをtimerへ置換して成功扱いしていない。

Explorer selected state観測のmedian12ms / p9514–15ms。clicked targetのselection React commit median7ms / p958–9ms。schema redirectではclicked schemaとredirect後record targetを分ける。raw adapterの`selectionReactCommitMs`はschemaだけ後者を指したため、summaryのfeedbackは保存済みtraceからclicked pathの最初のcommitへ補正した。input開始は最初のgrid commit at − recorded gridReactCommitMsで復元できる。raw timestampsを変更せず、補正式をsummaryへ記録し、次回adapterではinputAtを直接保存する。

request start → IPC return p9543–46ms、target grid React commit p9584–88ms、target-filtered grid rAF opportunity p95105–108ms、accepted ArrowRight p95155–160ms（pooled）。独立exclusive stagesの和ではなく、それぞれinputからのboundaryまたはrequest span。layout cost単独 / GPU paint完了は測っていない。grid到達後にcell focusのための1frameを挟み、ArrowRightが実際に別cellへfocusを移したことを確認する。同じprotocolでrewriteと比較する。WebDriver往復や「keyを送った時刻」をusableへ代用しない。

harness injection → initial gridはclean2460 / dirty2540ms、first source → accepted interactionは180 / 178ms。各process単発でp95ではなく、OS launch / WebView生成以前を含むcold wallではない。cold native distributionは別boundaryとして下記へrouteする。

clean monitorのinterior invalid0、dirty monitor384 observationsのinterior invalid0。mounted record rows最大31、dirty300 revisitsで値999999を保持し、disk input hash不変、Undoがoriginal値へ戻ることを確認。rapidは50selections×3。全て最後のa-1がcurrent、dirty保持、最後のselection以後obsolete pathのReact commitは0、全request drain後もrollbackなし。23 / 23 / 24 requestsは今回の観測値でありcontractにしない。bounded pendingは既存controlled-deferred readのdeterministic testsも合わせて保護する。

raw [clean](clean-desktop.json.gz) / [dirty](dirty-desktop.json.gz)、[clean host](clean-host.json) / [dirty host](dirty-host.json)、OS monitor gzipを同directoryへ保持。hostのproducer HEADは567b425、workingTreeDirty=true（測定host run-loop更新、cleanではno-op準備selection除外を含む）。bundle hashを個別記録した。後続Candidateにこのharness修正を含める。production read / React / CSSはこの期間に変更しておらず、measurement-only差分とconsumer codegen修正を混同しない。

## Current vs target / Tier1 coverage

first accepted interaction p95は155–160ms pooled（個別batch最大163ms）。rewrite target <150msに5–13ms以上のgap、stretch <100msに55–63ms以上のgapがある。legacy UI最適化は行っていない。旧未制御partial sampleとの改善率を主張しない。

macOS actual Desktopを今回controlledに取得。Windowsはnativeのみでactual Desktopは今回未取得。Human Decision Bに従い、**Clean-room final conformanceではmacOS arm64 / Windows x64双方のactual Desktop**を要求する。正式ownerは[Performance Constitution](../../rewrite-preparation/performance.md)。actual Unity Editor / Playerは環境に存在せず未実施、actual .NET8 / MasterMemory managed consumerは実証した。Unity未実施を独立したBlockingへ増やさない。

既存native exact Candidate a09e37dの[Tier1 CI](https://github.com/kamahir0/masterdata/actions/runs/37076734568)はmacOS M1 Virtual / 7GiB、Windows EPYC9V74 / 約16GiBで100×3×4。Mac cold2676–3296ms / warm median28.71–41.51ms / p9536.85–62.78ms / max96.8ms、Windows cold2437–2455ms / median30.20–32.48ms / p9530.93–35.73ms / max47.62ms。双方のwarm discovery / enumeration / parse / validationは0。codegen correctionはnavigation pathを変更していない。final Candidate CIで必要なreconciliationを行い、この旧値をfresh再測定結果とは呼ばない。

100k correctness / RSS / stage evidenceは[finalization report](../../rewrite-preparation/finalization-report.md#independent-oracle-coverage)とそのexact Candidate Deep CIに保持する。consumer修正を理由にone-shot100k topologyをrewriteへ要求せず、cold / warm / capacityを混同しない。

## Fresh adversarial / code-blind review

同一agentの別passで、diffとcorpusを分けてreviewした。期待2001を変更していない。nested特殊case / consumer postprocess / reflection補正はなく、Custom全fieldのkey bindingを修正する。direct VO・配列・nullable非null・uint・2records・key-order・actual binary reloadを確認。public constructor / source bytes / fresh write preflight / Publish semanticsを変更していない。before / after binary SHA-256は共に`6e71150e3319fc472c9c1d15e13ca8dca7321a88d8fef874291ae3e9fbbb7617`で、既存wireを新readerが復元する証拠。

code-blind passではconstitution → canonical contracts → oracle → matrixを読み、旧module構成をauthorityにしない。D1〜D6とindependent source / Save / failure / consumer期待値はそのまま。今回のformatter名や測定commandはlegacy adapter evidenceでありrewrite MUSTではない。最初のsliceからwork-count / bounded state / feedback / p95 / accepted interactionを測れる。known D6 writer target gapは新Blockingへ昇格しない。

Review findings: consumer / controlled baselineに未解決のcorrectness・safety・semantic contradictionはない。既知NuGet / nullable warningsは修正対象と切り分け、dependency upgradeや無関係なcleanupを行わなかった。独立reviewerの承認を取得したという主張ではない。

## Verification / verdict ownership

focused actual consumer、codegen11 tests、portable consumer全10checks、repeat artifacts / Build / Publish、rationale / execution-state、専用Tauri release build / JS syntaxを実行。workspace check-allは全pass（GUI174 tests / 14 filesを含む）。exact Candidate required CIとfresh final Candidate consumer実行をreconcileしてからStageを完了させる。Tier1 workflowは両platformのactual managed consumer reportもartifactへ保持する。

**Consumerとcontrolled Tier1 baselineの二つのgapは閉じた。** 最終Ready / Not Readyはexact Candidateとrequired CIを含めた[Development State](../../execution-state.md)と最終報告が所有する。本evidenceだけでpending CIをpass扱いしない。Readyでもrewriteは開始せず、次のHuman-selected Objectiveを待つ。
