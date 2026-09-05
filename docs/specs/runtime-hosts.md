# Runtime hosts / composition root / capability仕様

Status: Approved

この仕様は、CLI、Tauri Desktop、Standalone Web、Connected Web、およびそれらが利用する
Native Host / Browser Hostのruntime boundaryを定義する。これは
[Web / Native Host runtime RFC](../rfcs/0004-web-native-host-runtime.md)、
[Host capability composition ADR](../adr/0006-host-capability-composition.md)、および
[specification change 0009](../spec-changes/0009-web-native-host-architecture.md)と
[zero-terminal lifecycle change 0010](../spec-changes/0010-zero-terminal-native-host-lifecycle.md)を根拠とする。

このdocumentのnormative requirementは、Web runtime、WASM adapter、Native Host、loopback transport、
またはpublish runtimeの実装を意味しない。既存のbuild/publish semanticsは
[Build pipeline仕様](build-pipeline.md)が所有し、Connected WebはそのNative application servicesを
adapter経由で利用する。

## Runtime model

MasterDataは、YAMLをsource of truthとするlocal-first productである。Web frontendはstatic hosting
可能な共有frontendとして提供され、user dataとcomputeは原則としてuserのlocal environmentに残る。

```text
                         Shared Frontend
                               |
                         Host Interface
                    /            |             \
             Browser Host      Tauri       Native/loopback adapter
                  |              |                |
          Standalone Web   Native services   Connected Web
                               ^
                         direct in-process
                               |
                              CLI
```

### 用語

- **Shared Domain / Application**: YAML semantics、typed document、Type System、Table resolution、
  Build Selection、validation、canonical ordering、platform-independentなC# generation、および
  structured diagnosticsを担当する共有Rust logic。
- **Native Application Services**: native filesystem、project discovery、native path safety、
  system dotnet、MasterMemory build、canonical artifact publication、external publishなどの
  use-caseを、既存のapplication/domain semanticsに従って実行するservice群。
- **Native Host**: local machine上でNative Application Servicesとnative capabilityを提供するhost。
  Web専用の別backendではない。
- **Browser Host**: explicit permissionを受けたbrowser workspaceとlogical project-relative pathを
  Shared Applicationへ提供するadapter。
- **Standalone Web**: Native Hostへ接続していないWeb runtime。authoringとvalidationを提供し、
  native build/publish capabilityは持たない。
- **Connected Web**: compatibleかつauthorizedなNative Hostへ接続し、negotiated capabilityを利用する
  Web runtime。
- **Composition root**: host固有のadapter、service、permission、transportを組み立てて共有application
  serviceへ注入するentrypoint。CLI、Desktop、Webは別々のcomposition rootを持つ。

## Normative Requirements

### RUNTIME-HOST-001

CLI、Tauri Desktop、Standalone Web、Connected Webは、同じdomain/application semanticsを共有しなければ
ならない（MUST）。YAML semantics、validation、Type System、Table resolution、Build Selectionなどを
hostごとに再実装してはならない（MUST NOT）。editor/domain UIの意味を持つ共有frontendは同じcodebaseを
利用すべきであり（SHOULD）、entrypoint、transport adapter、permission UIなどhost固有部分は分離してよい
（MAY）。これは全source codeを100%共有することを要求しない。

### RUNTIME-HOST-002

CLIは、Native Application Servicesをin-processの直接呼び出しで利用しなければならない（MUST）。Web対応を
理由として、CLIにlocalhost RPC、HTTP/WebSocket round-trip、browser transport、または不要なserialization
を必須にしてはならない（MUST NOT）。CLI単体のdirect pathと性能特性は、Webのtransport boundaryから独立して
維持する。

### RUNTIME-HOST-003

Standalone WebはBrowser Hostを通じて、userが明示的に許可したworkspaceのopen、source read、source edit、
source write、YAML parse、semantic validation、Type System、Table validation、Build Selection semantics
（適用可能な範囲）、およびdiagnosticsを提供できなければならない（MUST）。

