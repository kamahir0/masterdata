# Current Objective

## Role

この文書はHuman-selectedなcurrent priorityとwork package boundaryの唯一のownerである。
Approved behaviorは[仕様index](specs/README.md)の各canonical specification、現在のStageは
[Development State](execution-state.md)、implementation realityはcode / tests / Gitで確認する。
この文書は未承認のproduct behaviorのimplementation authorityではない。

## Objective

現在のHuman priorityは、**GUIのWorkspace Explorerから新しいsource artifactを安全に作成し、YAMLを手書きせずにProjectを組み立て始められる体験を完成させる**ことである。

既存record authoring Objectiveはcandidate `44a840c343bd0c560cf1963bcdefb31d4a20a54d`のfinal verificationでBlockingなしとなり、2026-09-11に`objective-complete`へ到達した。その後Humanが「次に進む」と指示したため、直前に第一候補として提示していたExplorerからのsource artifact creationを次priorityとして選択する。

初期creation sliceでは、Workspace上のfolderと、現在Approvedなdomain contractで完全な初期形を定義できる次のsource artifactを対象とする。

- Table schema document
- record Data document
- Value Object
- Normal Enum / Flags Enum
- Custom Type

Table作成は名前だけの不完全なschema fileを置くoperationにはせず、field declarationとPrimary Keyを含むvalidな初期schemaをguided inputから作成する。Data documentは既存Tableを選択してempty `records`から開始する。Value Object / Enum / Flags / Custom Typeは各Approved Type System仕様に従うcomplete declarationを作成する。

Humanが既に選択した方針どおり、使い勝手・データ安全性・互換性を大きく左右しないroutine interaction detailは既存UI方針、platform convention、accessibility、testabilityに従って仕様側で決定し、実使用後に必要なら調整する。

## Why now

現在のGUIは既存Data documentをExplorerから開き、編集・validation・Diff・file Save・external conflict recoveryまで実行できる。一方で、新しいTable、Data file、type declarationをGUIから作れないため、新規Projectや新しいmaster-data領域を始めるには依然としてYAMLの手書きが必要である。

Product VisionはYAMLをcanonical Source of Truthに保ちながらtable / column-oriented GUIとschema-aware authoringを提供する方向を持つ。既存record編集の安全なsource write基盤が整った今、次にcreation boundaryをshared application semanticsへ接続することで、既存file編集からProject authoringへ体験を拡張する。

## Completion boundary

- Workspace Explorerのsource root / folderから、keyboardでも到達可能な`New` actionを実行できるようにする。
- configured source root内に新しいfolderを作成できるようにする。folder pathをTable / type等のdomain identityとして扱わない。
- Table schema作成では、Table identity、optional `csharpName`、1個以上のfield、MessagePack `key`、field type / modifier、exactly one Primary Key、およびoptional Secondary Keyをguided inputで指定し、Approved Table / Key / Type System仕様に従うvalidな初期schema documentを作成できるようにする。
- Data document作成では既存Table identityとdestinationを選び、`kind: data` / `table` / empty `records`を持つvalidな初期documentを作成できるようにする。
- Value Object作成ではname、key-compatible primitive underlying、およびsupported conversion optionを指定してvalidなtype documentを作成できるようにする。
- Normal Enum / Flags Enum作成ではname、underlying、memberを指定し、各Approved Enum / Flags ruleを満たすvalidなtype documentを作成できるようにする。
- Custom Type作成ではnameと1個以上のfieldを指定し、Approved Custom Type / Field Modifier / MessagePack key ruleを満たすvalidなtype documentを作成できるようにする。
- YAML rendering、document semantics、name/type/key validation、identity collision判定をfrontendへ複製せず、shared core/application boundaryへ委譲する。
- source fileは`.yaml`または`.yml`としてconfigured source root内へ作成し、path traversal / unsafe symlink等でworkspace外を書き換えない。
- target file / folderが既に存在する場合は上書きせず、creation conflictとして入力を保持したままrecoveryできるようにする。
- ordinary create failureで既存sourceを変更せず、partialなdestinationを成功扱いしない。write outcomeを確定できない場合は自動retry / overwriteせず、workspaceを再確認してから次のmutationへ進む。
- Create成功後はExplorerを更新し、作成itemを選択・表示する。既存dirty bufferを暗黙Saveまたは破棄しない。
- workspace write capabilityがないhostではCreate actionを実行可能として扱わない。
- focused domain/application tests、React操作test、repository required checks、final verificationでBlockingがないことを確認する。

## Explicit non-scope

現Objectiveでは次を含めない。

- 既存source file / folderのrename、delete、move、duplicate。
- Data documentへのrecord追加・削除。新規Data documentはempty `records`から開始する。
- 作成後のTable schema / Value Object / Enum / Flags / Custom Typeを編集する専用typed editor。初期definitionはcreation flowで確定する。
- Table schemaとData document等、複数artifactを1回のtransactionで同時作成するwizard。
- Reference declarationなど、canonical ownerがまだDraftのdomain featureをcreation formで先取りすること。
- template marketplace、import / copy from external file、Git stage / commit / push。
- Standalone / Connected Web全体、Native Host lifecycle、distributionの同時完成。
- Programmable View / Computed Column / Annotation Column、Table横断view、高度なspreadsheet操作。

## Next candidate

このObjective完了後は、Data Editorのrecord追加・削除、Table / Type専用editor、rename / delete / move、spreadsheet操作拡張、Programmable View、Build Profile / Publish、Standalone / Connected Webへのauthoring surface展開を候補として比較する。

次priorityは自動昇格せず、current realityとproduct valueを確認してHumanが選択する。

## Relevant authorities

- [Product vision](product/vision.md)
- [GUI specification index](gui/README.md)
- [GUI app shell](gui/app-shell.md)
- [Workspace Explorer](gui/explorer/spec.md)
- [Project layout and discovery](specs/project-layout.md)
- [Masterdata YAML subset](specs/yaml-subset.md)
- [Table / Primary Key / Secondary Key](specs/table-and-keys.md)
- [Primitive Types](specs/type-system/primitives.md)
- [Field Modifiers](specs/type-system/field-modifiers.md)
- [Value Objects](specs/type-system/value-objects.md)
- [Enum / Flags](specs/type-system/enums.md)
- [Custom Types](specs/type-system/custom-types.md)
- [Runtime hosts / capability](specs/runtime-hosts.md)
- [Source Record Edit](specs/source-edit.md) — source write safetyの既存evidence。creation operationのauthorityではない。
- [YAML Source of Truth ADR](adr/0001-yaml-is-source-of-truth.md)
- [shared core ADR](adr/0002-rust-core-shared-by-cli-and-gui.md)
- [host capability ADR](adr/0006-host-capability-composition.md)
- [Specification workflow](contributing/specification-workflow.md)
- [Development workflow](execution-workflow.md)
