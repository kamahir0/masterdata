# Legacy Decommission / Clean-slate Review

Status: Self-review complete / exact Candidate CIへroute

## Git boundary / classification

Ready freeze: `c13d5a7da32acbd2f2c1927dc6d572648512fb6e`。annotated `legacy-final`をremoteへpush済み。`main`へ変更を入れず、descendant branch `rewrite/clean-room`だけで退役する。

[exact manifest](decommission-manifest.json)は削除前621 tracked assetsを分類した。KEEP Authority/Corpus315、KEEP Governance/Neutral97、REMOVE186、REVIEW/TRANSITIONAL23。これはrequirement数ではなくfile数。歴史資料のKEEPはnormative rankの昇格を意味しない。

REMOVE: Core / Application / CLI / codegen / native .NET / Desktop / frontend / Unity production、同居するtests、legacy oracle/measurement adapters、mixed xtask、旧Cargo/npm/.NET/Tauri workspace wiring、runtime CI。working treeのtarget / node_modules / bin / obj / compiled frontendも除去。別directoryへ旧sourceを移動していない。

KEEP: canonical domain / GUI baseline、Product/UX/Non-goals、Domain/Safety、Performance、matrix、全250fixture assets（exact bytes）、consumer public execution oracle、navigation / capacity definitions、readiness raw evidence、governance / policy / review skills / LICENSE。新product runtime / module / skeletonは作らない。

TRANSITIONAL: root READMEをhandoffへroute。旧内部名をcurrent authorityから外し、shared Rust semantics / native .NET delegationを保持。historical source linkはtagへのforensic linkへ変換。local install手順はFORENSIC ONLY。新checkはtree/corpus integrityだけを検査し、old mixed toolをneutral toolへ大規模refactorしていない。

## Knowledge-loss review

[test classification](test-purification.md)のlegacy literalsが守ったdirty / lossless / focus / stale responseは独立workflows / bytes / Save / faults / navigation oracleとcanonicalに移植済み。inline/separate/mixed、fresh identity、Conflict / Unknown / Recovery、64-bit、occurrence、migration authorization、Build/Publish分離を保持。per-test topologyや全historical calibrationの再現は要求しない。

[consumer corpus](../../fixtures/rewrite-oracle/v1/consumer/scenario.json)とminimal二recordにdirect / nested Value Object、persisted keyとdeclaration順の分離、expected2001、PK/SK/reference/array/64-bitを保持。C# consumerだけをoracleとして残し、generator / builder / Unity sourceは退役。Unity公開delivery意味はcanonicalに残り、実Unity未検証を.NET成功と混同しない。

capacity100k / 10files / 20columns / 10k paste、2k×20 navigation、controlled actual Desktop / native双方のevidenceを保持。D1〜D6、target<150ms / stretch<100msを変更していない。現在target未達を残し、legacy optimizationをしていない。

## Adversarial pass / code-blind simulation

同一agentの別pass。別reviewerによる独立承認ではない。clean-slateのhandoff → constitution → canonical → oracle → performanceだけから、product、daily workflow、YAML authority、shared semantics、Save scope/fresh safety、Conflict、Build/Publish、UX、consumer値、success oracleを説明できる。旧production/historyをsolution derivationに使う必要はない。

旧surface MUSTの復活、新architecture placeholder、renamed archive、private APIを要求するoracle、current-vs-target / native-vs-Desktop混同がないことを検査する。削除先へのcurrent local linkは許容しない。historical evidenceの旧名はforensic文脈だけに限定する。

## Verification / stopping point

local boundary checks / tag recoverability / branch CI / fresh Candidate reviewはDevelopment StateとGitにreconcileする。旧runtime testsは意図的にabsent。今回actual consumer / Desktop / 100kを再実行したとは主張しない。保持したrawはfreeze前検証の証拠。

完了後は新しいagent/contextへhandoffする。このcontextでClean-room実装を継続しない。新Implementation開始は次のHuman-selected Objective。

## Fresh review-code report

Scope: freezeからclean-slateへのtracked removal / current authority名整理 / readonly evidence保存 / neutral CI。Specification Conformance: Pass（0052、D1〜D6、public契約を保持）。

Tests and Regression Evidence: boundary checker PASS。621 frozen assets / 186退役、250fixtures byte保持、55JSON・JSONL/gzip evidence、176YAML、TOML、168Markdown、530 Requirement definitions、relative file / heading linksを検査。negative checksは旧workspace混入・fixture改変・expected欠落・不正complete Stateを全て拒否し、元bytesを復元した。git diff --check PASS。

Rationale Freshness: retired mechanismsのrationaleはforensicに限定し、保存対象のsafety invariant / consumer値 / baseline protocolはcanonical / oracleへroute。Evidence Integrity: Requirement / ADR / corpus参照を保持。旧test参照はforensic tagへ変換。performance数字はraw datasetごとに分離し、再測定とは表示しない。

Architecture: 新product implementation / skeletonなし。visible runtime sourceはconsumer oracleの2C#だけ、neutral checker以外のadapterなし。Git tagから旧Core / GUI / .NET sourceが存在することをobject存在で確認し、旧solutionを再配置していない。

Findings: exact Candidate scanでcanonicalの旧component名1件を発見し、同じresolved value編集能力というobservable契約へ補正。再scanでBlocking None identified。Non-blocking: actual Windows Desktop / Unity未実施は既存conformance条件へ引継ぎ済みで、今回の削除によるgapではない。

Verdict: Ready to deliver clean-slate boundary（branch CI successはexact CandidateのDevelopment Stateでreconcileする）。旧product conformanceを実行したというverdictではない。
