# GUI app shell（GUIアプリシェル）

Status: Approved

## 目的

Tauri v2 applicationをthin desktop adapterとして保ち、Project open、Workspace Explorer、typed editor、validation、Buildをshared application/domain semanticsへ接続する。

## レイアウト（Layout）

### GUI-SHELL-LAYOUT-001

Desktop authoring画面は、左の[Workspace Explorer](explorer/spec.md)と中央のtyped editor areaを主要surfaceとして持たなければならない（MUST）。record data YAMLを選択した場合、中央は[Data Editor](data-editor/spec.md)を表示する。

validation diagnosticsはData Editorが定義する下部`Problems` panelへ表示できなければならず（MUST）、source diffはfile単位の独立`Diff` viewとして開けなければならない（MUST）。常設right inspectorを初期sliceの必須要件にはしない。

### GUI-SHELL-LAYOUT-002

Save、Validate、Build、Project Reload等の主要commandは、現在のselectionやcapabilityに応じて到達可能なcommand surfaceから実行できなければならない（MUST）。exact button placement、icon、spacing、themeはnormative contractとして固定しない。

## Project open / reload

### GUI-SHELL-PROJECT-001

Projectが開かれていない状態では、利用者が既存Projectを選択して開ける`Open Project` actionを提示しなければならない（MUST）。Tauri Desktopの`Open Project`はhostのnative directory pickerを起動しなければならず（MUST）、事前にpath文字列の入力を要求してはならない（MUST NOT）。picker cancelはProject state、Recent Projects、diagnostic stateを変更してはならない（MUST NOT）。frontend自身がfilesystemを探索してProject semanticsを判定してはならない（MUST NOT）。

選択されたdirectoryはshared application serviceへ渡し、[Project layout](../specs/project-layout.md)のProject identity / config semanticsに従って解決しなければならない（MUST）。Projectを開けない場合はstructured diagnosticを表示し、既存の正常なProject stateまたはProject未選択stateを破壊してはならない（MUST NOT）。

initial implicit discoveryでProjectが見つからないことは正常なProject未選択stateであり、operation failureとしてExplorerへ表示してはならない（MUST NOT）。利用者が明示選択したdirectoryを開けない場合は、原因を再確認できるpersistentなdiagnosticをProject未選択Welcome surfaceまたは保持された既存Project surfaceへ表示しなければならない（MUST）。

### GUI-SHELL-PROJECT-002

Project Reloadはshared application serviceを通じてworkspace sourceを再取得しなければならない（MUST）。dirty bufferが存在する場合はData Editorの`Save All` / `Don't Save` / `Cancel` lifecycleを先に適用し、local bufferを暗黙に破棄してはならない（MUST NOT）。

## Window lifecycle

### GUI-SHELL-LIFECYCLE-001

OSのwindow close requestは、dirty source、dirty Project Settings、Build / Publish実行中のいずれもない場合、windowを閉じなければならない（MUST）。保護対象stateがある場合はcloseを一旦停止し、既存の`Save All` / `Don't Save` / `Cancel` guardを適用しなければならない（MUST）。利用者がcloseを確定した後、host permission不足やclose eventの再入によってwindowが残ってはならない（MUST NOT）。

## Architecture boundary

### GUI-SHELL-ARCH-001

React frontendはfilesystemを直接inspect / mutateせず、YAML semanticsをparse / validateせず、domain処理のためにCLIをspawnしてはならない（MUST NOT）。Tauri commandはshared `masterdata-app` serviceを呼び、domain workを`masterdata-core`へ委譲しなければならない（MUST）。

Tauri adapterはserialization / host boundaryであり、Table、Type System、validation、source-edit、Build semanticsのauthorityになってはならない（MUST NOT）。

## Commands

### GUI-SHELL-VALIDATE-001

manual Validate actionはshared validation operationを使用しなければならない（MUST）。Data Editorのlocal-buffer automatic validationと、manual Validateがdisk上の保存済みProjectを対象とする場合は、結果がどのsnapshotを対象としているか利用者が混同しない表示にしなければならない（MUST）。

