# Product / Desktop Constitution案

Status: Draft

これはrewrite用の精製案。[適用境界](README.md) に従う。各判断の根拠とriskは [registry](classification.md)、materialな未決定は [queue](human-decisions.md) が所有する。

## What MasterData is

MasterDataは、human-readable YAMLをcanonical sourceにする、MasterMemory / Unity向けMasterDataのDesktop-first authoring environmentである。人が編集でき、Gitでreviewできるsourceを保ち、schema-awareな編集と厳格なdeliveryを提供する。

日常workflowは **Open → Find Table/source → Edit directly → Understand relevant problems → Save**。CLIはautomation / CI / scriptingの重要なsurfaceである。Desktopの日常interactionをone-shot CLI operationの順序や粒度に従属させない。

MasterDataが所有するのはschema/type-aware authoring、semantic validation、source-preserving mutation、migration、deterministic buildと明確なpublish boundaryである。YAMLはsource authority、shared Rust semanticsは意味論のauthorityである。frontendは編集表現を所有し、YAML解釈・型解決・lost-update判定を複製しない。

この方向は [Vision](../product/vision.md)、[0040 direct authoring](../spec-changes/0040-desktop-direct-authoring.md)、[0041 unified Table](../spec-changes/0041-unified-table-editor.md)、[0050 interactive navigation](../spec-changes/0050-interactive-navigation.md) と今回のHuman promptに基づく。Visionのcore構成・extension段階はproductの意味と区別して再記述する候補であり、既存crate構成を継承する指示ではない。

## Desktop UX Principles

| Principle | 継承する意味 | 固定しないもの |
| --- | --- | --- |
| Data first | 日常編集の対象と問題の理解にspaceを使う | 現行component tree / pane比率 |
| Quiet by default | persistent UIは日常作業上の理由を持つ。正常状態の説明を常設しない | 全commandをtoolbarへ並べること |
| Direct manipulation | scalar / Complex Value / orderを対象で操作する。普通の編集にtransaction footerを要求しない | exact dialog / popover / button名 |
| Progressive disclosure | 高度・低頻度・危険な操作は必要なcontextで開く | capabilityがあるだけで常設すること |
| Contextual actions | 対象と作用範囲がわかる場所で操作する | icon library / exact pixel |
| Stable editing surface | routine status、diagnostics、query stateでgridやpointer下の対象を移動させない | 特定CSSやDOM順序 |
| One Table surface | schemaとrecordsを同じTableの編集contextで理解する | inline/separateを異なる日常UIに分けること |
| Independent physical source lifecycles | sourceごとのdirty/history/query、選択を混同しない | 現行state variable / store topology |
| Shared semantic feedback | 解釈・validation・mutationはshared layerの判断を表示する | CLI transportとの共通化 |
| Capability does not imply persistent UI | engineが持つ機能を全部常設しない | 既存command数とcontrol数の一致 |
| Persistence topology does not dictate interaction topology | inline / separate / mixedでも一貫したauthoringを行う | physical sourceの安全な所有単位を隠すこと |

重複防止のため、具体的なSave、Undo、Conflict等の境界は [domain-safety](domain-safety.md)、latencyは [performance](performance.md)、操作例は [acceptance matrix](acceptance-matrix.md) が所有する。

### Human UX knowledgeを残す粒度

[0045](../spec-changes/0045-direct-complex-value-authoring.md)〜[0050](../spec-changes/0050-interactive-navigation.md) から、直接編集、sticky spatial context、掴んだrow/columnの追従preview、neighborの移動、cancel時の無変更、header内の名前とhandleの非重複、上端・左端の外側にscrollの隙間を作らないこと、compactな一列toolbar、即時navigation feedbackを継承する。

20px grip / 40px toolbar等のtest calibrationをHumanのexact pixel要件へ昇格しない。0047の「正しい挿入線」は0048のpreviewに置換されているため、現行presentationとして両立を要求しない。keyboard / focus / reduced motion / context lossの安全な操作は失わない。

## Product Non-goals案

concreteなHuman-selected needなしに、次をcore / persistent UIへ追加しない。

- general-purpose Git client、Git history browser、staging / merge / reviewの再実装。
- collaboration platform、issue tracker、AI providerの独自統合を日常authoringの前提にすること。
- general text editor、spreadsheet feature completeness、無制限のquery / batch / computed DSL。
- plugin frameworkやgeneral incremental compilerをrewriteの前提にすること。
- retired Web hosts、Released Compatibility、Computed View v1の再導入。

これは既存のcontextual compare、clipboard paste、reference semantics、Build profile selectionを削除する決定ではない。独立Diff view / Filter / Sort / advanced Batch / Overviewは [Human queue](human-decisions.md) で個別に評価する。Gitが提供するhistoryと、未保存candidate / Conflict / Migrationの理解に必要なcompareは責務が異なる。

non-goalの根拠はVision、[0022 Web retirement](../spec-changes/0022-retire-web-product-hosts.md)、[0029 rejected Git expansion](../spec-changes/0029-git-native-collaboration-automation.md)、[0030](../spec-changes/0030-retire-released-compatibility.md)、[0031](../spec-changes/0031-retire-computed-view-v1.md)。将来の具体的要求を永久に禁止するものではない。