Standalone WebはNative Host capabilityなしに、canonical MasterMemory build、system dotnet invocation、
canonical artifact publication、またはexternal publishを実行してはならない（MUST NOT）。native capabilityが
存在しない場合、UI/application contractは機能が利用不可であることをcapabilityとして判定できなければ
ならず、unsupportedなbuildを実行してからerrorにする設計を必須にしてはならない（MUST NOT）。

### RUNTIME-HOST-004

Webがcompatibleかつexplicitly authorizedなNative Hostへ接続したConnected modeでは、Native Hostがadvertise
し、かつsessionでgrantしたcapabilityの範囲でworkspace access、validation、build、publish、native
filesystem semantics、system dotnetを利用できる（MAY）。Connected WebはNative capabilityを暗黙に仮定して
はならず、capabilityがないoperationを実行してはならない（MUST NOT）。

### RUNTIME-HOST-005

build、publish、workspace read/writeなどのfeature availabilityは、OSやhost名だけではなく、negotiatedかつ
granted capabilityによって決定しなければならない（MUST）。概念上のcapabilityには
`workspace.read`、`workspace.write`、`validate`、`build`、`publish`を含めてよいが、exact API spellingは
この仕様で固定しない。`web`だから常にbuild不可、または`desktop`だから常にbuild可というplatform-only
分岐をcanonical contractにしてはならない（MUST NOT）。

### RUNTIME-HOST-006

CLI、Tauri Desktop、Connected Webは、Native filesystem、project loading、path safety、system dotnet、
canonical artifact build/publication、external publishなどのNative Application Servicesを共有しなければ
ならない（MUST）。Tauri adapterまたはloopback adapterにbuild semanticsを複製してはならず（MUST NOT）、
Connected WebをCLIとは異なるbuild pipelineの実装として扱ってはならない（MUST NOT）。adapterはuse-caseを
呼び出すboundaryであり、shared application semanticsのauthorityではない。

### RUNTIME-HOST-007

Browser Hostは、userのexplicit permissionを得たworkspaceだけをapplicationへ提供しなければならない
（MUST）。workspace外のfileを暗黙にread/writeしてはならず（MUST NOT）、local absolute filesystem pathを
Webのauthorityまたはpermission tokenとして扱ってはならない（MUST NOT）。Browser Hostはlogical
project-relative pathとpermission-granted workspace boundaryを提供する。File System Access API、IndexedDB、
またはそれらのfallbackの選択はこの仕様で固定しない。

### RUNTIME-HOST-008

Native Hostは、少なくとも次のsecurity boundaryを持たなければならない（MUST）。

- local/loopbackに限定したexposure
- explicit user authorizationまたはpairing
- request originの認識と制限
- session-scoped authorization
- capability-scoped grant
- workspace-scoped authority
- protocol compatibility validation

任意のpublic Web originが任意のOS absolute pathを渡すだけで、local filesystem read/write、build、または
publish authorityを得られるprotocolを設計してはならない（MUST NOT）。Native Host APIはRust内部functionの
1:1 mirrorではなく、workspace open、snapshot、source change、validate、build、publishなどのuse-case
coarse-grained boundaryを優先しなければならない（MUST）。token format、pairing UX、origin allowlist、
transport mechanicsの詳細は後続security designで定める。

### RUNTIME-HOST-009

Connected Webは、Native Hostとの接続前またはoperation開始前に、少なくともprotocol version、engine/tool
version information、およびcapabilitiesをnegotiationできなければならない（MUST）。互換性を確認できない
Native Hostへcommandを送ってはならず（MUST NOT）、Standalone modeへのfallbackまたはNative Host update
guidanceを可能にする。wire JSON shape、transport protocol、backward compatibility windowはこの仕様で
固定しない。

### RUNTIME-HOST-010

