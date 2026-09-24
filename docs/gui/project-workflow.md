# GUI仕様: Project Workflow

Status: Approved

この仕様はDesktopでのCreate Project、logical Table/Type navigation、Recent Projectsを定義する。Project作成のshared serviceは[Project Initialization](../specs/project-init.md)、Project identityは[Project layout](../specs/project-layout.md)が所有する。適用記録は[仕様変更0018](../spec-changes/0018-desktop-build-delivery.md)を参照する。

## 規範要件

### GUI-PROJECT-001

未選択時にOpen / Createを提示し、Createはdestinationとproject inputを確認してから実行しなければならない（MUST）。既存Projectが開いている場合は、作成開始前に全dirtyのSave All / Don't Save / Cancel guardを通す。Cancelでは作成しない。成功後は新Projectを開き、失敗時は旧Projectを保持する。
入口はfolder作成、型、Table、Data file、Add Rowへのguided actionを提供するが、それぞれを独立した明示operationにしなければならない（MUST）。schema creationでData fileを暗黙作成しない。empty stateもkeyboardで次操作へ進める。

Create画面はnative directory pickerと手入力でdestinationを指定でき、picker cancelは入力を変更しない（MUST）。作成開始前のCancelはProjectを作成せず元の画面へ戻り、Project未選択ならWelcomeとRecent Projectsへ戻る（MUST）。作成実行中はCancelとform編集を無効にする（MUST）。destinationの有効性は既存shared serviceが判定し、frontendへfilesystem semanticsを追加しない。

### GUI-PROJECT-002

Explorerのsource treeを維持し、logical Table一覧からOverview / schema、Type一覧からType Editorへ到達できなければならない（MUST）。一覧はshared workspace情報を使い、pathからidentityを導出しない。どの入口も同一sourceのbufferを共有する。
Recent Projectsはuser-localに最大10件、successful open順で保持する（MUST）。同じhostで同じcanonical rootは一件として扱い、missing/permission failureで他Projectを破壊しない。removeはrecent entryだけを消しdiskを変更しない。自動openや自動Buildを行わず、path情報をProjectのGit管理configやremoteへ送らない（MUST NOT）。

## 受け入れ証拠

dirty Cancelで作成なし、失敗で旧Project保持、成功後guided actions、recent removeでdisk不変を検証する。
