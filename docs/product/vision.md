# プロダクトビジョン（Product vision）

Status: Draft

`masterdata` は、MasterMemoryを利用するUnity project向けのlocal-first authoring・build systemである。YAMLを人間が編集するSource of Truthとする。Rust application coreは、CLIとTauri desktop applicationの双方へ、project、schema、data、validationの同一semanticsを提供する。MasterMemory固有のcompileとbinary buildは、狭い.NET adapterが担当する。

clone可能なrepositoryを、人間のdeveloperとAI agentの双方が理解できる状態にする。behaviorはGitで仕様化し、generated artifactは再現可能にし、errorはstructured locationを持たせ、未対応featureは黙って近似せず明示する。

## 成功条件

- developerがUnityを開かずにprojectをdiscoverし、validateできる。
- 同じvalidation resultをCLIとGUIの双方から取得できる。
- YAML fileを分割または移動してもtable identityが変わらない。
- schema evolutionにstable IDとcompatibility checkが明示されている。
- MasterMemory internalsは.NET ecosystemへ委譲されたままである。

## 初期セットアップにおける非目標

- 完全なschema language実装
- MasterMemory Source Generatorまたはbinary formatの再実装
- production-gradeなtable editor
- code signing、notarization、またはdistribution automation

未解決のproduct questionは初期codeへ隠さず、関連するspecificationで管理する。
