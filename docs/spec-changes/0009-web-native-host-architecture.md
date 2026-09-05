# 仕様変更: Web / Native Host capability architectureを記録する（Specification change）

Status: Applied

<!-- Lifecycle: Draft -> Proposed -> Approved -> Applied、またはRejected。Approved canonical
     specificationを変更する前にhuman approvalが必要である。 -->

## Affected Specifications

- 新しいcanonical specificationとして[`docs/specs/runtime-hosts.md`](../specs/runtime-hosts.md)を追加し、
  `RUNTIME-HOST-001`から`RUNTIME-HOST-013`を定義した。
- [`docs/product/vision.md`](../product/vision.md)へ、Web、Standalone/Connected mode、Native Host、local-first/static
  hostingのproduct directionを反映した。visionのStatusは`Draft`のままとした。
- [`docs/specs/README.md`](../specs/README.md)へRuntime hosts specificationを追加した。
- [`docs/rfcs/0004-web-native-host-runtime.md`](../rfcs/0004-web-native-host-runtime.md)を`Accepted`として、alternativeと
  trade-offを記録した。
- [`docs/adr/0006-host-capability-composition.md`](../adr/0006-host-capability-composition.md)を`Accepted`として、既存
  ADR 0002/0003を維持したhost boundary decisionを記録した。

既存の`BUILD-ARTIFACT-*`、`ARTIFACT-SET-*`、`PUBLISH-*`、`PUBLISH-PATH-*`、`PUBLISH-EXEC-*`、project layout semanticsは
変更しない。canonical requirement ownerは各既存documentのままである。

## 根拠と分類（Source Evidence and Classification）

このchangeは、Human maintainerが承認したWeb product host、local-first/static hosting、Standalone/Connected Web、Native
Host capability、CLI direct composition、shared frontend/domain、.NET boundary維持を記録するarchitecture changeである。
会話上の承認を、RFCのalternative/rationale、ADRのarchitecture decision、canonical specificationのobservable behaviorへ
分離して永続化する。

承認済みの中心decisionは次のとおりである。

- Web applicationは正式なproduct hostであり、Standalone Webはlocal-first authoring/validationを提供する。
- Connected Webはcompatibleかつexplicitly authorizedなNative Hostのcapabilityを利用できる。
- CLIはNative Application Servicesをdirect/in-processで利用し、Web対応のためのlocalhost RPCを必須にしない。
- CLI、Desktop、Connected Webは同じNative application/build semanticsを利用し、domain logicを複製しない。
- Browser workspaceとNative workspaceはhost boundaryで分離し、explicit authorizationとworkspace/capability scopeを要求する。
- MasterMemory/.NET delegationを維持し、browserでbinary formatまたはfull builderを再実装しない。
- static Web hostingはremote backend/database/accountを必須にせず、初期deployment targetはGitHub Pagesとする。

## 適用した差分（Applied Delta）

1. `runtime-hosts.md`に、composition root、Native/Browser Host、Standalone/Connected capability、security boundary、
   protocol negotiation、workspace URL、static hosting、pure core boundaryを`RUNTIME-HOST-001..013`として追加した。
2. RFC 0004でOption A/B/Cと採用Option Dを比較し、browser .NET、central build service、CLI RPCなどをrejected alternativeとして
   記録した。
3. ADR 0006でADR 0002のshared Rust core決定をWeb/Native/Browser hostへ一般化し、ADR 0003の.NET boundaryを維持した。
4. Product visionとspec indexへarchitectureの存在と未実装状態を追記した。

## 互換性（Compatibility）

このchangeは既存CLI、Tauri、project-layout、build pipeline、canonical artifact receipt、external publish path/execution
semanticsのruntime behaviorを変更しない。Web runtime、WASM adapter、Native Host process、loopback transport、Tauri/Web
publish UI、browser filesystem adapterは未実装である。Connected Webは将来、既存Native application servicesを呼ぶadapterとして
実装する。

## 受け入れと実装への影響（Acceptance and Implementation Impact）

新canonical specificationのfuture evidenceは`runtime-hosts.md`のacceptance matrixに記載した。以下はすべて
`pending implementation`であり、今回pass済みとは扱わない。

- CLI direct pathとDesktop/CLI shared semantics
- Standalone Webのshared validationとnative build capability不在
- Connected Webのbuild capability negotiationとCLIと同じNative build service利用
- Native Hostのunpaired request拒否、workspace scoping、protocol mismatch時のoperation未実行
- workspace URLがabsolute filesystem pathを含まないこと
- shared domainがloopback transportやremote backendを必須にしないこと

## Open Questions

次のdetailはarchitecture acceptance後も未確定である。

- browser support matrix、File System Access API fallback、IndexedDB等のexact persistence
- Native Host transport、port discovery、pairing token、origin allowlist、protocol compatibility window
- installer/service lifecycle、auto-start、update、dirty-buffer handoff、mode transition UX
- GitHub Pages custom domain、PWA/offline、service worker/cache、browser filesystem edge cases

これらは`runtime-hosts.md`のOpen Questionsが所有し、今回のcanonical architecture approvalを阻害しない。

## レビュー（Review）

Status `Applied`。Human Approval済みのWeb / Native Host capability architectureを、RFC、ADR、canonical specification、
product vision、spec indexへ分離して記録した。canonical `runtime-hosts.md`は`Approved`だが、runtime implementation evidenceが
ないため`Implemented`へ変更していない。既存のbuild/publish canonical specificationへ未承認のsemantic deltaは適用していない。

## 承認記録（Approval Record）

このtask inputにおいてHuman maintainerは、次のscopeを明示的に承認した。

- Web applicationを正式なproduct hostとし、Standalone Webをlocal-first authoring/validation environmentとする。
- compatibleかつexplicitly authorizedなlocal Native HostによりConnected Webがnative build/publish capabilityを利用できる。
- CLIはNative application servicesをdirect/in-processで利用し、Web対応のためにlocalhost RPCを必須にしない。
- CLI、Tauri、Connected Webは同じNative application/build semanticsを利用し、shared frontend/domainを再利用する。
- Browser HostとNative Hostのcapability/security/workspace boundary、protocol negotiation、workspace URL local bookmark semanticsを採用する。
- MasterMemory/.NET boundaryを維持し、central server/databaseとbrowser full buildを必須にしない。

このapprovalを根拠として、`RUNTIME-HOST-001..013`をcanonical `Approved` specificationとして追加し、本artifactをApplied
deltaのaudit recordとした。Web runtime、WASM host adapter、Native Host、Connected Web build/publishは未実装である。
