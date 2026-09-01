# .NET builder boundary（.NET builderの境界）

このprojectは、`masterdata-dotnet` がbridge smoke testとproduction buildで利用するrepository-owned .NET builderである。
production pathではRustが生成C#とvalidated/normalized valueを一時的なstaging workspaceへ配置し、staged projectとして
このbuilderをcompile/runする。builderはMasterMemory v3 Source Generator、C# compilation、DatabaseBuilder、binary reload
validationを所有する。

RustはMasterMemoryやMessagePackのinternalを再実装せず、YAMLを.NET側で再parseしない。Rust/.NET間のrequest/reportは
repository同梱builderとのinternal protocolであり、released compatibility contractではない。

repositoryにおける実際のdependency/API compatibility experimentは、独立した
[MasterMemory v3 technical spike](../spike/masterdata-mastermemory-spike.csproj) であり、
`cargo xtask mastermemory-spike` によって実行する。これはproduction schema-driven builderではない。

手動smoke test:

```bash
dotnet run --project dotnet/builder/masterdata-builder.csproj -- --self-test
```

production buildはapplication serviceから起動される。通常のbuildではfinal binaryへ直接書き込まず、builder reportと
`MemoryDatabase` reload validationが成功した後にapplication serviceがfinal pathへpublishする。
