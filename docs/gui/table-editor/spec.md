# GUI仕様: Table Editor

Status: Approved

## 目的

既存Table schema documentをWorkspace Explorerから選択し、Schema Migration v1の`AddField` / `RenameField` / `DropField`とReference declaration authoringをdeterministic Planとsource Diffを確認しながら安全に実行できるschema-aware editorを提供する。

Table / field / key / type semantics、Migration dependency resolution、source-preserving rewrite、lost-update preflight、multi-file commit / rollbackは[Schema Migration v1](../../specs/schema-migration.md)と既存domain specificationsが所有する。本仕様はそれらを再定義せず、Table Editorのlayout、state、interaction、dirty-buffer composition、surface-local error/recovery、adapter boundaryを所有する。`Recovery Required`中のcross-surface command gateは[GUI app shell](../app-shell.md)の`GUI-SHELL-STATE-001` / `GUI-SHELL-CAPABILITY-001`が所有する。

## レイアウト（Layout）

### GUI-TABLE-LAYOUT-001

Workspace Explorerでshared source semantics上のTable schema documentを選択した場合、main areaはTable Editorを表示しなければならない（MUST）。folder名やphysical pathをTable identityとして扱ってはならない（MUST NOT）。Table Editorは少なくともlogical Table identity、schema source provenance、field declaration orderを確認できなければならない（MUST）。

### GUI-TABLE-LAYOUT-002

Table Editorは各fieldについて少なくともMessagePack key、field name、type、Nullable / Array modifierを表示しなければならない（MUST）。Primary Key / Secondary Key membershipも利用者が識別できなければならない（MUST）。表示値はshared application/domain snapshotから取得し、frontendがYAMLを独自parseして再構成してはならない（MUST NOT）。

### GUI-TABLE-LAYOUT-003

初期mutation actionは`Add Field`、既存fieldに対する`Rename Field`、`Drop Field`だけを表示しなければならない（MUST）。field type変更、modifier変更、field reorder、MessagePack key変更、Primary / Secondary Key編集、Table renameを同じdirect-edit UIとして提供してはならない（MUST NOT）。

### GUI-TABLE-LAYOUT-004

Table EditorはReferenceごとに、current name、ordered source fields、target table、target key fields、resolved single/multi、resolved
required/optional、およびshared validation diagnosticを確認できなければならない（MUST）。target key matching、type compatibility、
nullable validity、integrity、query method derivationはfrontendが再実装してはならない（MUST NOT）。

## 状態（States）

### GUI-TABLE-STATE-001

Table Editorは少なくともschema loading、ready、operation input editing、Plan loading、Plan ready、Apply in progress、stale plan / re-plan required、commit failure with rollback / no mutation、Recovery Requiredを観測上区別できなければならない（MUST）。以前のschemaやPlanを、新しく選択したTableのcurrent editable / applicable stateとして表示してはならない（MUST NOT）。

### GUI-TABLE-STATE-002

Migration operation inputやPlanはData Editorのsource dirty bufferと同一conceptとして扱ってはならない（MUST NOT）。Table Editorの`Apply`だけがMigration source mutationを開始し、Cmd+S / Ctrl+SをMigration Applyのshortcutへ暗黙に割り当ててはならない（MUST NOT）。operation inputはPlan / Apply failureで不必要に失ってはならない（MUST NOT）。exact navigation-time retention policyはimplementation detailとしてよい。

### GUI-TABLE-STATE-003

Migration Planがaffected source fileとして示すfileにData Editorのdirty bufferが存在する場合、Table EditorはApplyを開始してはならない（MUST NOT）。該当dirty fileを利用者が識別できなければならない（MUST）。Planに含まれないunrelated dirty bufferだけを理由にPlanまたはApplyを全面禁止してはならず（MUST NOT）、Migration操作によってそのbufferを保存・破棄・reloadしてはならない（MUST NOT）。

### GUI-TABLE-STATE-004

Migration成功後、affected sourceのcurrent workspace authorityを再取得し、Table Editorをcurrent schemaへ更新しなければならない（MUST）。affected fileについて既存clean editor snapshotをstale contentのままeditableとして残してはならない（MUST NOT）。unrelated dirty bufferは保持しなければならない（MUST）。

## 操作（Interactions）

### GUI-TABLE-INT-001

`Add Field`はSchema Migration v1の`AddField` semantic commandを構成するguided inputを提供しなければならない（MUST）。少なくともMessagePack key、name、type、Nullable / Array modifier、およびMigration semantics上必要なexplicit initializerを入力できなければならない（MUST）。

