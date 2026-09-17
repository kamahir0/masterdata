# 仕様変更: Desktop制作v1 — P3 Project入口・Build / Publish

Status: Applied

## Affected Specifications

Applied canonical owners:

- [Project Initialization](../specs/project-init.md): `PROJECT-INIT-001`〜`002`
- [Project Workflow](../gui/project-workflow.md): `GUI-PROJECT-001`〜`002`
- [Build Request / Publish Preview](../specs/build-request-preview.md): `BUILD-REQUEST-001`, `PUBLISH-PREVIEW-001`〜`002`
- [Build / Publish GUI](../gui/build-publish/spec.md): `GUI-DELIVERY-001`〜`006`

P1/P2のownerは[0016](0016-desktop-daily-editing.md)と[0017](0017-desktop-workspace-settings.md)に記録する。

## 根拠と分類（Source Evidence and Classification）

- Decision: 2026-09-16、HumanはDesktop制作v1 P1–P3を一つのwork packageとして詳細化する方向を選択した。
- Requirement: 新規Projectから制作、保存、Profile選択、Build、Unity向けPublishまでGUIから完遂できること。
- Constraint: shared native services、saved-source-only Build、receipt authority、all-target preflight、target-local failure、.NET委譲を維持する。
- Approval: 2026-09-18、Human maintainerが0016–0018を一括で明示Approvalした。

元のProposed全文、44 requirementの設計根拠、self-reviewとrefinement過程はGit historyに残す。本artifactは適用後のaudit recordでありimplementation authorityではない。

## 提案する差分（Proposed Delta）

承認済みdeltaは上記canonical ownerへ適用済み。安全なGUI Create Project、captured saved-input Build request、read-only Publish preview / stale recheck、Build / Publish GUI lifecycleをApproved contractとして追加した。

次の既存境界を維持する。

- canonical Buildとexternal Publishは別operation。
- Buildは保存済みsource/configを入力とし、dirty bufferを暗黙Saveしない。
- Publish-onlyはreceipt authorityを使用し、current YAML freshnessを推測しない。
- all-target preflight、target-local failure、partial success、path safetyは既存Build pipeline contractに従う。
- GUI Createはshared Project initializationを使い、sample data、Unity directory、Git repositoryを暗黙作成しない。

## 互換性（Compatibility）

source format、Table/key identity、generated C#、binary、receipt、CLI grammarを変更しない。GUI confirmationをCLI automationへ要求しない。

## 受け入れと実装への影響（Acceptance and Implementation Impact）

P1–P3の統合完了条件は、新規Projectで型・Table・Dataを作成し、complex initializerとrecordを入力し、scalar paste→Undo→Redo→Save、Tag / Profile / target設定、OverviewとProblemsによる確認、明示ProfileのBuild、Unity向けC# / binary PublishまでDesktop GUIの通常経路で完遂できること。

error-pathではexternal conflict、stale preview、Save All部分失敗、Recovery Required、Publish部分失敗でlocal inputとunmanaged destination contentを保持する。

性能は10万record（分割file）、20列Table、1万cell pasteの固定生成inputについてhardware / OS / build mode / dependency versionとload/query/preview/validation時間・peak memoryを測定evidenceとして残す。製品上限やms保証はこのpackageでは設定しない。

実装はApplied artifactではなくApproved canonical ownerを入力とし、Requirement IDとtests / fixtures / GUI workflowsをtraceする。

## 未解決事項（Open Questions）

None within P1–P3. P4 key/source移動、P5 expression、P6 Reference/Webは別package。

## レビュー（Review）

一括reviewでBlocking Issues: None、Non-blocking Issues: None、Questions: None、Approved as Proposed: Yes。Source preservation、validation non-blocking、Migration initializer、Build Selection、receipt authority、path safety、multi-target executionとの整合を確認した。

## 承認記録（Approval Record）

2026-09-18 Human Approval。0016–0018を一括承認し、canonicalへ適用してimplementation-readyへ進める指示を受領。3 changeを同一packageとして`Applied`とした。
