# Development State

Stage: decision-required
Candidate: none

## Blocking findings

None.

## Human decision needed

Type Editor v1のmutation strategyを次の3案から選ぶ。

- **Shared Type Migration v1 + Plan / Diff（推薦）** — shared core/applicationがdependency resolution、source-preserving rewrite、stale-plan preflight、multi-file rollback / Recovery Requiredを所有し、Type Editorはsemantic command入力・Plan / Diff確認・authorization・Apply結果表示に限定する。v1ではValue Object conversion setting、Enum / Flags member Add / Rename / Drop、Custom Type field Add / Rename / Dropを扱う。
- **selected type fileのtyped direct edit** — 選択中type declarationだけをtyped formで編集・保存し、dependent sourceは自動変更しない。保存後にproject validationで不整合を表示する。
- **dependency-free changeだけに限定** — project-wide rewriteを必要としない変更だけをType Editor v1で許可し、rename / add / drop等の主要workflowは後続へ送る。

推薦案では、type rename、underlying変更、member numeric value変更、Custom Type field type / modifier / key / reorderはv1非対象とする。

RFC参照ラベルは順にOption C / A / Bだが、Human decisionでは上記の意味のある名称を主ラベルとして扱う。
