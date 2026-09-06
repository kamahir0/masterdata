# プロダクトビジョン（Product vision）

Status: Draft

`masterdata` は、MasterMemoryを利用するUnity project向けのlocal-first authoring・build systemである。YAMLを人間が編集するSource of Truthとする。Rust application coreは、CLI、Tauri desktop application、Web applicationのすべてに、project、schema、data、validationの同一semanticsを提供する。MasterMemory固有のcompileとbinary buildは、狭い.NET adapterが担当する。Standalone Webはlocal workspaceのauthoring・validationを提供し、compatibleかつ明示的に許可されたlocal Native Hostへ接続した場合に限り、native build・publish capabilityを利用できる。

Web frontendはstatic hosting可能な共有frontendとし、初期deployment targetはGitHub Pagesとする。ただし通常利用にcentral application server、database、user account、remote build serviceを必須にしない。Webのhost capability、composition root、Browser/Native Host boundaryは[Runtime hosts仕様](../specs/runtime-hosts.md)で定義する。

Native componentsのsetupと初回authorizationが完了したユーザーは、通常のWeb利用でterminal操作を繰り返さずにNative Hostへ再接続し、利用可能なnative capabilityを得られる方向とする。

clone可能なrepositoryを、人間のdeveloperとAI agentの双方が理解できる状態にする。behaviorはGitで仕様化し、generated artifactは再現可能にし、errorはstructured locationを持たせ、未対応featureは黙って近似せず明示する。

## Product problem / motivation

このproductは単なるYAML parserではない。Excelやopaque binaryを中心としたmaster-data authoringから、YAML、Git diff、
review、automation、AI-assisted workflowを組み合わせたmaster-data development environmentへ移行することを、主要な
motivationの一つとする。

HumanとAIは、同じcanonical source、version-controlled specification、structured diagnostics、reproducibleなworkflowを
利用する。AI専用のhidden semanticsやAI専用のauthorityを追加せず、同じrepository artifactsを読んで、同じcontractに基づいて
変更をreviewできる状態を目指す。

## Product direction

長期的には、YAMLをcanonical Source of Truthとして保ちつつ、Desktop、Web、CLIでsemantic coreとapplication semanticsを
できるだけ共有する。Web対応のためにCLIをNative Host RPC-onlyへ統一せず、Standalone Web、Connected Web、Native Hostを
それぞれのcapability境界として構成する。table / column-oriented GUI、schema-aware editing、source-preserving Migration、
project layoutとsettings UXもproductとして整理するが、具体的なpublic command、config key、protocol、file formatは各owner
specificationで承認されるまで固定しない。

将来候補として、Generated C# Preview、explicit C# export、read-only binary inspect/query、formatter、SQL-like UXを検討し得る。
これらはProduct Direction上のcandidateであり、現時点のApproved RequirementやCLI grammarではない。

## 成功条件

- developerがUnityを開かずにprojectをdiscoverし、validateできる。
- 同じvalidation resultをCLIとGUIの双方から取得できる。
- Standalone Webからも共有semanticsによるauthoring・validationを利用でき、native capabilityが必要なoperationは対応するNative Hostへ明示的に接続して実行できる。
- Native Host-enabled environmentでは、valid authorizationの範囲でWeb起動時のdetection・handshake・capability negotiationからConnected modeへ移行できる。
- YAML fileを分割または移動してもtable identityが変わらない。
- schema evolutionにstable IDとcompatibility checkが明示されている。
- MasterMemory internalsは.NET ecosystemへ委譲されたままである。

## 初期セットアップにおける非目標

- 完全なschema language実装
- MasterMemory Source Generatorまたはbinary formatの再実装
- production-gradeなtable editor
- code signing、notarization、またはdistribution automation

未解決のproduct questionは初期codeへ隠さず、関連するspecificationで管理する。
