# Human Decisions Required

Status: Draft

materialなproduct / source-format choiceのみ。監査method、型・lock・transport、artifact構成の選択は含めない。このqueueを作ることはどちらかのoptionを承認したことではない。current canonicalは [適用境界](README.md) に従い存続する。

## D1 — 独立Diff view

**Question:** file単位の独立Diff viewを保つか、compareをunsaved Save-preview / Conflict / Migrationのcontextへ集約するか。

A evidence: [GUI-SHELL-LAYOUT-001](../gui/app-shell.md) は独立viewを要求。[GUI-DATA-DIFF](../gui/data-editor/spec.md) は未保存candidateの理解を目的とする。既存workflowの入口を残せる。

B evidence: Vision / 今回のdirectionはquiet daily authoringとGit ecosystemの利用。context compareで理解を満たせればpersistent tabを減らせる。ただし現DiffはGit history browserと同義ではなく、Gitで未保存candidateの比較を全部置換できるという証拠はない。

Simpler if removed: shellの常設view・mode switch。Risk: Save前の比較の発見性低下。Recommended framing: **compare capabilityを残し、常設独立viewが必要かを選ぶ**。Save / Conflict / Migrationの安全性削減を選択肢にしない。

## D2 — Typed Filter

**Question:** typed filterを日常authoring capabilityとして残すか、まずFind / relevant problem navigationに絞るか。

A evidence: [authoring-query](../specs/authoring-query.md) / [GUI query](../gui/data-editor/spec.md) はread-only subset / typed predicatesを定め、長いTableの作業範囲を狭める用途がある。

B evidence: 最近のdirect authoring directionはfield editを中心とする。全operator UIを維持するdaily価値の独立Human evidenceはP1/P2/P3 package承認ほど明瞭ではない。

Simpler if removed: typed filter builder・draft/applied state・subset reorder制約の一部。Risk: targeted authoring / issue triageが難しくなる。Recommended framing: **Searchの継承とFilterのoperator範囲を別々に決める**。shared semantic query engineの存在はpermanent UIの理由にならない。

## D3 — View Sort

**Question:** single-field view sortを残すか、source順を基本にFindへ寄せるか。

A evidence: current query specはinvalid/nullの順、stable tie、source bytes無変更を定め、値の探索に利用できる。

B evidence: sorted viewはrow occurrence / source orderとの違いを説明する必要があり、direct reorderとのmode frictionを持つ。Sortのdaily価値をFilterと同一にする根拠はない。

Simpler if removed: sort state / comparator projection / reordering mode restriction。Risk: 大量値の比較・探索が不便。Recommended framing: **Sortを独立に評価し、column order / Build canonical order / source orderingは変えない**。

## D4 — Advanced Batch

**Question:** Fill / range Set Null / explicit preview等の高度batchをcore daily UIに残すか、直接pasteと個別操作をbaselineにするか。

A evidence: [authoring-batch](../specs/authoring-batch.md)、[grid authoring](../gui/data-editor/grid-authoring.md) はall-or-none typed batch、preview、10k paste inputを持つ。反復修正を減らす可能性がある。

B evidence: [0040](../spec-changes/0040-desktop-direct-authoring.md) は普通のpasteを直接buffer操作へ変更。capabilityを常設controlにする必然性はない。高度batchの範囲を残すだけで多くのsurface / stateを維持する。

Simpler if removed: Fill/Set Null preview workflowとrange modal state。Risk: repetitive authoring時間の増加。Recommended framing: **clipboard copy/paste・typed lossless codec・一回Undoを保持した上で、高度batchの必要範囲とdisclosureを選ぶ**。existing PKのbatch制約を黙ってordinary PK editingへ拡張しない。

## D5 — Saved Table Overview

**Question:** saved-source Table Overview / profile previewを独立GUI capabilityとして残すか、editing / Build previewのcontextへ集約するか。

A evidence: [Table Overview](../gui/table-overview/spec.md) はphysical filesを横断してsaved Table / profile inclusionを理解する面。File Viewのdirty bufferとは明確に異なる。

B evidence: One Table surfaceと日常編集優先では別surfaceの発見・切替コストがある。Build profile semantics自体は消せないが、常設Overviewの必要性は別choice。

Simpler if removed: saved/dirty query modelのdual UI、独立navigation entry。Risk: cross-source / build inclusionの理解が弱くなる。Recommended framing: **saved cross-source inspectのuser needと入口を決め、Build selection / tagsの意味を維持する**。

## D6 — YAML flow mapping

**Question:** Approved subsetのflow mapping禁止を次実装でも維持するか、現在受理・保存されるflow mappingを明示的に互換syntaxとして承認するか。

A evidence: [YAML-SUBSET-007](../specs/yaml-subset.md) はflow mappingをMUST NOTとし、unsupported例も明示する。小さいsource subsetとsafe patchabilityを守れる。

B evidence: core `source_edit::tests::flow_custom_member_insertion_preserves_existing_mapping_bytes` は `{...}` のparseとpreservationを保護し、このauditでfocused testが実際にpassした。complex value materializationにもflow mappingの例がある。導入履歴 `fee882c` / scanner `bc0a28c` は機能実装を示すが、flow mapping許容の明示Human decisionは確認できなかった。

Simpler if restricted: syntax / patch compatibilityの範囲を抑えられる。Risk: 現在受理される既存sourceをrejectする可能性。Allowedにするrisk: Approved syntaxの拡張・ambiguity処理とpreservationの追加義務。

Recommended framing: **現在のsource使用実態とparser/patch oracleを確認し、source formatの明示deltaとして決定する**。test存在を承認の代用にせず、禁止specだけを根拠にcompatibilityを切らない。今回production修正もcanonical改変も行わない。

## Queue外のgap

Tier1 p95、Unity actual compile/runtime、inline compatibility oracleの抽出不足は追加evidenceの仕事であり、low-level設計をHumanへ選ばせる質問ではない。documented public compatibilityは保持をdefaultとし、将来具体的なbreakが必要になった時だけその変更をgateへ上げる。
