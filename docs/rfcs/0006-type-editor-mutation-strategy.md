# RFC: Type Editor v1 mutation strategy

Status: Accepted

## 背景（Context）

Humanは、Table Editor Objective完了後の次priorityとしてType Editorを選択した。

現在のGUIはWorkspace ExplorerからValue Object、Enum、Flags Enum、Custom Typeを新規作成できるが、作成済みtype documentには専用editorがない。各type categoryの静的なschema/data/generated C# semanticsはApproved Type System specificationで定義済みである。一方、既存type declarationを変更するときのdependency rewrite、source mutation、lost-update、rollback、destructive authorization、既存dataの扱いには専用のcanonical mutation contractがなかった。

既存のSchema Migration v1はTable fieldの`AddField` / `RenameField` / `DropField`だけを対象とし、Type Editorやtype-definition mutationをscope外としている。したがって、Type Editorを単なるGUI convenienceとして実装し、frontendまたはTauri adapterでtype YAMLを書き換えることはできない。

## 課題（Problem）

Type definitionの変更はselected fileだけで完結しない場合がある。

- Enum / Flags member renameは既存dataのsymbolic valueへ影響する。
- Enum / Flags member dropは既存dataがそのmemberを使用している場合にreplacementを自動決定できない。
- Custom Type field addは既存のすべてのCustom Type value occurrenceへ新memberを導入しなければschema/data semanticsと整合しない。
- Custom Type field rename / dropはnested mappingを含む既存dataへ影響する。
- type name、underlying、field type/modifier、MessagePack key、member numeric value等の変更はgenerated API、serialization、validation、compatibilityへ追加の意味を持つ。

このため、Type Editor v1では「どの変更を許可するか」と「依存sourceをどう安全に変換するか」を先に決める必要があった。

## 目標（Goals）

- 既存Value Object / Enum / Flags Enum / Custom Typeをraw YAML手編集なしで変更できる方向を決める。
- Type Systemのdomain semanticsをfrontendへ複製せず、shared core/application boundaryを維持する。
- project-wide dependencyを持つ変更について、silent rewrite / silent data loss / stale overwriteを避ける。
- Type Editor v1を1つのbounded implementation work packageへ切れるmutation operation setを決める。
- source-preserving YAML、Git diff、structured diagnosticsという既存product directionを維持する。

## 非目標（Non-Goals）

本RFC自体はproduct specificationではなく、implementation authorityではない。また、次をこのRFCだけで承認しない。

- exact GUI layout / component choice。
- Type declaration file/folderのrename、move、delete、duplicate。
- general-purpose raw YAML editor / formatter。
- arbitrary type conversion language、SQL-like migration language、bulk scripting。
- released-version compatibility system全体。
- Build / Publish / Git operationとの自動結合。

## 選択肢（Options）

### Option A: selected type fileのtyped direct edit

Type Editorでselected type declarationをtyped formとして編集し、単一source fileのcandidateを保存する。dependency sourceは自動変更せず、保存後のproject validationで問題を表示する。

利点:

- implementationが小さい。
- single-file source editとして理解しやすい。

欠点:

- Custom Type field addやEnum member renameで、操作直後に既存dataとの不整合を意図的に作り得る。
- dependent source rewriteの責務が利用者のraw YAML手編集へ戻る。
- どのinvalid stateを保存可能とするかという新しいschema-edit policyが必要になる。
- Type Editorの主要目的である「安全な既存definition編集」を十分に満たさない。

### Option B: dependency-free changeだけをType Editor v1へ限定

dependent source rewriteを必要としない変更だけを初期UIへ出す。例えばValue Object conversion flagの変更や、未使用の新規Enum member追加などに限定する。

利点:

- project-wide transactionを導入せずに安全な範囲を作りやすい。
- implementation量が小さい。

欠点:

- type categoryごとの編集能力が非対称で、Custom Typeはほぼ表示専用のまま残る。
- 利用者が期待するrename / add / dropの主要workflowがYAML手編集へ残る。
- 後続で結局project-wide migration contractが必要になる可能性が高い。