Native full buildは、[ADR 0003](../adr/0003-dotnet-mastermemory-bridge.md)の.NET delegation boundaryを
使用しなければならない（MUST）。Web対応を理由にbrowserへRoslyn、system dotnet、MasterMemory builderを
bundleしてはならず（MUST NOT）、raw YAMLをbrowser独自のbuilderへ渡してはならず（MUST NOT）、MasterMemory binary
formatをRustまたはbrowser側で再実装してはならない（MUST NOT）。Standalone Webはbinary buildを行わず、Connected Webの
full buildはNative Hostが既存Native pipelineとsystem dotnetを実行する。

### RUNTIME-HOST-011

workspace URLは、同じmachine/browser profileまたはNative registrationでworkspaceを再openするためのlocal
bookmark/registration semanticsとして扱わなければならない（MUST）。URLへOS absolute filesystem pathを埋め
込んだり、それ自体をpermission tokenとして公開してはならない（MUST NOT）。registrationまたはpermissionが
失効した場合はreconnectまたはreauthorizeを要求し、別machine、別user、または別browser profileでlocal
identityが存在しない場合はworkspaceの再選択を可能にする。cloud project share URLやcross-device identityは
この仕様の意味に含めない。

### RUNTIME-HOST-012

Web frontendは、central application server、database、user account、remote build serviceへの必須依存なしに
static hostingできなければならない（MUST）。初期deployment targetはGitHub Pagesとするが、GitHub Pages固有の
制約をdomain semanticsへ埋め込んではならず（MUST NOT）、将来別のCDNまたはstatic hostへ移行できる構造を
維持する。Standalone Webのuser dataとcomputeはlocal-firstであり、cloud workspace sync、multi-user
collaboration、remote buildはこのcontractの必須機能ではない。

### RUNTIME-HOST-013

host boundaryはfilesystem I/O、workspace acquisition、native filesystem identity、process execution、
system dotnet、artifact/external publication、browser persistence、browser permission、およびtransportなど
host固有の責務に限定すべきである（SHOULD）。YAML parser、typed document、resolver、validator、diagnostics
などのpure/domain logicへ、platform、transport、または不要なasync semanticsを漏らしてはならない（MUST NOT）。
asyncなHost I/Oはbytesまたはlogical documentsへ変換してからsynchronousなpure coreへ渡し、結果を必要に
応じてasync Host I/Oへ戻す境界を基本とする。Web対応だけを理由に全coreをtrait化またはasync化しない。

### RUNTIME-HOST-014

Native componentsがsupported environmentへ正常にinstall/setupされ、必要なpairingまたはauthorizationが有効な場合、
通常のWeb利用でConnected modeを利用可能にするためにterminal操作、manual CLI invocation、またはshellからのNative Host
手動起動を要求してはならない（MUST NOT）。Native distribution/setupは、Webから利用可能なNative Host lifecycle
integrationを提供しなければならない（MUST）。これはbrowserが任意のinstalled executableをspawnできることを意味せず、
login/startup service、background agent、on-demand activation、desktop helper、custom protocol等のexact mechanismは
この仕様で固定しない。

初回setup時に必要な明示的なauthorizationを省略してはならない。zero-terminalはzero-consentを意味せず、後続の
reauthorization requirementもこの要件によって弱められない。

### RUNTIME-HOST-015

Web起動時にNative Hostが利用可能な場合、Web applicationはterminal操作なしにhost detection、protocol negotiation、
capability negotiation、および既存authorizationのvalidationを自動的に試行できなければならない（MUST）。

次の条件がすべて成立した場合、Web applicationはConnected modeへ自動遷移しなければならない（MUST）。

- compatibleなNative Hostがdiscoveredである。
- protocol compatibilityが確認されている。
- required authorizationがvalidである。
- advertised/granted capabilitiesが既知である。

