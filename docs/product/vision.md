# プロダクトビジョン（Product vision）

Status: Draft

`masterdata` は、MasterMemoryを利用するUnity project向けのlocal-first authoring・build systemである。YAMLを人間が編集するSource of Truthとする。Rust application coreは、CLI、Tauri desktop application、Web applicationのすべてに、project、schema、data、validationの同一semanticsを提供する。MasterMemory固有のcompileとbinary buildは、狭い.NET adapterが担当する。Standalone Webはlocal workspaceのauthoring・validationを提供し、compatibleかつ明示的に許可されたlocal Native Hostへ接続した場合に限り、native build・publish capabilityを利用できる。

Web frontendはstatic hosting可能な共有frontendとし、初期deployment targetはGitHub Pagesとする。ただし通常利用にcentral application server、database、user account、remote build serviceを必須にしない。Webのhost capability、composition root、Browser/Native Host boundaryは[Runtime hosts仕様](../specs/runtime-hosts.md)で定義する。

Native componentsのsetupと初回authorizationが完了したユーザーは、通常のWeb利用でterminal操作を繰り返さずにNative Hostへ再接続し、利用可能なnative capabilityを得られる方向とする。

clone可能なrepositoryを、人間のdeveloperとAI agentの双方が理解できる状態にする。behaviorはGitで仕様化し、generated artifactは再現可能にし、errorはstructured locationを持たせ、未対応featureは黙って近似せず明示する。

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
