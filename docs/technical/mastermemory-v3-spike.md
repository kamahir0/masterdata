# MasterMemory v3 technical spike（technical spike）

Status: Experimental validation artifact

このspikeはproduct YAML Type Systemおよびproduction schema-driven generatorから意図的に独立している。
pinされた.NET dependencyを `masterdata-dotnet` 経由で呼び出し、次のMasterMemory v3 round tripを実行できる
ことだけを検証する。

```text
hand-written TestMaster.cs equivalent
  -> dotnet build/run
  -> MasterMemory Source Generator
  -> DatabaseBuilder.Build()
  -> binary file
  -> MemoryDatabase(byte[])
  -> generated FindBy... lookup/assertion
```

## 固定されたinput

- .NET SDK: repositoryの `global.json` は8.0.300 feature bandを要求し、最新featureへのroll-forwardを許可する。
- MasterMemory NuGet: `3.0.4`。
- MessagePack NuGet: `3.1.3`。
- Target framework: `net8.0`。

package versionを明示するのは、将来のbuilder cache compatibility inputになるためである。technical spikeには
`dotnet/spike/masterdata-mastermemory-spike.csproj` に独自のprojectがあり、Rust C# scaffoldから生成されるものではない。
`packages.lock.json` はcheck-inし、locked restoreを有効にしてtransitive versionが黙って変化しないようにする。
current pinned MessagePack versionはこのenvironmentでNuGet audit warningを発生させる。spikeではselected
MasterMemory packageが要求するversionを維持し、future dependency upgradeはwarningをsuppressionして隠すのではなく、
compatibility/security changeとして評価する必要がある。

## 検証内容

spikeは、hand-writtenな `SpikeItem` tableを1つ、integer primary keyを1つ、recordを3つ宣言する。binaryをbuildして
`target/` 配下へwriteし、generated `MemoryDatabase`へそのbinaryをreloadし、`FindByItemId(1002)`を呼び出し、recordが
`Hi-Potion` であることをassertする。Rustの `masterdata-dotnet` bridgeはJSON resultをparseし、
`cargo xtask mastermemory-spike` と `cargo xtask check-all` へevidenceを公開する。

## 境界と非目標（Boundary and non-goals）

spikeはrepository YAMLをparseせず、production C#をgenerateせず、MasterMemory internalをRustで再実装せず、futureの
product primary-key specificationを定義しない。spikeの成功は.NET adapterに対するcompatibility evidenceに限られる。

次で実行する。

```text
cargo xtask mastermemory-spike
```

commandはclean machine上でnetwork-capableな.NET restoreを必要とする。CIではUbuntu、Windows、macOSに.NET 8 SDKを
installした後に実行する。
