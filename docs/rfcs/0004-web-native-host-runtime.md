# RFC: Web applicationとNative Hostのlocal-first runtime architecture

Status: Accepted

## 背景（Context）

MasterDataは現在、Rust application coreをCLIとTauri Desktopで共有し、MasterMemory固有のcompileとbinary buildを
.NET adapterへ委譲している。将来はWeb applicationを正式なproduct hostとし、GitHub Pagesなどのstatic hostingから
authoringを提供しながら、必要に応じてlocal Native Hostへ接続してnative build/publish capabilityを利用する。

このarchitectureは、cloud backendを必須にせず、user dataとcomputeをlocal environmentに置くlocal-first方針と、
既存のCLI/desktop semantics、.NET boundary、canonical artifact/publish contractを同時に維持する必要がある。

## 課題（Problem）

Webを既存CLI/Tauriと別applicationとして実装すると、YAML validation、Type System、Build Selection、build semantics、
diagnosticsが複製される。一方、CLIまでWebのためのRPCを通すと、単体利用時の性能と単純性を損なう。またbrowserへ
.NET/Roslyn/MasterMemoryを持ち込むと、既存の責務分離と安全境界が崩れる。

## 目標（Goals）

- Webを正式なproduct hostとして扱う。
- Standalone Webで、explicit permission内のlocal authoringとvalidationを可能にする。
- CompatibleかつauthorizedなNative Host接続時に、既存Native build/publish capabilityを再利用する。
- CLIのdirect in-process pathと性能特性を維持する。
- Shared frontend、shared domain/application semantics、host capability boundaryを定義する。
- static hosting、monorepo/separate checkout、local-first workflowを支える。

## 非目標（Non-Goals）

- browser内の.NET、Roslyn、またはMasterMemory binary formatのRust再実装。
- central build server、database、user account、cloud workspace sync、multi-user collaboration。
- このRFCでのNative Host process、loopback RPC、WASM adapter、external publish runtimeの実装。
- Native Hostのtransport、pairing token、installer、origin allowlistのexact design。

## 選択肢（Options）

### Option A — Desktop / CLIのみを維持

実装とsecurity surfaceは最小だが、static Web authoringというproduct directionを満たさず、remote/cloudを必須に
しないlocal-first Webの入口も提供できない。

### Option B — Webを完全に別applicationとして実装

browser側の自由度は高いが、domain/editor/validation/build semanticsが複製され、CLI、Desktop、Web間のdriftと
maintenance costが増える。

### Option C — WebをStandalone authoring-onlyに固定

browser implementationは単純になるが、同じlocal machine上のNative build/publish capabilityをWeb UIから安全に
利用する道を閉じ、Desktop/CLIと共有できるapplication serviceの価値を制限する。

### Option D — Adopted: Shared frontend/domain + Browser Standalone + optional local Native Host Connected mode

Shared frontendとShared Domain/Applicationを中心に置き、Standalone WebはBrowser Hostのpermission boundary内で
authoring/validationを行う。Connected WebはcompatibleかつauthorizedなNative Hostのadvertised capabilityを利用する。
CLIはNative servicesをdirect/in-processで呼び、DesktopはTauri adapterを使う。

## 採用案（Proposal）

```text
                 Shared frontend / shared semantics
                              |
                         Host interface
                /             |              \
        Browser Host        Tauri       Native/loopback adapter
             |                |                 |
       Standalone Web    Desktop + Native   Connected Web
                              ^
                         CLI direct call
```

Host boundaryはfilesystem、workspace acquisition、permission、process、dotnet、artifact/publish、transportに置き、
pure parser/resolver/validatorへ不要なplatform traitまたはasync runtimeを伝播させない。Native Hostはlocal/loopbackに
限定し、explicit authorization、origin/session/capability/workspace scoping、protocol negotiationを要求する。APIは
Rust内部functionの1:1 mirrorではなく、workspace、validate、build、publishなどのcoarse-grained use-caseを境界とする。

Standalone Webはnative build/publishを持たず、その不可用性をcapabilityで判定する。Connected Webのfull buildはNative
Hostが既存のNative application servicesとsystem dotnetを使う。GitHub Pagesは初期static deployment targetだが、
provider固有のsemanticsはproduct contractにしない。

## 代替案を採用しない理由（Rejected alternatives）

- Browser内で.NET/Roslyn/MasterMemory full buildを行う: Native/.NET boundaryと配布サイズ・runtime前提を壊すため拒否。
- MasterMemory binary formatをRustで再実装する: ADR 0003の責務分離とbinary互換のauthorityを壊すため拒否。
- central build serverをmandatoryにする: local-first、offline、user data ownershipに反するため拒否。
- CLIもlocalhost RPCを通す: direct in-process性能と単純性をWeb transportのために犠牲にするため拒否。

## トレードオフ（Trade-offs）

initial implementation complexityとbrowser compatibility costは増える。Native Hostのsecurity、protocol negotiation、
installer/update lifecycleのmaintenance surfaceも現れる。一方、domain/editor/application semanticsの重複を避け、
CLI hot pathをdirectに保ち、同じNative servicesをDesktopとConnected Webから再利用できる。Standalone Webはnative
toolchainなしでもauthoring/validationを提供できる。

## 互換性（Compatibility）

既存のCLI/Tauri shared core（ADR 0002）、.NET bridge（ADR 0003）、project layout、canonical artifact、receipt、publish
path/execution semanticsは変更しない。Connected WebはそれらのNative application servicesをadapter経由で呼び出すだけで、
別のbuild semanticsを追加しない。具体的なruntime implementationは未実施であり、既存runtime behaviorもこのRFC自体では
変更しない。

## 未解決事項（Open Questions）

- browser support matrix、File System Access API fallback、IndexedDB等のexact persistence。
- Native HostのHTTP/WebSocket等のtransport、port discovery、pairing token、origin allowlist。
- Native Host installer/service lifecycle、auto-start、update、protocol backward compatibility window。
- dirty-buffer handoff、Standalone/Connected transition UX、permission/session renewal。
- GitHub Pages custom domain、PWA/offline、service worker/cache、browser filesystem edge cases。

## 決定（Decision）

このRFCではOption Dを採用する。Human maintainerは、Web product host、Standalone/Connected mode、Native Host capability
boundary、CLI direct composition、shared frontend/domain、.NET boundary維持、およびlocal-first/static-hosting方針を
承認した。採用されたobservable behaviorのcanonical ownerは
[Runtime hosts仕様](../specs/runtime-hosts.md)であり、RFCはalternativeとtrade-offを記録する。詳細なruntime、security、
transportは後続のimplementation/specification taskで決める。
