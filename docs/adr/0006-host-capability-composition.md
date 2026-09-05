# ADR 0006: Host capabilityとcomposition rootを分離する

Status: Accepted

## 背景（Context）

ADR 0002はRust core/application semanticsをCLIとTauri Desktopで共有し、GUIからCLI subprocessを起動しないことを
決定した。Web applicationを追加する際は、同じ原則をBrowser Host、Native Host、CLI、Tauri、Connected Webへ一般化する
必要がある。Webのasync filesystemとpermissionをpure coreへ漏らさず、Native buildをbrowserへ複製せず、CLIのdirect pathを
維持することが重要である。

## 決定（Decision）

次のarchitectureを採用する。

```text
pure/shared domain
        |
shared application / use cases
        |
host ports
   /              \
Native adapters   Browser adapters
   |                    |
Native Host         Standalone Web
   |
+----------+----------------+
|          |                |
CLI       Tauri        Connected Web adapter
direct    Desktop      (loopback等のtransport)
```

- CLI、Tauri、Connected Webは同じNative Application Servicesを利用する。
- CLIのcomposition rootはNative servicesをdirect/in-processで組み立て、Web対応のためのlocalhost RPCを必須にしない。
- Desktopは共有frontendとTauri adapterを使い、shared application/domain semanticsへ接続する。
- Standalone WebはBrowser Host adapterと共有可能なpure logicを使い、explicit permission内のauthoring/validationを行う。
- Connected WebはcompatibleかつauthorizedなNative Hostがadvertiseするcapabilityだけを利用する。
- host portsはfilesystem、permission、process/dotnet、artifact/publish、transportなどhost固有の責務に限定する。
  YAML semantics、Type System、validation、Table resolution、Build Selectionなどを不要にtrait化しない。
- async Host I/Oはlogical documents/bytesとpure synchronous coreの境界で扱い、core全体をasync化しない。
- Native full buildはADR 0003の.NET bridgeを使用し、browserでMasterMemory binary formatを実装しない。

security boundary、protocol negotiation、capability/workspace scopingのobservable requirementは
[Runtime hosts仕様](../specs/runtime-hosts.md)が所有する。transport、pairing、installer、exact module layoutはこのADRで
選択しない。

## 結果（Consequences）

Shared frontendとshared semanticsの維持により、CLI、Desktop、Webのdomain/editor behaviorのdriftを抑えられる。CLIは
serializationやnetwork round-tripなしで動作できる。Standalone Webはnative toolchainなしでauthoring/validationを提供でき、
Connected WebはNative Hostのsecurity/protocol lifecycleを新たに維持する必要がある。Browser permission、WASM、Native Host、
transportのexact implementationは後続taskのscopeとなる。

既存ADR 0002の「CLIとGUIが同じRust core/applicationを共有する」決定は有効であり、このADRはそれをWebとhost capability
boundaryへ拡張する。既存build/publish specificationのownerを移動または複製しない。

## 代替案（Alternatives）

- Webを独立したdomain/applicationとして実装し、CLI/Desktopとsemanticsを個別に保守する。
- CLIもloopback RPC経由に統一し、全hostを同一transportで扱う。
- browserへ.NET/MasterMemory builderを移植し、Native Hostを不要にする。
- すべてのI/Oとparser/resolverを汎用async traitへ抽象化する。

これらは、domain duplication、CLI performance regression、既存.NET boundaryの破壊、またはpure coreへの不要なhost
semanticsの漏出を招くため採用しない。

## Traceability

observable contractは`RUNTIME-HOST-001`から`RUNTIME-HOST-013`が所有する。RFC、specification change、既存ADR 0002/0003は
rationaleとcross-referenceであり、同じnormative ruleの第二のownerではない。
