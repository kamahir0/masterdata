# GUI仕様: Table Editor

Status: Approved

## 目的

Table schemaとrecord authoringを、backend operationの手順ではなくspreadsheet-firstのdirect manipulationとして提供する。利用者はfield headerとrecord gridを主操作面とし、field name / type、field追加、record値を対象物の場所で直接編集できなければならない。

Schema Migrationのdeterministic Plan、source-preserving rewrite、lost-update preflight、multi-file commit / rollback、Recovery Requiredは引き続きshared core/applicationが所有する。ただし通常のnon-destructive操作でPlan / Diff / Applyのbackend手順を利用者へ逐次要求してはならない。GUIはuser intentをshared application operationへ翻訳し、本当に利用者判断が必要なdestructive / conflict / blocked conditionだけを前景化する。

## レイアウト（Layout）

### GUI-TABLE-LAYOUT-001

Workspace Explorerでshared source semantics上のTable schema document、またはそのTableに属するrecord-bearing documentを選択した場合、main areaはTable schema contextとrecord gridを連続したauthoring surfaceとして表示しなければならない（MUST）。folder名やphysical pathをTable identityとして扱ってはならない（MUST NOT）。

schema fileがinline recordsを持つ場合は同じsurfaceでschema headerとrecordsを直接編集できなければならない（MUST）。separate Data documentでも対応Tableのfield headerを同じgrid contextで表示し、Advanced settingsへ到達できなければならない（MUST）。

### GUI-TABLE-LAYOUT-002

record gridのcolumn headerはfield nameとtypeを常時確認でき、schema mutation capabilityが利用可能な場合はfield nameをinline input、typeをcompact selectorとして直接編集できなければならない（MUST）。Nullable / Array、Primary / Secondary Key、MessagePack key等の補助情報はheader badge / secondary detail / Advanced settingsから確認可能でなければならない（MUST）。

表示値とedit capabilityはshared application/domain snapshotから取得し、frontendがYAMLを独自parseして再構成してはならない（MUST NOT）。

### GUI-TABLE-LAYOUT-003

日常的なschema authoringとして少なくとも`Add Field`、`Rename Field`、`Change Field Type`をdirect manipulationで提供しなければならない（MUST）。field追加はcolumn header列の右端に常時到達可能な`+` affordanceを持たなければならない（MUST）。Renameはheader name input、type変更はheader type selectorから開始する。

Drop Fieldはcolumn context action等のsecondary surfaceへ置いてよい（MAY）が、destructive confirmation contractを維持する。field reorder、MessagePack key変更、Primary / Secondary Key編集、Table renameのdirect manipulationは本Objectiveの必須範囲ではなく、Advanced settingsまたは後続scopeとしてよい（MAY）。

### GUI-TABLE-LAYOUT-004

Table EditorはReferenceごとに、current domain name、optional exact `csharpName`、shared snapshotが返すeffective C# helper name、ordered source
fields、target table、target key fields、resolved single/multi、resolved required/optional、およびshared validation diagnosticを確認できなければ
ならない（MUST）。target key matching、type compatibility、nullable validity、integrity、query method derivation、helper name derivationはfrontendが
再実装してはならない（MUST NOT）。

### GUI-TABLE-LAYOUT-005

Migration Plan / Diff / Applyはbackend safety mechanismとして維持しなければならない（MUST）が、non-destructiveな日常操作のprimary workflowとして常時表示または逐次confirmationを要求してはならない（MUST NOT）。

GUIはshared Planを内部取得し、安全に適用できるnon-destructive操作はdirect editのcommitに続けて実行してよい（MAY）。利用者が明示的にDetails / Diffを開ける経路を提供してよい（MAY）。destructive authorization、affected dirty file、stale source、unresolvable migration、Recovery Required等、利用者判断または回復操作が必要な場合だけblocking surfaceを前景化する。

## 状態（States）

### GUI-TABLE-STATE-001

Table Editorはschema loading、ready、inline edit pending、operation in progress、stale / retry required、commit failure with rollback / no mutation、Recovery Requiredを内部状態として区別できなければならない（MUST）。以前のschemaやPlanを新しく選択したTableのcurrent editable stateとして表示してはならない（MUST NOT）。

Plan loading / Plan ready等のbackend lifecycleを、そのまま利用者向けstepとして常時露出する必要はない。

### GUI-TABLE-STATE-002

field headerの未確定text editはlocal interaction stateとして扱い、typing一文字ごとにMigrationをcommitしてはならない（MUST NOT）。Enter、blur、selector choice等のedit確定でshared operationを開始する。Escapeは未確定header editを破棄しcurrent schema値へ戻さなければならない（MUST）。

Schema mutationはData Editorのsource dirty bufferと同一conceptとして扱ってはならない（MUST NOT）。Cmd+S / Ctrl+SをMigration Applyのshortcutへ暗黙に割り当ててはならない（MUST NOT）。

### GUI-TABLE-STATE-003

Migration Planがaffected source fileとして示すfileにData Editorのdirty bufferが存在する場合、Table Editorはautomatic / explicitいずれのApplyも開始してはならない（MUST NOT）。該当dirty fileを利用者が識別できなければならない（MUST）。Planに含まれないunrelated dirty bufferだけを理由にPlanまたはApplyを全面禁止してはならず（MUST NOT）、Migration操作によってそのbufferを保存・破棄・reloadしてはならない（MUST NOT）。

