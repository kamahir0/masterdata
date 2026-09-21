# プロダクトビジョン（Product vision）

Status: Draft

`masterdata`は、MasterMemoryを利用するUnity project向けのlocal-first authoring・build systemである。人間が読めるYAMLをcanonical Source of Truthとし、Gitで差分を確認・reviewできる。CLIとTauri Desktopは同じRust domain / application semanticsを利用し、MasterMemory固有のcompileとbinary buildは狭い.NET adapterへ委譲する。

clone可能なrepositoryを、人間のdeveloperとAI agentの双方が同じsource、version-controlled specification、structured diagnosticsから理解できる状態にする。generated artifactは再現可能とし、未対応featureを黙って近似しない。

## Product problem / motivation

Excelやopaque binaryを中心としたmaster-data authoringから、YAML、Git diff、review、automation、AI-assisted workflowを組み合わせた制作環境へ移行する。HumanとAIに別のhidden semanticsやauthorityを設けない。

## Product direction

CLIとDesktopの制作workflowを、共有Rust semantics、source-preserving authoring / migration、明示的なrollback / recovery、再現可能なBuild / Publishを中心に発展させる。YAMLのsource authorityとGit上のreview可能性を保ち、schema-aware editing、project layout、settings UXを改善する。具体的なpublic command、config key、file formatは各canonical specificationが所有する。

未承認のauthoring設計案は[Authoring system v1 RFC](../rfcs/0008-authoring-system-v1.md)を参照する。RFCの提案をApproved behaviorとは扱わない。

## 成功条件

- developerがUnityを開かずにprojectをdiscover、validate、authoring、Buildできる。
- CLIとDesktopが同じdomain / application semanticsとstructured diagnosticsを利用する。
- YAML fileの移動でTable / Typeのsemantic identityが変わらない。
- source-preserving editとmigrationが、競合、rollback、Recovery Requiredを明示する。
- generated artifactとMasterMemory binaryを決定的に再生成できる。
- MasterMemory internalsは.NET ecosystemへ委譲されたままである。
