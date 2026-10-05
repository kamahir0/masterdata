# ADR 0009: Desktop workspaceの所有と局所projection

Status: Accepted

## 背景（Context）

[constitution](../rewrite-preparation/constitution.md)、[performance](../gui/performance.md)、[source safety](../specs/source-edit.md)から、selectionごとにProjectを再構築せず、sourceのsyntaxと意味、readとwrite authorityを分離する必要がある。旧module / command / component構成は判断材料にしない。

## 決定（Decision）

shared Rust engineはsource bytes、byte rangeを持つ構文木、schema-directed interpreter、resolution、patch derivationを所有する。構文位置はTree-sitter YAMLから取得し、quote / block decodingにはYAML parserのscalar eventsを使用する。parserのimplicit numeric categoryをdomainへ渡さない。mutationは位置を限定したpatchと再parse postconditionで成立させる。

Project openでinventoryとparsed documentsを作り、Desktop workspaceはimmutable read generationとphysical sourceごとのdraft/historyをsession中保持する。draftはread cacheと別の所有であり、source bytesを変更したphysical sourceだけdirtyになる。selectionは必要なsource/dependencyのfreshnessを確認してbounded projectionを返す。background diagnosticsはgenerationと対象を保持してpublishする。selection queueは実行中一件と最新pending一件に集約し、obsolete result / focusを受理しない。

native write boundaryはactual path / binding / bytesをfresh確認してcommitを認可する。read generationはwrite authorizationを提供しない。file別の結果で成功したdraftだけbase/historyを進める。Migration / Build / Publishはordinary edit lifecycleとは別に捕捉したinputを扱う。

Desktop hostにはTauri 2を使う。macOSのWKWebView / WindowsのWebView2でnative text control、IME、keyboard、accessibilityを利用でき、Rust libraryを直接呼べることから選んだ。表示はReact + Ant Designをbaselineとする。standard controls / menus / overlays / theme / motionをAnt Designへ集約し、single active editorとbounded authoring gridはcustom Reactで表現する。汎用Tableのrow lifecycleへauthoring identity / spatial drag / viewport transportを従属させず、gridも同じdesign tokensを使う。cellごとのhidden editorや全Table payloadを作らない。transportは表示範囲、resolved field descriptor、lossless value、operation intent/resultに限定する。selection / scroll / focus / range / drag previewはpresentation-only。構文木、型解決、write authorityはDesktopへ渡さない。

CLIは同じengineをshort-livedに使用する。.NET process invocationはnative delivery boundaryに集約し、validated Rust valuesとgenerated C#をnative MasterMemoryへ渡す。YAMLを.NETで再解釈しない。

Desktop delivery jobはworkspace actorから分離したnative workerで実行する。長いcompile / destination I/Oがsource selectionやdraft authoringを待たせないためである。immutable BuildPlanを確定するまでのinput取得と、その後のartifact / structural mutationの排他を別に扱い、Project epochに対応した結果を保持する。

session内history / projection / diagnosticsは永続sourceではない。shared configはproject、reconstructable project UI stateはproject-local、theme / recent projectsはOS per-user application storageへ置く。

## 結果（Consequences）

targeted freshness、immutable publication、draft ownership、fresh write checkの各境界にobservable testを置ける。syntax受理、semantic validity、safe patch capabilityを個別に扱える。viewportを超えるrecordsはRust側に残す。最初のvertical sliceでproject-wide work counts、transport量、frame / accepted interactionを測り、局所編集のreparse costも測定する。

## 代替案（Alternatives）

CLI operation wrapperはDesktopのlifetimeとwarm invariantsを満たさない。native immediate-mode gridはtext / IME / assistive interactionの独自実装量が増える。全semantic modelのfrontend保持は意味論とgeneration ownershipを二重化する。初期のplain DOM choiceは[2026-10-04 Human presentation decision](../spec-changes/0053-desktop-presentation-quality.md)が置換した。控えめなsurfaceでもprofessionalなvisual / interaction品質が必要であり、standard UIを自作し続けるよりAnt Designのsystemへ統合する。workspace / source / semantic / writeの判断はこの変更で置換しない。

外部APIの確認: [Tauri process model](https://v2.tauri.app/concept/process-model/)、[async commands](https://v2.tauri.app/develop/calling-rust/)、[Tree-sitter YAML](https://docs.rs/tree-sitter-yaml/0.7.2/tree_sitter_yaml/)。これらはproduct authorityではない。
