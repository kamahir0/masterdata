# Build Request / Publish Preview仕様

Status: Approved

Domain: Build

この仕様はDesktop GUIからshared applicationへ渡すBuild request captureと、Publish実行前のread-only previewを定義する。artifact / receipt / publish execution semanticsは[Build pipeline](build-pipeline.md)、Profile semanticsは[Build Selection](build-selection.md)が所有する。適用記録は[仕様変更0018](../spec-changes/0018-desktop-build-delivery.md)を参照する。

## 規範要件

### BUILD-REQUEST-001

GUIのBuild requestはProject bindingと、unfilteredまたは明示named Profileをshared applicationへ渡さなければならない（MUST）。named Profile解決、source parse、validation、selection、Buildは既存ownerを使用し、frontend独自selectorやCLI subprocessを使用してはならない（MUST NOT）。
開始時に取得した保存済みconfig/source snapshotからresolved BuildPlanを構成し、そこから同じartifact setを生成する。読込中の変更が検出された場合はsnapshotを確定できないfailureとしてartifact publication前に停止する（MUST）。Plan確定後のsource変更はそのBuild inputへ混ぜず、operation結果はcaptured inputの結果として扱う。全filesystemに対するglobal lockを約束する意味ではない。
dirty YAML/config、Tag draft、View filter、clipboard previewを入力へ暗黙に含めてはならない（MUST NOT）。captured profileとconfig/inputの識別情報をsession resultへ残すがreceipt v1へnew required fieldを追加しない。

### PUBLISH-PREVIEW-001

GUI向けread-only Publish previewはshared receipt validation / all-target preflightを使用し、artifact-set identity、config identity、configured target kind/path、resolved destination、preflight結果を返さなければならない（MUST）。成功時は既存manifestに基づくC# addition/update/removalとbinary replacementの予定を示す。frontendがmanifestやfilesystem ownershipを再解釈してはならない（MUST NOT）。
preflight failureで全部の詳細を計算できなければ全targetをnot_attemptedとし、検査できなかった項目を未確認と表示する。最初のerrorだけ取得できるserviceでも未取得項目を成功扱いしない（MUST NOT）。previewはtarget parent作成を含むmutationを行わない。

### PUBLISH-PREVIEW-002

GUIのConfirm Publishは表示したpreviewと同じartifact setとconfigを対象にし、実行直前にidentityを再確認しなければならない（MUST）。変更時は無mutationでstale previewとして再確認へ戻す。
destination / manifest / unmanaged contentは毎回既存path-safety / all-target preflightで再検査する。表示した管理file集合やreplacement/removal計画が変われば古いpreviewでmutationせず再確認する（MUST）。mtimeだけをauthorityにせず、affected content/ownershipのexact identityを用いる。
全target実行を開始した後のfailureとcontinuationは既存PUBLISH-EXEC contractに従い、preview機能がcross-target rollbackやglobal atomicityを追加してはならない（MUST NOT）。execution-time recheckも既存path-safetyに従う。

## 受け入れ証拠

Build requestはprofile-separate同PK、unknown profile、dirty YAML/config除外、読込中source変更、captured後変更を混ぜないことを検証する。Publish previewはartifact/config/manifest変更で無mutation、unmanaged collision、対象集合の変更、preview自体無mutationを検証する。