初回pairing、authorization expiry、またはsecurity-sensitiveなreauthorizationが必要な場合、Web applicationは
explicit user consentを要求しなければならず（MUST）、無効なauthorizationを自動的に再利用してはならない（MUST NOT）。
Connected modeは全capabilityを無条件に有効化する状態ではなく、build、publish、workspace read/writeなどのoperationは
既存のcapability contractに従う。

### RUNTIME-HOST-016

Native Hostがunavailable、incompatible、またはunauthorizedの場合、Web applicationはStandalone modeを維持または
fallbackできなければならない（MUST）。その状態ではnative operationを実行せず、利用可能なauthoring/validationを継続し、
必要に応じてconnect、setup、update、またはreauthorizeのactionable guidanceを提示する。

Native Hostのdetect、activation、またはhandshake failureだけを理由に、Web pageのstartupまたはStandalone authoringを
fatal failureにしてはならない（MUST NOT）。protocol mismatchの場合もnative commandを送らず、Standalone fallbackまたは
update guidanceを維持する。Native Host lifecycle failureはCLIのdirect/in-process compositionを変更せず、CLI使用時に
RPC、daemon round-trip、Native Host availability、またはWeb handshakeを必須にしない。

## Connected lifecycle state model

これはexact transportやprocess managerを固定しないconceptual lifecycleである。

```text
Native components installed/setup
    -> Web opened
    -> Native Host discovered/available
    -> protocol + capability negotiation
    -> valid prior authorization checked
       | valid                         | missing/expired/incompatible/unavailable
       v                               v
Connected mode                    Standalone mode + recovery guidance
       |
capability-scoped native operations
```

初回またはauthorization expiry時は、explicit user authorizationを完了した後に同じnegotiationを再開する。後続の通常
visitでは、validなauthorizationを再利用できる範囲でterminalを介さないautomatic pathを使用する。

## Capabilityとcomposition root

feature availabilityは、次の概念モデルで表現する。名前は説明用であり、stable wire APIではない。

| Runtime | Host | workspace read/write | validation | native build | external publish |
| --- | --- | --- | --- | --- | --- |
| CLI | Native composition root | Native capability | 共有application | Native capability | Native capability |
| Desktop | Tauri + Native composition root | Native capability | 共有application | Native capability | Native capability |
| Standalone Web | Browser composition root | explicit permission内 | 共有可能なpure logic | capabilityなし | capabilityなし |
| Connected Web | Browser/loopback adapter + Native Host | grantされたcapability | 共有Native application | grantされたcapability | grantされたcapability |

CLI、Desktop、Webはそれぞれのcomposition rootでadapterとpermission/transportを組み立てる。DI frameworkの採用、
具体的なmodule layout、wire endpoint名はこの仕様で決めない。Native Host APIは`get_capabilities`、
`open_workspace`、`get_project_snapshot`、`apply_source_change`、`validate`、`build`、`publish`のような
coarse-grained use-caseを概念例とするが、exact endpointやtransportはOpen Questionである。

## 既存仕様との接続

- [ADR 0002](../adr/0002-rust-core-shared-by-cli-and-gui.md)のCLI/Tauri shared core決定を維持し、Webまでhost
  boundaryを一般化する。ADR 0002の意味を否定しない。
- [ADR 0003](../adr/0003-dotnet-mastermemory-bridge.md)の.NET delegationを維持する。Connected WebはNative
  serviceを呼ぶが、browserに.NETを持ち込まない。
- [Build pipeline仕様](build-pipeline.md)の`BUILD-ARTIFACT-*`、`ARTIFACT-SET-*`、`PUBLISH-*`、
  `PUBLISH-PATH-*`、`PUBLISH-EXEC-*`を変更しない。Connected Webは同じNative servicesをadapter経由で呼ぶ。
- [Project layout仕様](project-layout.md)のNative project discoveryをBrowser Hostへ暗黙適用しない。user-granted
  browser workspaceとNative project discoveryは別のauthorityである。

## 互換性と非目標

