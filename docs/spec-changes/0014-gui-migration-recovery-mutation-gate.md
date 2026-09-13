# 仕様変更: Migration Recovery Required時のproject-level source mutation gate

Status: Applied

## Affected Specifications

- `docs/gui/app-shell.md` — `Status: Approved`
  - `GUI-SHELL-STATE-001`
  - `GUI-SHELL-CAPABILITY-001`
- Related new specification: `docs/gui/table-editor/spec.md` — `Status: Approved`
- Canonical domain authority: `docs/specs/schema-migration.md` — `MIGRATION-010`

## 根拠と分類（Source Evidence and Classification）

- **Decision / Human priority**: HumanはSchema Migration v1のAdd/Rename/DropをGUI Table Editorから安全に実行するObjectiveを選択した。
- **Approved constraint**: `MIGRATION-010`はMigration commitが`Recovery Required`になった場合、それ以上のintentional mutationを停止し、automatic silent continuationを禁止している。
- **Architecture constraint**: Data Editor Save / Overwrite、Source Creation、Table Editor Apply等は同一Projectのcanonical YAML sourceをmutateする別surfaceであり、Table Editorだけをdisabledにしてもdomain requirementを満たせない。
- **Ownership constraint**: cross-surfaceなproject-level command availability / stateはTable Editor単体ではなくGUI app shellがcanonical ownerになるべきである。

## 提案する差分（Proposed Delta）

`docs/gui/app-shell.md`へ、Migration `Recovery Required`をproject-level source mutation safety stateとして扱うruleを追加する。

1. shared application boundaryがMigration commit resultとして`Recovery Required`を返した場合、GUI shellはそのProjectを**source mutation recovery-required state**として扱わなければならない（MUST）。
2. recovery-required state中は、canonical YAML sourceを意図的に変更するGUI commandを開始してはならない（MUST NOT）。少なくとも次を含む。
   - Data Editor Save / Save All / explicit Overwrite
   - Source Creation
   - Table Editor Migration Apply
   - 将来追加されるsource rename / delete / move等のsource-mutating command
3. read-only操作は継続してよい（MAY）。少なくともExplorer navigation、Problems閲覧、Diff / source inspection、current workspace stateのre-read、recovery guidance表示まで一律に禁止してはならない（MUST NOT）。
4. Buildは保存済みsourceを読む別operationであるが、`Recovery Required`中はcanonical source setがold/newどちらとして整合しているか確定していないため、GUI shellからnormal Buildを開始可能として表示してはならない（MUST NOT）。safe source stateが再確立された後に通常capabilityへ戻す。
5. recovery-required stateは利用者が原因、affected file state、利用可能なrecovery informationを確認できるpersistentまたは再確認可能なsurfaceを持たなければならない（MUST）。単なるtoastだけでstateを消費してはならない（MUST NOT）。
6. shellがsource mutationを再度有効化してよいのは、shared application / host boundaryがactual workspace sourceを再取得し、安全なsource stateを確立した後だけである（MUST）。frontend local flagの解除だけでrecovery完了扱いしてはならない（MUST NOT）。
7. exact recovery command、journal format、manual file recovery UI、crash/power-loss transaction保証はこのdeltaで固定しない。

Canonical適用時は、既存`GUI-SHELL-STATE-001`のoperation failure stateにRecovery Requiredのproject-level persistenceを追記し、`GUI-SHELL-CAPABILITY-001`のcommand availabilityへrecovery-required mutation gateを追加する。Requirement IDを増やすか既存IDへ統合するかはcanonical wording適用時に重複を避けて決定する。

## 互換性（Compatibility）

通常stateでのData Editor Save、Source Creation、Build、Explorer navigationのbehaviorは変更しない。変更されるのはMigration multi-file commitがrollback不能となり、Approved domain contract上すでに追加mutation停止が要求されるexceptional stateだけである。

YAML format、Migration semantics、Build semantics、Tauri wire shapeを変更しない。GUI shellはshared resultをcross-surface command availabilityへ反映するだけである。

## 受け入れと実装への影響（Acceptance and Implementation Impact）

- Migration `Recovery Required`をReact workflowで注入し、Table Editor ApplyだけでなくData Editor Save / Save All / Overwrite、Source Creation、Buildが開始されないことを確認する。
- Explorer navigation、Problems / Diff閲覧、re-read/recovery guidanceが引き続き到達可能であることを確認する。
- frontend local stateを単に閉じる・別fileへnavigateするだけでmutation gateを解除できないことを確認する。
- shared applicationからsafe source stateを再取得した後にだけ通常command availabilityへ戻ることを確認する。
- Tauri/frontendがMigration rollback semanticsやfilesystem recoveryを独自実装しないことを確認する。

## 未解決事項（Open Questions）

None for the initial GUI safety gate. Recovery workspaceをGUIからどこまで直接操作するか、dedicated recovery wizardを設けるかは別Objective候補とする。

## レビュー（Review）

Self-review: このdeltaは`MIGRATION-010`の既存Approved safety requirementをGUI cross-surface behaviorへfaithfully mapするもので、通常時のsource mutation semanticsを変更しない。Table Editor specへglobal shell ruleを重複配置せず、project-level command availabilityのownerであるapp shellへroutingする。

## 承認記録（Approval Record）

- Human approval: 2026-09-13。このsessionで提示したTable Editor v1と本deltaの2件に対する「承認」。
- Canonical application: 本承認反映commitで`docs/gui/app-shell.md`の`GUI-SHELL-STATE-001` / `GUI-SHELL-CAPABILITY-001`へatomicに適用。既存IDを維持する。
- Table Editor v1も同時にApprovedへ移行し、cross-surface gateの参照先をapp shellのcanonical ownerへ更新した。
- review-spec照合: intent、内部整合、cross-spec整合、用語、規範強度、testability、互換性、未決定事項、adapter boundary、scopeを確認。Blocking / Non-blocking / QuestionsはNone identified。Approved as Proposed: Yes。これはreview判定であり、status変更の根拠は上記Human approvalである。
