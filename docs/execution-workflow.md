# Execution Workflow

Status: Active

開発の継続、自律実行、Human gate、現在地、Git deliveryのcanonical owner。仕様のlifecycleは[Specification Workflow](contributing/specification-workflow.md)が所有する。

## Development phase

一般公開（v0.1.0以降）への移行に伴い、第三者の利用者が存在する前提で開発を進める。既存のプロジェクト定義（`masterdata.toml`）、YAMLデータ形式、主要なCLIコマンド・IPC契約は安定性を重視し、既存ユーザーに影響を与える破壊的変更は慎重に検討する。

実在する作業データの喪失、セキュリティ、外部への不可逆な作用は厳格に保護し、既存authorizationと[Human gate](#human-gate)に従う。

## Legacy boundary

2026-10-17 Human requestにより、新しいGUIプロジェクトを独立したGit履歴で開始する。旧リポジトリ `masterdata-legacy2` のGit履歴・関連資料は、2026-10-29のリポジトリ整理に伴いGitHub外のバックアップに保持する。

旧仕様、ADR、テスト、oracle、CI基準、実装、作業stateは、新リポジトリのauthorityや完成条件として継承しない。旧資料の読込みをcold-startの必須手順にしない。旧会話内の指示も現在の実行指示にはならない。

引き継ぐのは仕様駆動開発の方法と、製品を評価するための知見。必要な知識・挙動・実装を再利用する際は、新しいObjectiveとの関係を確かめ、必要なbehaviorをこのリポジトリの仕様ワークフローで採用する。旧実装との一致だけを正しさの根拠にしない。

## Fresh-session recovery

1. `AGENTS.md`と、現在のbranch / working tree / upstream / remote HEADを確認する。新規リポジトリでHEADやremoteがない場合は、その事実を確認する。
2. [Development State](execution-state.md)からStage、Candidate、Work base、残作業を読む。
3. [Current Objective](current-objective.md)から目的、完了条件、non-scopeを確認する。
4. Work baseからHEADのGit差分とworking treeを確認する。`none`なら初期commitからのhistoryを使う。ドキュメントの記述よりGitの実際の状態を優先する。
5. 今回の作業に関係する仕様、設計理由、コードおよびテストを確認する。

前提が変わらない限り同じ文書を繰り返し読まない。外部変更、不可逆な操作、commit / push、最終報告の前は、関係する仕様とGitの実際の状態を再確認する。

cleanでupstreamへstrictly behindなbranchはsafe fast-forwardしてよい。それ以外をreset / stash / rebase等で勝手に整合させない。

## Autonomous execution

HumanがObjectiveを選択した時点で、その範囲の調査、仕様の具体化、review、承認可能な判断、実装、検証、修正、通常のdeliveryを委任したものとして扱う。stageやsub-featureの完了はturn boundaryではない。

2026-10-17の実装Objectiveでは、Humanは通常の製品・技術判断をlegacy2の資料と新製品の目的から補完・導出することも明示委任した。資料に答えがあっても旧制約を無条件に採用せず、新GUIとの適合性を判断してAgent Decisionとして残す。この委任内の選択をHuman gateへ戻さない。実データの破棄・secret・公開release等へは拡張しない。

実装案を機械的に処理するだけでなく、ユーザーが達成したい作業、問題の一般性、類似箇所、より単純な設計を検討する。細かい内部選択やroutine approvalだけを理由にHumanへ戻さない。

「continue」「進める」は選択済みObjectiveの範囲で継続する指示。完了後の新しい製品範囲や、future ideaを自動で選択する指示にはしない。Humanが明示した停止範囲を守る。

## Human gate

既存authorizationから安全に決められない次の場合だけHumanへ判断を戻す。

1. 新しいObjective、または選択済み範囲の重要な拡張。
2. 実データの意図的破棄、history rewrite、外部の不可逆操作。
3. security / permission / authentication / secret等のboundary変更。
4. release、製品のpublish、deployment、課金・契約等の外部作用。明示依頼されたrepository作成や通常のcommit / pushは、委任された範囲で実行できる。
5. development phaseの前提が変わった後の、未承認の互換性破壊。
6. 目的や既存authorityから決められず、製品の方向を大きく分ける選択肢。
7. authorityの矛盾や環境制約により、必要な結果を安全に得られない場合。

到達した場合は、具体的な判断、選択肢、影響、推奨、止まる理由と、その判断後に自走する範囲を提示する。Human gateに該当しない可逆なObjective内の判断はagentが解決する。

## Development State

[execution-state.md](execution-state.md)は現在地のpointerであり、作業日誌・仕様本文・テストlogを置かない。

| Stage | 意味 |
| --- | --- |
| `designing` | 仕様・設計を整理している |
| `decision-required` | 具体的なHuman gateで止まっている |
| `implementation-ready` | 必要な仕様がApprovedで、実装可能 |
| `verification-ready` | exact Candidateを最終検証できる |
| `correction-ready` | 具体的なBlockingを修正できる |
| `objective-complete` | 選択された完了条件を満たした |

`Candidate`は最終検証対象の40文字commit SHAまたは`none`。`verification-ready` / `correction-ready` / `objective-complete`ではSHAを記録する。`Work base`は差分確認の起点SHA。履歴のない初期整備では`none`でよい。`Active work`はwork package単位のCompleted / In progress / Remainingだけにする。

`decision-required`には`Human decision needed`、`correction-ready`には具体的なBlocking findingsを記録する。完了時は`Active work: None.`とし、次のObjectiveを勝手に設定しない。

## Git delivery and completion

- Humanが`commitしない` / `pushしない`と指定しない限り、scope内の変更をreview / check後にcurrent branchへcommitし、通常のfast-forward pushで届ける。無断branch switch、force-push、history rewriteはしない。
- unrelatedなdirty changeは混ぜない。長い作業では、意味のあるsliceがfocused validationを通った時だけcheckpoint commitを作ってよい。壊れた途中状態をresume用としてcommitしない。
- commit / PR説明は原則日本語。commit bodyには背景・目的、変更内容、検証を残す。
- 変更に応じたfocused tests、self-review、`python3 tools/check_workspace.py`、`git diff --check`を行う。製品実装のverificationには、仕様のacceptanceと代表作業に対応する証拠を用いる。
- candidate commit後はexact SHAをstateへ記録し、authoringから離れたfresh reviewと必要なCIを確認する。Candidate自体にSHAを埋めるためのhistory rewriteは行わず、stateだけのmetadata commitを使ってよい。
- Candidate以後のstateだけのmetadata commitは製品の証拠を無効にしない。metadataを再検証するためだけの無限のcommit / CI loopを作らない。
- Blockingは具体的に修正し、影響する検証を再実行する。好みの改善や新しい目標を最終段階で完了条件へ追加しない。
- required CI pendingはHuman gateではない。成功を確認するまで完了扱いにしない。実行不能・未取得の証拠は理由と限界を報告し、取得済みと主張しない。
- 最終報告直前にremote HEAD、working tree、stateを確認する。実際に変えたもの、検証、残る制約を簡潔に報告する。

初期CIはワークスペースの整合性だけを確認する。製品のruntimeや技術選定がない段階で、旧製品の検証基盤を復元する必要はない。実装後はscopeに合った検証を追加し、構造checkを製品品質の証明に使わない。