この仕様の適用は既存CLI/Desktop runtimeのobservable behaviorを変更しない。Web runtime、WASM host adapter、
Native Host process、loopback RPC、external publish、Tauri publish UIは未実装である。Build Profile、Reference、
semantic schema cache、`build --publish`、Unity`.meta` lifecycle、artifact signingもこの仕様の対象外である。

次の事項は今回のarchitecture acceptanceで決めない。

- Standalone Webの正式browser support matrix、File System Access API fallback、browser filesystem edge case
- IndexedDB等のexact persistence、PWA、offline install、service worker/cache policy
- Native Host lifecycle integrationのmechanical choice（background service、on-demand activation、OS-specific startup、
  package/installer形態、process lifetime、idle shutdown、crash restart）とupdate mechanism
- pairing token、production/development origin allowlist、protocol backward compatibility window
- Standalone/Connected transition、dirty buffer handoff、permission/session UI
- GitHub Pages custom domain

## Acceptance / future evidence

以下はcanonical architectureに対するfuture implementation evidenceであり、現時点ではすべて
`pending implementation`である。存在しないruntime testをpass済みとは扱わない。

| Requirement | Planned evidence | Status |
| --- | --- | --- |
| RUNTIME-HOST-001/002 | `cli_uses_direct_native_application_path`; `desktop_and_cli_share_native_build_semantics`; `shared_domain_does_not_depend_on_loopback_transport` | pending implementation |
| RUNTIME-HOST-003 | `standalone_web_uses_shared_validation_semantics`; `standalone_web_does_not_expose_build_without_native_capability` | pending implementation |
| RUNTIME-HOST-004/005/006 | `connected_web_can_receive_build_capability_from_native_host`; `connected_web_uses_same_native_build_service_as_cli` | pending implementation |
| RUNTIME-HOST-007/011 | `native_host_scopes_workspace_authority`; `web_workspace_url_does_not_embed_absolute_filesystem_path` | pending implementation |
| RUNTIME-HOST-008 | `native_host_rejects_unpaired_privileged_requests`; `native_host_scopes_workspace_authority` | pending implementation |
| RUNTIME-HOST-009 | `protocol_mismatch_does_not_execute_native_operation` | pending implementation |
| RUNTIME-HOST-010 | `connected_web_uses_same_native_build_service_as_cli` and .NET boundary integration evidence | pending implementation |
| RUNTIME-HOST-012 | `web_architecture_does_not_require_remote_backend` | pending implementation |
| RUNTIME-HOST-013 | shared domain/host adapter boundary review and async boundary evidence | pending implementation |
| RUNTIME-HOST-014 | `paired_native_host_connects_without_terminal_command`; `cli_does_not_depend_on_native_host_process` | pending implementation |
| RUNTIME-HOST-015 | `web_start_auto_negotiates_native_host`; `first_connection_requires_explicit_authorization`; `expired_authorization_requires_reauthorization` | pending implementation |
| RUNTIME-HOST-016 | `native_host_unavailable_keeps_standalone_usable`; `protocol_mismatch_keeps_native_operations_disabled` | pending implementation |

## Open Questions

以下はarchitectureのapproved boundary内で、後続のsecurity design、GUI specification、またはimplementation spikeが
observable detailを決めるまでOpenとする。

- Standalone Webのbrowser support matrixとFile System Access API fallback strategy。
- Browser persistenceのexact mechanism（IndexedDB等）とworkspace registration storage。
- Native Host lifecycle integrationの具体方式（background service vs on-demand activation、OS-specific startup integration、
  installer/package manager、process lifetime、idle shutdown、crash restart、update mechanism）。zero-terminal normal workflow
  自体は`RUNTIME-HOST-014..016`で確定している。
- pairing token format、production/development origin allowlist、session renewal、protocol compatibility window。
- exact workspace dirty-buffer synchronizationとStandalone/Connected mode transition UX。
- GitHub Pages custom domain、PWA/offline install、service worker/cache policy。
- browser-specific filesystem edge casesとsource discovery symlinkのproduct policy。