### Option C: shared Type Migration v1を導入し、Type EditorはそのGUI adapterにする

Type definition mutationをshared semantic operationとして定義し、mutation前にdeterministic Planとaffected-file Diffを生成する。dependency resolution、source-preserving rewrite、postcondition、lost-update preflight、multi-file commit / rollback / Recovery Requiredはshared core/application boundaryが所有する。Type Editorはsemantic command input、Plan/Diff表示、explicit authorization、Apply結果表示だけを担当する。

Type MigrationはSchema Migration v1を無制限に拡張せず、別canonical ownerとして定義する。既存Schema Migrationの安全性patternは再利用してよいが、Type-specific semanticsを`MIGRATION-*`へ黙って追加しない。

利点:

- Table Editorと同じPlan/Diff-first authoring modelを維持できる。
- dependent data rewriteをfrontendやHumanの手作業へ押し戻さない。
- stale plan、multi-file failure、destructive operationをfail-closedで扱える。
- Desktop以外のfuture adapterでも同じsemantic engineを利用できる。

欠点:

- core/application workがOption A/Bより大きい。
- initial operation setを明示的に狭く切らないとwork packageが肥大化する。

## トレードオフ（Trade-offs）

Type Editor v1の価値は「既存type definitionを安全に変更できること」にあるため、単にform UIを追加するだけでは主要な断点を閉じない。一方で、type rename、underlying/type変更、member numeric value変更、field modifier/key変更まで一度に扱うとcompatibilityとdata conversion policyが急激に広がる。

そのため、project-wide safetyは最初からshared migration boundaryへ置きつつ、operation setは「既存valueを決定論的に変換でき、arbitrary value conversionを要求しない変更」に限定するのが最も小さいcoherent sliceと判断した。

## 採用した方向（Accepted Direction）

**Option C: Shared Type Migration v1 + Plan / Diffを採用する。**

採用した初期operation setは次の範囲でcanonical specificationへ移す。

- Value Object: conversion setting変更。
- Normal Enum / Flags Enum: member Add / Rename / Drop。
- Custom Type: field Add / Rename / Drop。
- type declaration rename、underlying変更、Enum/Flags member numeric value変更/reorder、Custom Type field type/modifier/key変更/reorder、type category conversionはv1非対象。

project-wide safetyはshared Type Migration boundaryが所有し、Type EditorはPlan / Diff / Applyのthin adapterとする。source mutation成功はBuild / Publish / Git / generated artifact更新を暗黙に開始しない。

詳細なobservable contractは次のcanonical candidateへ移した。このRFCはrationaleを保持するがimplementation authorityではない。

- [Type Migration v1仕様](../specs/type-migration.md)
- [GUI Type Editor仕様](../gui/type-editor/spec.md)

## 互換性（Compatibility）

Type Migration operationはsource YAML、generated C# API、binary inputへ影響し得る。v1ではcompatibility surfaceをboundedにするため、type declaration rename、underlying変更、Enum/Flags numeric value変更、Custom Type field type/modifier/key変更を除外する。

`RenameEnumMember`および`RenameCustomField`はgenerated C# identifierを変更し得るため、source migrationとして成功してもexternal consumerのsource compatibilityまで保証しない。released-version compatibility systemは別scopeである。

## 未解決事項（Open Questions）

RFCのadoption decisionは解決済み。canonical Type Migration / GUI Type Editor specificationのreviewとHuman Approvalは、それぞれのspecification lifecycleで扱う。

exact GUI component、Plan panel layout、field/member row action placement、default focus等、data safety / compatibility / semantic outcomeを変えないinteraction detailはGUI specificationとimplementation detailの境界に従う。

## 決定（Decision）

2026-09-14、Human maintainerは**Shared Type Migration v1 + Plan / Diff（Option C）**をType Editor v1のdesign directionとして採用した。

このdecisionはRFCを`Accepted`にするが、product specificationを自動で`Approved`にはしない。implementationは、[Type Migration v1仕様](../specs/type-migration.md)と[GUI Type Editor仕様](../gui/type-editor/spec.md)が明示的なHuman Approvalを受けるまで開始しない。
