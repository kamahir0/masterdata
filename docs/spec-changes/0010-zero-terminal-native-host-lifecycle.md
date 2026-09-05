# 仕様変更: Native Hostのzero-terminal Connected lifecycleを定義する（Specification change）

Status: Applied

<!-- Lifecycle: Draft -> Proposed -> Approved -> Applied、またはRejected。Approved canonical
     specificationを変更する前にhuman approvalが必要である。 -->

## Affected Specifications

- [`docs/specs/runtime-hosts.md`](../specs/runtime-hosts.md)へ、既存`RUNTIME-HOST-001..013`を補完する
  `RUNTIME-HOST-014..016`を追加した。
- [`docs/product/vision.md`](../product/vision.md)へ、setup/authorization後のzero-terminal Web reconnectionを反映した。
- [`docs/rfcs/0004-web-native-host-runtime.md`](../rfcs/0004-web-native-host-runtime.md)へ、zero-terminal contractへの短い
  traceabilityを追加した。RFCのStatusは`Accepted`のままとした。

既存のCLI direct composition、Tauri/Web shared semantics、capability security、protocol negotiation、build/publish semantics、
および.NET boundaryは変更しない。canonical requirement ownerは`docs/specs/runtime-hosts.md`である。

## 根拠と分類（Source Evidence and Classification）

Human maintainerは、Native componentsがsupported environmentへinstall/setupされ、必要なpairing/authorizationが有効な
通常利用では、Web applicationからterminal操作なしにNative Hostへ接続しConnected modeへ移行できることを承認した。

この承認はzero-consentではない。初回connection、authorization expiry、またはsecurity-sensitiveなreauthorizationでは
explicit user consentを要求する。CLI executableがPATH上にあることと、Webから利用可能なNative Host lifecycleが存在する
ことは別conceptであり、browserがarbitrary executableをspawnする設計を採用しない。

承認済みdecision:

- 通常のWeb利用でterminal、manual CLI invocation、shellからのdaemon手動起動を要求しない。
- Native Hostのdetection、protocol/capability negotiation、valid prior authorization validationはautomatic pathで行える。
- 条件が成立した場合、Connected modeへautomatic transitionする。
- Native Host unavailable、incompatible、unauthorizedの場合はStandalone Webを維持し、native operationを無効化して
  recovery guidanceを提示する。
- Native Host lifecycleの具体mechanism、package shape、transport、port discovery、process managerはこのchangeで決めない。
- CLIはdirect/in-process pathを維持し、Native Host processをCLIのmandatory dependencyにしない。

これはproduct/runtime contractのdocs-only changeであり、Native Host process、installer、OS service、custom URL scheme、
localhost RPC、WASM/Web runtime、CLI/Tauri、tests/fixtures runtime behaviorは実装しない。

## 適用した差分（Applied Delta）

### RUNTIME-HOST-014 — zero-terminal Connected lifecycle

Native componentsが正常にinstall/setupされ、authorizationがvalidなsupported environmentでは、通常のWeb利用にterminal
またはmanual CLI invocationを要求しない。Native distribution/setupはWebから利用可能なNative Host lifecycle integrationを
提供するが、具体的なservice、agent、on-demand activation、helper、custom protocolの選択は固定しない。

### RUNTIME-HOST-015 — automatic discovery and negotiation

Web startup時にhostが利用可能ならdetection、protocol negotiation、capability negotiation、authorization validationを
automatic pathで試行し、compatible・authorized・capability-knownな場合はConnected modeへ遷移する。初回pairingまたは
expired authorizationではexplicit consentを維持し、Connected modeをall-capability stateとはみなさない。

### RUNTIME-HOST-016 — Standalone fallback and recovery

host unavailable、incompatible、unauthorized、detect/activation/handshake failureでもStandalone authoringをfatalにせず、
native operationを実行しない。必要なconnect/setup/update/reauthorize guidanceを提示する。CLI direct/in-process pathは
変更しない。

## 互換性（Compatibility）

このchangeは、既存のStandalone Web capability、Connected Web capability negotiation、RUNTIME-HOST-008/009のsecurity・
authorization・protocol validation、およびRUNTIME-HOST-002のCLI direct pathを強化する。通常利用時のterminal不要という
observable contractを追加するが、初回/expired authorizationのexplicit consentを弱めない。Native Hostが未実装の現在の
runtimeには、今回のdocs適用によるbehavior changeはない。

## 受け入れと実装への影響（Acceptance and Implementation Impact）

次のfuture evidenceを`runtime-hosts.md`のacceptance matrixへ追加した。すべて`pending implementation`であり、現時点で
pass済みとは扱わない。

| Requirement | Planned evidence | Fixture / observation |
| --- | --- | --- |
| RUNTIME-HOST-014 | `paired_native_host_connects_without_terminal_command`; `cli_does_not_depend_on_native_host_process` | setup/authorization済み環境の通常Web起動とCLI direct pathの分離を確認する。 |
| RUNTIME-HOST-015 | `web_start_auto_negotiates_native_host`; `first_connection_requires_explicit_authorization`; `expired_authorization_requires_reauthorization` | automatic handshakeと初回/期限切れconsentの境界を確認する。 |
| RUNTIME-HOST-016 | `native_host_unavailable_keeps_standalone_usable`; `protocol_mismatch_keeps_native_operations_disabled` | host failure/mismatch時にStandaloneが利用可能でnative operationが実行されないことを確認する。 |

## Open Questions

Human decisionにより「通常のWeb利用でterminalを要求しないか」はOpen Questionではない。以下のmechanical detailだけを残す。

- background service、background agent、on-demand activation、OS-specific startup/login integrationの選択
- installer/package manager、CLI/Desktop/native companion packageのdistribution shape
- Native Hostのtransport、port discovery、custom URL/protocol usage、process lifetime、idle shutdown、crash restart
- Native Host update mechanism、browser extensionの必要性、PWA/offlineとの関係
- pairing token format、origin allowlist、session renewalのexact security design

## レビュー（Review）

Status `Applied`。Human Approval済みのzero-terminal Connected lifecycleを`RUNTIME-HOST-014..016`としてcanonical
`runtime-hosts.md`へ適用した。`runtime-hosts.md`のStatusは`Approved`のままとし、Native Host lifecycle、handshake、Web runtimeの
implementation evidenceがないため`Implemented`へ変更していない。

このapplicationではNative Host process、installer、OS service/login item、custom URL scheme、localhost RPC、WASM/Web
runtime、CLI、Tauri、CI deployment、tests/fixtures runtime behaviorを変更していない。

## 承認記録（Approval Record）

このtask inputにおいてHuman maintainerは、次のscopeを明示的に承認した。

- Native componentsがinstall/setup済みでvalid pairing/authorizationがある通常利用では、terminal操作なしにWebから
  Native Hostへ接続できる。
- Web startup時のNative Host detection、handshake、protocol/capability negotiation、prior authorization validationを
  automatic pathとして扱い、条件成立時はConnected modeへ自動遷移する。
- 初回connection、authorization expiry、security-sensitive reauthorizationではexplicit user consentを維持する。
- host unavailable/incompatible/unauthorizedではStandalone Webを維持し、native operationを実行せずrecovery guidanceを示す。
- CLIはNative Host processに依存せず、direct/in-process Native Application Servicesを維持する。

このapprovalを根拠としてcanonical `runtime-hosts.md`へdeltaを適用し、本artifactをApplied deltaのaudit recordとした。Native
Host lifecycle implementation、handshake transport、Web runtimeは未実装である。
