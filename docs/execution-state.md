# Development State

Stage: decision-required
Candidate: none

## Blocking findings

- P4-A: existing record key editのkey scope、batch scope、追加warning / confirmation policyが未決定。
- P4-B: source file moveのdestination scope、dirty buffer policy、destination conflict / overwrite、case-only rename supportが未決定。

## Human decision needed

Comparison owner:
- `docs/spec-changes/0019-existing-record-key-edit.md`
- `docs/spec-changes/0020-source-file-rename-move.md`

P4-A:
- **A1: Primary Keyのみ** — scopeは小さいが、Secondary Key構成fieldは引き続きread-only。
- **A2: Primary + Secondary Key** — key fieldという現在のGUI分類を一貫してeditableにする。**推薦**。
- **B1: single-cell editのみ** — 初期scopeを小さくし、batchによる大量key変更を後段化。**推薦**。
- **B2: batchも含む** — paste / fill / range operationまで同時にkey fieldへ開放。
- **C1: 追加modal confirmationなし** — key field表示とvalidation diagnosticsは維持し、通常typed editとして扱う。**推薦**。
- **C2: warning / confirmation必須** — key変更前またはSave前に追加interactionを要求。

P4-B:
- **D1: same source root内のrename / move** — rename系filesystem semanticsを限定し、cross-filesystem transactionを避ける。**推薦**。
- **D2: configured source roots間moveも含む** — 利便性は高いが、cross-filesystem copy/delete・rollback/Outcome Unknown設計が必要。
- **E1: move前にdirtyをSave / Don't Save / Cancelで解決** — path rebind中のunsaved stateを避ける。**推薦**。
- **E2: dirty bufferを保持してnew pathへrebind** — seamlessだがConflict / watcher / Undo lifecycleが複雑。
- **F1: destination既存時はConflict、Overwriteなし** — destructive replacementをinitial sliceへ入れない。**推薦**。
- **F2: explicit Overwriteを追加** — destination data loss用の別authorizationとrecheckが必要。
- **G1: case-only renameをTier 1でsupport** — OS差をuser-visible restrictionにしない。**推薦**。
- **G2: case-only renameはinitial non-scope** — implementationは単純だがOSによって通常renameの期待が割れる。

短い「進める」は、推薦bundle **A2 + B1 + C1 + D1 + E1 + F1 + G1** の選択として扱える。選択後はDraftへdecisionを反映し、review可能な`Proposed`へ収束させる。implementation classへはまだ進まない。
