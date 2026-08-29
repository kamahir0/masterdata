# .NET builder boundary（.NET builderの境界）

このprojectは、`masterdata-dotnet` がreal bridge smoke testに使用する、意図的に小さなdependency-free .NET executable
である。Rustがrepository-owned .NET projectを呼び出し、そのexit statusとoutputを読み取れることを検証する。

MasterMemory binary generationは実装しない。将来のbuilderがMasterMemory v3 Source Generator、C# compilation、binary
build、binary validationを所有する。Rustはadapterであり続け、これらのinternalを再実装してはならない。

repositoryにおける実際のdependency/API compatibility experimentは、独立した
[MasterMemory v3 technical spike](../spike/masterdata-mastermemory-spike.csproj) であり、
`cargo xtask mastermemory-spike` によって実行する。これはproduction schema-driven builderではない。

手動smoke test:

```bash
dotnet run --project dotnet/builder/masterdata-builder.csproj -- --self-test
```
