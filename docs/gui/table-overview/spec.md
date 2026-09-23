# GUI仕様: Table Overview

Status: Approved

Table Overviewは分割されたlogical Tableを保存済みsnapshotで横断表示し、Profile selection、query、source navigationをread-onlyで提供する。dataset semanticsは[Authoring Query](../../specs/authoring-query.md)、Profile semanticsは[Build Selection](../../specs/build-selection.md)が所有する。適用記録は[仕様変更0017](../../spec-changes/0017-desktop-workspace-settings.md)を参照する。

## 規範要件

### GUI-OVERVIEW-001

OverviewにはTable、source file/occurrence、保存済みsnapshot、Profile、dirty file数、dirty config有無を表示し、未保存変更は未反映と識別できなければならない（MUST）。cellはread-only。refresh、sourceへ移動、search/filter/sort、Profile選択をkeyboardで実行できる。empty / loading / partial / unavailable / staleを区別する。
source/config変更を検出したら表示をstaleにし、read-only refreshでdirty bufferを保存・破棄してはならない（MUST NOT）。古いresponseは新query/profile/resultへ適用しない。

### GUI-OVERVIEW-002

rowからsourceへ移動するときは、対象file snapshot identityとeditor base identityの一致をshared layerで確認しなければならない（MUST）。一致するdirty editorでは同じoccurrenceを選択し、未保存値は保持する。Pending deleteならそのUndo導線へ移動する。
不一致ならfileを開いてsnapshot差を示してよい（MAY）が、rowを推測選択せずOverview refreshを案内する。PK/nameで再接続してはならない（MUST NOT）。queryで非表示なら一時的にqueryをclearしたことを伝え、source rowにfocusする。

### GUI-OVERVIEW-003

supported scalar search/filter/sortはshared application snapshotをpresentationするだけでなければならない（MUST）。
Overview表示だけでsource、Build、Publish、Gitを変更してはならない（MUST NOT）。

## 受け入れ証拠

 dirty base一致/不一致、Pending delete、stale response、query clearとfocus、0件とUnavailableの区別、supported query composition、saved snapshot boundaryを検証する。