### GUI-TABLE-STATE-004

Migration成功後、affected sourceのcurrent workspace authorityを再取得し、Table Editorをcurrent schemaへ更新しなければならない（MUST）。affected fileについて既存clean editor snapshotをstale contentのままeditableとして残してはならない（MUST NOT）。unrelated dirty bufferは保持しなければならない（MUST）。

## 操作（Interactions）

### GUI-TABLE-INT-001

column header右端の`+`から`Add Field`を開始できなければならない（MUST）。通常操作ではfield declaration一式を入力するmodalを必須にせず（MUST NOT）、shared application layerがunique name候補、explicit MessagePack key候補、default field type、およびexisting recordがある場合に必要なexplicit canonical initializerを含む完全なMigration commandを構成できなければならない（MUST）。

frontendはkey allocation、initializer validity、type validityをdomain authorityとして実装してはならない（MUST NOT）。default fieldは作成直後からheader上でrename / type changeできなければならない（MUST）。

### GUI-TABLE-INT-002

`Rename Field`はcolumn headerのfield nameを直接編集し、Enterまたはblurで確定するinteractionをprimary pathとしなければならない（MUST）。current logical Table identityとcurrent field nameをtarget selectorとしてshared Migration boundaryへ渡し、frontendがschema / data / key / Referenceを文字列置換してはならない（MUST NOT）。

shared MigrationがReference source/target componentを追随する場合もbackend/applicationがaffected setを所有する。成功後はheaderとaffected clean data viewをcurrent workspace authorityへ更新する。

### GUI-TABLE-INT-003

`Drop Field`はdestructive actionとして明示的に識別できなければならない（MUST）。Apply前に対象Table / fieldとdestructive natureを確認する明示的confirmationを要求しなければならない（MUST）。GUI confirmationだけをbackend authorizationの代替にしてはならず（MUST NOT）、shared application boundaryへmachine-actionable destructive execution authorizationを渡さなければならない（MUST）。

### GUI-TABLE-INT-004

source mutation前にshared Migration Planを取得しなければならない（MUST）。ただしnon-destructive direct editでPlan summary / Diff / Apply buttonを利用者へ必ず表示してから進める必要はない（MUST NOT）。

Plan failure、affected dirty file、stale state、destructive authorization requirement等で自動適用できない場合はsource mutationを開始せず（MUST NOT）、該当column / operationの文脈でstructured diagnosticと必要なactionを提示しなければならない（MUST）。

### GUI-TABLE-INT-005

利用者は必要に応じてcurrent Migration Planのaffected fileごとのbefore / after Diffへ到達できなければならない（MUST）。Diffを日常的なnon-destructive schema editのmandatory gateにしてはならない（MUST NOT）。

Diffはcurrent Planのbase sourceとtransformed candidateを比較し、別のcurrent workspace readやgenerated artifactを代用してはならない（MUST NOT）。

### GUI-TABLE-INT-006

automatic / explicitいずれのApplyも、確定したuser intentとbase snapshotに対応するcurrent Planだけを対象にしなければならない（MUST）。Plan作成後にoperation inputが変わった場合、old Planを適用してはならない（MUST NOT）。commit preflightがsource/config/membershipのstale stateを検出した場合はsource mutationを開始せず、stale operationとして表示し、retry / re-plan相当のactionを提供しなければならない（MUST）。silent retry / silent overwriteを行ってはならない（MUST NOT）。

### GUI-TABLE-INT-007

Migration成功をBuild / Publish / Git / generated artifact更新と同一操作へ結合してはならない（MUST NOT）。成功後にBuildを別actionとして実行できてよい（MAY）。

### GUI-TABLE-INT-009

Reference add/edit/removeはraw YAMLをfrontendで編集せず、shared source-preserving mutation boundaryを通らなければならない（MUST）。
Plan / Apply、exact-source lost-update protection、stale rejection、recovery、no implicit Build / Publish / Git side effectはfield
migrationと同じcontractを維持しなければならない（MUST）。`csharpName`の追加・編集・削除もshared source-preserving mutationを通り、
frontendは`REF-008..009`の命名・return semanticsを再実装してはならない（MUST NOT）。

### GUI-TABLE-INT-010

field type selectorから`Change Field Type`をdirectに開始できなければならない（MUST）。shared Migrationの`ChangeFieldType`がcurrent recordsとschema dependenciesをtarget typeでlosslessに成立させられる場合だけsource mutationを成功させる。

current valueのcoercion、default replacement、Reference / key dependencyの暗黙修復をfrontendまたはGUI convenienceとして行ってはならない（MUST NOT）。shared operationがrejectした場合はold typeをcurrent schemaとして維持し、該当columnからreasonを確認できなければならない（MUST）。

## キーボード（Keyboard）

### GUI-TABLE-KEY-001

Table / record gridはkeyboardだけでcell selection、cell edit、column header name edit、type selector、Add Field、Drop Field等のschema actionへ到達できなければならない（MUST）。Enter / Escapeはheader editのcommit / cancelとして一貫して動作し、IME composition中のEnterをcommitへ誤解釈してはならない（MUST NOT）。

destructive confirmationやblocking recoveryをkeyboard userだけが操作不能なsurfaceにしてはならない（MUST NOT）。

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

None identified for this direct-manipulation slice. Exact React component、popover placement、column width、virtualization library integration、key/default suggestion presentation、Diff layoutは、上記observable contractを満たす限りimplementation detailとする。