frontendはinitializerやtype validityを独自の簡易Type System / YAML validatorで確定してはならない（MUST NOT）。initializer transportは64-bit integer、sequence、mapping、`null`等のApproved canonical valueをlosslessにshared application boundaryへ渡せる形でなければならない（MUST）。exact wire shape / widgetは固定しない。

UIはMessagePack keyの候補値を提案してよい（MAY）が、それをsemantic auto-allocation ruleとして扱ってはならず（MUST NOT）、最終keyはPlan inputとして明示的でなければならない（MUST）。

### GUI-TABLE-INT-002

`Rename Field`はcurrent logical Table identityとcurrent field nameをtarget selectorとしてshared Migration boundaryへ渡さなければならない（MUST）。frontendがschema / data / key referenceを文字列置換してはならない（MUST NOT）。Rename UIからMessagePack key変更を暗黙に行ってはならない（MUST NOT）。

### GUI-TABLE-INT-003

`Drop Field`はdestructive actionとして明示的に識別できなければならない（MUST）。Apply前に対象Table / fieldとdestructive natureを確認する明示的confirmationを要求しなければならない（MUST）。GUI confirmationだけをbackend authorizationの代替にしてはならず（MUST NOT）、shared application boundaryへmachine-actionable destructive execution authorizationを渡さなければならない（MUST）。

### GUI-TABLE-INT-004

source mutation前に必ずMigration Planを取得しなければならない（MUST）。Plan surfaceは少なくともoperation / target、destructive state、affected source files、affected record count、Migration validation / diagnosticsを表示しなければならない（MUST）。Plan作成・表示はsourceを変更してはならない（MUST NOT）。Plan failureをApply successとして扱ってはならない（MUST NOT）。

### GUI-TABLE-INT-005

Planからaffected fileごとのbefore / after Diffへ移動できなければならない（MUST）。Diffはcurrent Planのbase sourceとtransformed candidateを比較し、別のcurrent workspace readやgenerated artifactを代用してはならない（MUST NOT）。Diff表示はnonmodalでもmodalでもよいが、operation input、Plan identity、選択fieldを不必要に失ってはならない（MUST NOT）。

### GUI-TABLE-INT-006

Applyはcurrent UIが示しているsemantic commandとbase snapshotに対応するcurrent Planだけを対象にしなければならない（MUST）。Plan作成後にoperation inputを変更した場合、old PlanをcurrentとしてApplyしてはならない（MUST NOT）。commit preflightがsource/config/membershipのstale stateを検出した場合はsource mutationを開始せず、stale planとして表示し、re-plan actionを提供しなければならない（MUST）。silent retry / silent overwriteを行ってはならない（MUST NOT）。

### GUI-TABLE-INT-007

Migration成功をBuild / Publish / Git / generated artifact更新と同一操作へ結合してはならない（MUST NOT）。成功後にBuildを別actionとして実行できてよい（MAY）。

### GUI-TABLE-INT-009

Reference add/edit/removeはraw YAMLをfrontendで編集せず、shared source-preserving mutation boundaryを通らなければならない（MUST）。
Plan / Apply、exact-source lost-update protection、stale rejection、recovery、no implicit Build / Publish / Git side effectはfield
migrationと同じcontractを維持しなければならない（MUST）。Reference helperのpublic C# method namingとOptional non-unique return
contractは別Human gateであり、declaration authoringはそのchoiceを発明してはならない（MUST NOT）。

## キーボード（Keyboard）

### GUI-TABLE-KEY-001

Table Editorはkeyboardだけでfield selection、Add / Rename / Drop action開始、operation form入力、Plan確認、Diffへの移動、ApplyまたはCancelへ到達できなければならない（MUST）。destructive confirmationをkeyboard userだけが操作不能なsurfaceにしてはならない（MUST NOT）。

## フォーカス（Focus）

### GUI-TABLE-FOCUS-001

ExplorerからTable schemaを開いた場合、main Table Editorへ移動できる明確なkeyboard pathを持たなければならない（MUST）。operation dialog / panelを閉じた場合は、可能な範囲で開始元fieldまたはactionへfocusを復元しなければならない（MUST）。Plan / DiffからTable overviewへ戻る場合も、可能な範囲でtarget field selectionを復元しなければならない（MUST）。

## 検証（Validation）

### GUI-TABLE-VAL-001

