# プロダクトビジョン（Product vision）

Status: Draft

`masterdata`は、MasterMemoryを利用するUnity project向けのlocal-first authoring・build systemである。人間が読めるYAMLをcanonical Source of Truthとし、Gitで差分を確認・reviewできる。CLIとTauri Desktopは同じRust domain / application semanticsを利用し、MasterMemory固有のcompileとbinary buildは狭い.NET adapterへ委譲する。

clone可能なrepositoryを、人間のdeveloperとAI agentの双方が同じsource、version-controlled specification、structured diagnosticsから理解できる状態にする。generated artifactは再現可能とし、未対応featureを黙って近似しない。

## Product problem / motivation

Excelやopaque binaryを中心としたmaster-data authoringから、YAML、Git diff、review、automation、AI-assisted workflowを組み合わせた制作環境へ移行する。HumanとAIに別のhidden semanticsやauthorityを設けない。

## Product direction

CLIとDesktopの制作workflowを、共有Rust semantics、source-preserving authoring / migration、明示的なrollback / recovery、再現可能なBuild / Publishを中心に発展させる。YAMLのsource authorityとGit上のreview可能性を保ち、schema-aware editing、project layout、settings UXを改善する。具体的なpublic command、config key、file formatは各canonical specificationが所有する。

未承認のauthoring設計案は[Authoring system v1 RFC](../rfcs/0008-authoring-system-v1.md)を参照する。RFCの提案をApproved behaviorとは扱わない。

## Scope discipline / 何を作らないか

`masterdata`のcoreへ入れるのは、MasterDataを定義・編集・検証・安全にmigrationし、再現可能なartifactをBuild / PublishしてUnityへ届ける中心workflowを成立させる責務をdefaultとする。「あると便利」「将来使えそう」だけではcore ownershipの理由にしない。

Git hosting / collaboration、AI支援、issue tracker、通知、provider固有workflow等の外部concernは、具体的なHuman-selected requirementがない限りcoreへ取り込まない。必要になった場合も、まずYAML/files、CLI、既存の明示boundaryを使って外側からcompositionし、依存方向を **integration / extension → masterdata** に保つことを優先する。masterdata coreが外部providerやcollaboration productのsemanticsへ依存する形をdefaultにしない。

この原則はplugin / extension frameworkを先回りして実装する要求ではない。extension APIやplugin architecture自体も、具体的で反復する需要とHuman-selected Objectiveが現れるまで作らない。

RFC、Deferred、future direction、Ideaは設計上の可能性を保存するだけで、roadmap、priority、次Objectiveの予約を意味しない。必要性が具体化した時点で改めてHumanがpriorityを選択する。

## 成功条件

- developerがUnityを開かずにprojectをdiscover、validate、authoring、Buildできる。
- CLIとDesktopが同じdomain / application semanticsとstructured diagnosticsを利用する。
- YAML fileの移動でTable / Typeのsemantic identityが変わらない。
- source-preserving editとmigrationが、競合、rollback、Recovery Requiredを明示する。
- generated artifactとMasterMemory binaryを決定的に再生成できる。
- MasterMemory internalsは.NET ecosystemへ委譲されたままである。
