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

Projectが開かれていない状態では、利用者が既存Projectを選択して開ける`Open Project` actionを提示しなければならない（MUST）。Tauri Desktopではhostのnative folder selectionを利用してよい（MAY）が、frontend自身がfilesystemを探索してProject semanticsを判定してはならない（MUST NOT）。

選択されたdirectoryはshared application serviceへ渡し、[Project layout](../specs/project-layout.md)のProject identity / config semanticsに従って解決しなければならない（MUST）。Projectを開けない場合はstructured diagnosticを表示し、既存の正常なProject stateまたはProject未選択stateを破壊してはならない（MUST NOT）。

### GUI-SHELL-PROJECT-002

Project Reloadはshared application serviceを通じてworkspace sourceを再取得しなければならない（MUST）。dirty bufferが存在する場合はData Editorの`Save All` / `Don't Save` / `Cancel` lifecycleを先に適用し、local bufferを暗黙に破棄してはならない（MUST NOT）。

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

Save、Build等のhost-dependent action availabilityは、[Runtime hosts](../specs/runtime-hosts.md)のgranted capabilityに従って決定しなければならない（MUST）。Tauri Desktopであることだけを理由に、利用不能なcapabilityを常に利用可能として表示してはならない（MUST NOT）。

## Diagnostics

### GUI-SHELL-DIAG-001

GUI boundaryはstructured diagnosticのcode、kind、path、line / column、schema path、record / source provenance、suggestion、related requirement reference等、shared layerが提供するdiagnostic structureを不必要にflattenして失ってはならない（MUST NOT）。

GUIはsurfaceに応じて表示量を調整してよい（MAY）が、diagnostic identityと対象locationを失うことでProblems panelやsource navigationを不可能にしてはならない（MUST NOT）。

## Loading / error state

### GUI-SHELL-STATE-001

Project open / reload / command実行中は、operationが進行中であることを識別できなければならない（MUST）。古いProject stateを新しいProjectの確定stateとして編集可能に表示してはならない（MUST NOT）。

operation failureはmodalだけに依存せず、利用者が原因とrecovery actionを確認できるpersistentまたは再確認可能なsurfaceへ残さなければならない（MUST）。

## 将来のtyped editor

Explorerからfolder、Table、record data、Value Object、Enum等のsource artifactを作成し、document kindごとのtyped editorを増やせる構造を維持する。ただし各create operationと専用editor semanticsは対応するcanonical specificationが整った時点で追加する。現在のrecord editing Objectiveでは新規artifact作成はscope外である。

## 未解決事項（Open Questions）

None identified for the initial Desktop existing-record authoring shell. Recent Project一覧、window layout persistence、right inspector、command palette、theme customization等は将来UXとして別途扱う。
