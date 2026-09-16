# Current Objective

## Role

この文書はHuman-selectedなcurrent priorityとwork package boundaryの唯一のownerである。
Approved behaviorは[仕様index](specs/README.md)のcanonical specification、現在のStageは
[Development State](execution-state.md)、implementation realityはcode / tests / Gitで確認する。

## Objective

現在のHuman priorityは、**Desktop制作v1（P1–P3）をまとまって実装するため、日常編集・Workspace設定・Build / Publishの詳細仕様を一括で具体化し承認可能にする**ことである。

2026-09-16、Humanは小刻みな進め方からまとまった実装へ移る意向を示し、その準備として創造性を発揮した仕様案の作成を明示依頼した。
直前のComplex Value Authoring v1はcandidate `fee882c2ef3c3516ded184707c6cc2d96cd4d252`のverificationを経て完了している。

## Why now

型・Table・recordのauthoringとBuild / Publishの既存基盤を、日常制作が完結する体験へまとめる時期にある。
個別機能の順番だけでなく、完成像、scope、失敗時の境界、受け入れscenario、実装packageを先に共有する。

## Completion boundary

- Product motivation、Approved decisions、実装evidence、未決事項を区別した文書がある。
- 理想の操作体験と、ひと区切りになる完成条件、後段の拡張が具体化されている。
- 提案にcanonical owner、compatibility / implementation impact、failure / acceptance scenarioがある。
- 詳細contractがstable Requirement IDと既存ownerへのdeltaを持ち、一括reviewでBlockingがない。
- 提案を承認済みとせず、Humanが具体的な仕様一式のApprovalを判断できる。

設計方向と比較のownerは[Authoring system v1 RFC](rfcs/0008-authoring-system-v1.md)。詳細contractの承認対象は下記0016–0018であり、一括reviewとcanonical適用手順は0018が所有する。

## Current direction

2026-09-16、Humanは直前summaryへの「進める」により、RFCの**Option B: P1–P3のDesktop制作v1**を選択した。
file単位編集、保存前Undo、scalar一括入力を基本に詳細化し、計算列を後段とする。
RFC採用とcanonical specificationのApprovalを区別し、未承認behaviorの実装は開始しない。

## Explicit non-scope

- 本activityでのproduct feature実装。
- Approved specificationの未承認semantic変更。
- specificationの自動Approved化。
- 後段candidateをHuman選択なしに実装priorityへ昇格すること。

## Next candidate

承認対象は[0016: 日常編集](spec-changes/0016-desktop-daily-editing.md)、[0017: Workspace・設定](spec-changes/0017-desktop-workspace-settings.md)、[0018: Project入口・Build / Publish](spec-changes/0018-desktop-build-delivery.md)の一式。
明示Approval後にcanonicalへatomic適用する。方向選択だけで本格実装開始とはしない。

## Relevant authorities

- [Product vision](product/vision.md)
- [Authoring system v1 RFC](rfcs/0008-authoring-system-v1.md)
- [Specification index](specs/README.md)
- [GUI index](gui/README.md)
- [Specification workflow](contributing/specification-workflow.md)
- [Development workflow](execution-workflow.md)
