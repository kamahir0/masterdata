# Current Objective

## Role

この文書はHuman-selectedなcurrent priorityとwork package boundaryの唯一のownerである。
Approved behaviorは[仕様index](specs/README.md)のcanonical specification、現在のStageは
[Development State](execution-state.md)、implementation realityはcode / tests / Gitで確認する。

## Objective

現在のHuman priorityは、**根源的な要望と既決の仕様を統合し、まとまった実装へ移れる理想のauthoring systemを設計・文書化する**ことである。

2026-09-16、Humanは小刻みな進め方からまとまった実装へ移る意向を示し、その準備として創造性を発揮した仕様案の作成を明示依頼した。
直前のComplex Value Authoring v1はcandidate `fee882c2ef3c3516ded184707c6cc2d96cd4d252`のverificationを経て完了している。

## Why now

型・Table・recordのauthoringとBuild / Publishの既存基盤を、日常制作が完結する体験へまとめる時期にある。
個別機能の順番だけでなく、完成像、scope、失敗時の境界、受け入れscenario、実装packageを先に共有する。

## Completion boundary

- Product motivation、Approved decisions、実装evidence、未決事項を区別した文書がある。
- 理想の操作体験と、ひと区切りになる完成条件、後段の拡張が具体化されている。
- 提案にcanonical owner、compatibility / implementation impact、failure / acceptance scenarioがある。
- 提案を承認済みとせず、Humanがまとまった方向選択を行える。

成果物は[Authoring system v1 RFC](rfcs/0008-authoring-system-v1.md)。比較・推薦・未決detailのownerは同RFCとする。

## Current direction

Human-selectedなのは設計・文書化のscopeである。RFCの**Option B: P1–P3のDesktop制作v1**は推薦であり、選択済み実装priorityではない。
RFC採用とcanonical specificationのApprovalを区別し、未承認behaviorの実装は開始しない。

## Explicit non-scope

- 本activityでのproduct feature実装。
- Approved specificationの未承認semantic変更。
- RFCの自動Accepted化、specificationの自動Approved化。
- 後段candidateをHuman選択なしに実装priorityへ昇格すること。

## Next candidate

HumanがRFCの方向を選択した場合、採用scopeのowner別spec-changeを一括で具体化・reviewし、明示Approval後にcanonicalへ適用する。
方向選択だけで本格実装開始とはしない。代替案とtrade-offはRFCのOptions / Open Questionsを参照する。

## Relevant authorities

- [Product vision](product/vision.md)
- [Authoring system v1 RFC](rfcs/0008-authoring-system-v1.md)
- [Specification index](specs/README.md)
- [GUI index](gui/README.md)
- [Specification workflow](contributing/specification-workflow.md)
- [Development workflow](execution-workflow.md)
