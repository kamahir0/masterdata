# 仕様変更: Standalone Web workspace

Status: Draft

## Affected Specifications

- [Runtime hosts](../specs/runtime-hosts.md) `RUNTIME-HOST-001`, `RUNTIME-HOST-003`, `RUNTIME-HOST-005`, `RUNTIME-HOST-007`, `RUNTIME-HOST-011..013`は既存Approved boundary。
- Browser workspaceの具体的なread/writeとGUI stateは、承認後に独立したcanonical ownerへ適用する。

## 根拠と分類（Source Evidence and Classification）

- Human Decision: 2026-09-21 JSTにStandalone Web authoring v1を次のObjectiveとして採用。
- Requirement: 許可した既存local workspaceのopen、source閲覧・編集・検証・保存、共有Rust semantics、静的配布可能な成果物。
- Constraint: Native Host / Connected Web / native Build / Publish / 実サイト公開はObjective外。
- Approved constraint: `RUNTIME-HOST-007`の明示的workspace許可、`RUNTIME-HOST-013`のpure coreとhost I/O境界。
- Agent Decision candidate: browser directory pickerで各sessionにworkspaceを明示選択し、handleを永続化しない。API availabilityで起動時に利用可否を示し、未対応環境で暗黙のfilesystem fallbackを行わない。
- Browser API evidence: [File System Access specification](https://wicg.github.io/file-system-access/)のpicker、permission、writable stream contract、および[File System Standard](https://fs.spec.whatwg.org/)のwrite stagingを確認。これらのAPI surfaceからは、外部processとのatomic compare-and-swapを行う手段は確認できない（agent inference）。

## 提案する差分（Proposed Delta）

### Workspace acquisition

- Browser Hostはuser gestureによるdirectory selectionをworkspace authorityの起点にする。選択されたdirectoryとその子handle以外をsource discovery/read/write対象にしない。
- 選択directory直下の`masterdata.toml`をshared Rust parserで解釈する。configured source rootはproject-relative logical pathとして扱い、absolute path、parent traversal、workspace外参照を拒否する。
- Browserがdirectory pickerを提供しない、またはpermissionが拒否・失効した場合、Web UIは操作不能の理由と再選択actionを表示し、未保存bufferを自動破棄しない。

### Shared semantics

- Browser Hostはsource bytesとlogical pathをshared Rust semantic boundaryへ渡し、YAML classification、Data snapshot、edit preview、validationをJavaScriptで再実装しない。
- Native Build / Publish capabilityを持たないStandalone状態では、それらの操作を実行前にdisabledとして表示する。

### Source write safety — Human decision pending

- 提案: 単一source Saveは、base content identityと保存直前の現在content identityを比較する。不一致なら書き込まずconflictを返し、local bufferを維持する。
- 提案: 書込み後に再読込しcandidate identityを確認する。書込み・検証中の失敗は、観測できたbytesに応じfailureまたはoutcome_unknownを返す。outcome_unknownの自動再試行を禁止する。
- Browser local-file APIには外部applicationとの原子的なcompare-and-swapがないため、照合とcommitの間に外部変更が起きるraceを完全には除去できない。この残余riskを受容するか、Webのin-place Saveを延期するかはHuman decision待ち。
- Table / Typeのmulti-file mutationについては、既存migration safety contractを満たすdurable recovery mechanismを別途設計する。単一file Saveの決定からmulti-file commit承認を推定しない。

## 互換性（Compatibility）

Desktop / CLIの既存contract、YAML/source format、project identityを変更しない。Browser内のlogical workspace identityをOS absolute pathまたはpermission tokenとして公開しない。

## 受け入れと実装への影響（Acceptance and Implementation Impact）

- permissionのない起動、explicit selection、unsafe root拒否、source classification、Rust validation、buffer保持、conflict、失敗後のrecheckをfocused evidenceにする。
- supported browserの実操作と、Desktop / CLIの回帰を確認する。WASM compileだけではruntime evidenceとしない。
- shared frontendからTauri固有I/Oを分離し、Browser adapterを追加する。

## 未解決事項（Open Questions）

- Browser in-place Saveの残余raceを受容するか。
- Table / Type multi-file mutationのjournal、rollback、crash recoveryとbrowser permission再取得のcontract。
- 具体的browser support matrix。API capability detectionとformal support statementを区別する。

## レビュー（Review）

Pending. Human gateはsource write safetyとmulti-file recoveryを中心に判定する。

## 承認記録（Approval Record）

Pending.