Table Editorのschema/type/dependency/initializer validationはshared Migration / Type System semanticsを使用しなければならない（MUST）。frontend独自のYAML parser、field collision rule、key dependency rule、type compatibility ruleをdomain authorityとして実装してはならない（MUST NOT）。Migration Planのblocking diagnosticとproject-wideのunrelated diagnosticを同一のApply gateとして誤解釈してはならない（MUST NOT）。Migration Resolvableの判定はSchema Migration v1へ委譲する。

### GUI-TABLE-VAL-002

Plan / diagnosticがsource locationまたはaffected fileへmap可能な場合、利用者が該当source / fieldを識別できる形で表示しなければならない（MUST）。unmappable project-level diagnosticも失ってはならない（MUST NOT）。

## エラー（Errors）

### GUI-TABLE-ERR-001

Migration commitがmutation開始前に失敗した場合、またはcommit failure後にcomplete old source setへrollbackできた場合、Successとして表示してはならない（MUST NOT）。operation inputとPlan情報を可能な範囲で保持し、原因確認とre-plan / retryへのpathを提供しなければならない（MUST）。

### GUI-TABLE-ERR-002

Migration commitが`Recovery Required`となった場合、Table Editorはその状態を明示し、affected file stateと利用可能なrecovery informationを表示できなければならない（MUST）。cross-surfaceなsource mutation停止は[GUI app shell](../app-shell.md)の`GUI-SHELL-STATE-001` / `GUI-SHELL-CAPABILITY-001`へ委譲し、Table Editor独自の別gateを実装してはならない（MUST NOT）。exact recovery command / journal UIは本sliceで固定しない。

### GUI-TABLE-ERR-003

Table schemaをshared semanticsで安全にresolveできない場合、別document kindとして誤って編集可能にしてはならない（MUST NOT）。structured error stateを表示し、既存のunrelated dirty bufferやExplorer selectionを不必要に破壊してはならない（MUST NOT）。

## アクセシビリティ（Accessibility）

### GUI-TABLE-A11Y-001

field list / table、operation controls、Plan summary、Diff navigation、destructive confirmation、error / Recovery Required stateはassistive technologyからpurposeとstateを識別できなければならない（MUST）。stateやdestructive severityを色・iconだけで伝えてはならない（MUST NOT）。

## Adapter boundary

### GUI-TABLE-ADAPTER-001

Tauri frontendはMigration Plan derivation、YAML source patch、dependency resolution、lost-update preflight、rollback transactionを実装してはならない（MUST NOT）。shared Native Application ServiceをTauri commandから呼び出し、frontendはserializable view model / command inputを扱う薄いadapterでなければならない（MUST）。

current core実装に不足する`RenameField` / `DropField`はGUI内で代替実装せず（MUST NOT）、Approved Schema Migration v1へshared core/application implementationを追随させなければならない（MUST）。

## 参照artifact（Reference Artifacts）

None.

## 検証evidence

少なくとも次をfocused core/application / adapter / React workflow evidenceで検証する。

- schema selectionでTable Editorが開き、Data / Type documentをTableとして誤解釈しない。
- AddField inputからshared Planを生成し、initializer-required / invalid type / key collision等をshared diagnosticとして表示する。
- RenameFieldがschema declaration、対象Table records、Primary / Secondary Key等のApproved dependencyをshared engineで整合して更新し、MessagePack keyを保持する。
- DropFieldがexplicit destructive authorizationなしではmutationせず、key/index dependencyを黙って削除しない。
- Plan / Diffはmutation前で、affected files / record count / diagnosticsをcurrent Planから表示する。
- plan後のexternal source/config/membership changeをApplyがstaleとして拒否し、sourceを上書きしない。
- affected dirty Data Editor bufferがApplyをblockし、unrelated dirty bufferは保持される。
- Migration success後にaffected clean editor snapshotがreloadされ、unrelated dirty bufferを破棄しない。
- commit failure + rollback successとRecovery RequiredをSuccessと混同しない。
- source-preserving regressionでunrelated comments、blank lines、quote / indentation、unaffected file bytesを保持する。
- Tauri / frontendにfilesystemやYAML patch semanticsを複製しない。

## 未解決事項（Open Questions）

None identified for the initial Table Editor v1. Exact Ant Design component、Plan panelのmodal / inline配置、field-row action placement、key suggestion algorithmのpresentation、Diff layout、initializer widget / serialized wire shapeは、上記observable contractを満たす限りimplementation detailとする。
