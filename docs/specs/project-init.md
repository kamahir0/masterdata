# Project Initialization仕様

Status: Approved

Domain: Project

この仕様はDesktop GUIから新規Projectを安全に作成するshared service contractを定義する。生成されるscaffoldは[Project layout](project-layout.md)の`PROJECT-CONFIG-008`に従う。適用記録は[仕様変更0018](../spec-changes/0018-desktop-build-delivery.md)を参照する。

## 規範要件

### PROJECT-INIT-001

GUI向けCreate Project serviceは、既存のempty directoryか、実在parent直下の未存在new directoryを明示targetとして受け付けなければならない（MUST）。hidden entryを含むnon-empty directory、symlink/junction/reparse point経由のtarget、既存Projectとcollisionするtargetは書込み前にrejectする。空判定だけに依存せず生成fileはexclusive createとし、途中で現れたentryを上書きしてはならない（MUST NOT）。
project id/name等は既存shared init input / validatorを使用する。生成物はPROJECT-CONFIG-008のscaffoldだけとし、sample YAML、Unity directory、Git repositoryを暗黙作成してはならない（MUST NOT）。CLI initの既存directory対応をこのGUI向け入口で撤回するものではない。

### PROJECT-INIT-002

Create successはconfigと必須scaffoldの作成完了・shared Project再解決成功後だけ報告しなければならない（MUST）。failure時は既存workspaceを維持し、作成済みentryと残状態を判定可能な範囲で報告する。multi-file crash atomicityを保証してはならない（MUST NOT）。
部分作成物を無断cleanupしたりblind retryで上書きしてはならない（MUST NOT）。利用者はactual targetを確認し、Open Projectで開けるなら開くか、別のempty/new directoryで明示再試行する。不可逆な自動recoveryを本contractへ導入しない。

## 受け入れ証拠

empty/new success、non-empty/hidden/symlink拒否、raceで既存file不変、途中failureと作成済みentry報告を検証する。