frontend独自validatorを正本として使用してはならない（MUST NOT）。

### GUI-SHELL-BUILD-001

full canonical Build actionはshared Native Application Servicesの既存Build operationを使用しなければならない（MUST）。Buildはexternal publish targetを更新してはならず（MUST NOT）、dirty bufferを暗黙Saveしてはならない（MUST NOT）。dirty中の表示とsaved-source-only behaviorはData Editor specificationに従う。

### GUI-SHELL-CAPABILITY-001

Save、Build等のaction availabilityは、workspaceのopen状態、実行中operation、Recovery Required等のcurrent application stateに従って決定しなければならない（MUST）。利用不能な操作を開始可能として表示してはならない（MUST NOT）。

Migrationの`Recovery Required`中は、そのProjectのcanonical YAML sourceを意図的に変更するGUI commandを開始してはならない（MUST NOT）。Data EditorのSave / Save All / explicit Overwrite、Source Creation、Table EditorのMigration Apply、および将来追加されるsource rename / delete / move等を含む。

この状態ではcanonical source setの整合が確定していないため、GUI shellからnormal Buildを開始可能として表示してはならない（MUST NOT）。Explorer navigation、Problems閲覧、Diff / source inspection、workspaceのre-read、recovery guidance等のread-only操作まで一律に禁止してはならない（MUST NOT）。

source mutationとnormal Buildを再度有効化してよいのは、shared application / host boundaryがactual workspace sourceを再取得し、安全なsource stateを確立した後だけである（MUST）。frontend local flagの解除だけでrecovery完了扱いしてはならない（MUST NOT）。

## Diagnostics

### GUI-SHELL-DIAG-001

GUI boundaryはstructured diagnosticのcode、kind、path、line / column、schema path、record / source provenance、suggestion、related requirement reference等、shared layerが提供するdiagnostic structureを不必要にflattenして失ってはならない（MUST NOT）。

GUIはsurfaceに応じて表示量を調整してよい（MAY）が、diagnostic identityと対象locationを失うことでProblems panelやsource navigationを不可能にしてはならない（MUST NOT）。

## Loading / error state

### GUI-SHELL-STATE-001

Project open / reload / command実行中は、operationが進行中であることを識別できなければならない（MUST）。古いProject stateを新しいProjectの確定stateとして編集可能に表示してはならない（MUST NOT）。

Project未選択時はExplorer、Problems、disabled Project command群を主surfaceとして表示せず、Open / Createと[Project Workflow](project-workflow.md)のRecent Projectsへkeyboardで進めるWelcome surfaceを表示しなければならない（MUST）。

operation failureはmodalだけに依存せず、利用者が原因とrecovery actionを確認できるpersistentまたは再確認可能なsurfaceへ残さなければならない（MUST）。

shared application boundaryがMigration commit resultとして`Recovery Required`を返した場合、GUI shellはそのProjectのsource mutation recovery-required stateとして保持しなければならない（MUST）。原因、affected file state、利用可能なrecovery informationを確認できるpersistentまたは再確認可能なsurfaceを提供し、単なるtoastだけでstateを消費してはならない（MUST NOT）。command availabilityは`GUI-SHELL-CAPABILITY-001`に従う。

Migrationのsource-set stateとrollback semanticsは[Schema Migration v1](../specs/schema-migration.md)の`MIGRATION-010`が所有する。exact recovery command、journal format、manual file recovery UI、crash / power-loss transaction保証は本shell仕様で固定しない。

## 将来のtyped editor

Explorerからfolder、Table、record data、Value Object、Enum等のsource artifactを作成し、document kindごとのtyped editorを増やせる構造を維持する。ただし各create operationと専用editor semanticsは対応するcanonical specificationが整った時点で追加する。現在のrecord editing Objectiveでは新規artifact作成はscope外である。

## 未解決事項（Open Questions）

None identified for the initial Desktop existing-record authoring shell. window layout persistence、right inspector、command palette、theme customization等は将来UXとして別途扱う。
