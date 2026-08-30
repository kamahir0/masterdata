# Build pipeline仕様（Build pipeline）

Status: Draft

想定するpipelineは次のとおりである。

```text
resolve project
 -> load config
 -> discover YAML
 -> parse schema/data
 -> semantic validation
 -> resolve types/indexes/references
 -> generate C#
 -> calculate schema source-content hash
 -> compile or reuse .NET builder
 -> build MasterMemory binary
 -> validate binary
 -> write temporary output
 -> atomically replace final output
```

Rust coreは最初のstageを担当し、validated documentとdeterministicなschema source-content hashを
持つ `BuildPlan` を公開する。`masterdata-codegen-csharp` はstructured C# renderingを担当する。
.NET builderとの唯一のprocess boundaryは `masterdata-dotnet` である。MasterMemory binary
formatとSource Generatorのbehaviorは、.NET dependencyに残さなければならない（MUST）。

current behaviorは意図的に小さい。`prepare_build` はvalidationとschema source-content hashの
計算を行い、C# crateはprimitive immutable scaffoldをplanし、.NET crateはbridge smoke testと
独立したMasterMemory v3 technical spikeの双方を実行できる。production schema-driven binary
generationは、明示的なnot-implemented boundaryとして残る。non-dry-run application buildは
generated C#をpublishする前にそのboundaryを呼び出すため、現在のnot-implemented failureが
misleadingなfinal generated directoryを残すことはない。production builderが利用可能になった
場合、generated C#、binary output、cacheへのwriteはstagingを使い、必要なすべてのstageが成功した
後にのみpublishしなければならない（MUST）。

Record Tags、Build Profiles、Build Selectionのselection semanticsは、[Build Selection仕様](build-selection.md)が所有する。このdocumentの
high-level pipelineにおけるsemantic validationは、selection前のprofile-independent validationと、selection後のdataset-level
constraint validationへ分けて解釈する。Primary Key、Unique、Referenceの具体的なsyntaxは、それぞれのowner specificationが所有する。

Value Object / Custom Typeのgenerated identifier validationのobservable contractは、Approvedの[C#命名仕様](type-system/csharp-naming.md)が所有する。
採用理由と比較したalternativeは[C# naming RFC](../rfcs/0003-csharp-naming.md)に残す。build pipelineはこの命名ruleを別の
normalizationやrepairへ置き換えてはならない（MUST NOT）。

## Outputとidentityの境界

current configurationでは `build.output` をgenerated C# output directoryとして扱う。任意の
`build.binary_output` は将来のMasterMemory binary destinationを指定でき、`build.cache` は独立した
cache directoryを指定する。これらのpathはbuild plan内で分離して表現し、互いから推論してはならない。final
configuration contract、atomic replacementの詳細、Unityへの配置policyは
引き続きOpen Questionである。

次のidentityは別物であり、混同してはならない（MUST NOT）。

- **Schema source-content hash**: current scaffoldでdiagnosticsとchange detectionに使う、schema source bytesのhash。whitespaceやcommentによって変化し得る。
- **Semantic schema hash**: parsed、resolved、canonicalなschema meaningから将来計算するhash。type systemとYAML subsetが仕様化されるまで実装しない。
- **Builder cache key**: semantic schema hash、C# generator version、MasterMemory version、MessagePack version、builder protocol version、target/runtime compatibility inputなどを含む可能性がある、将来のcomposite identity。

current `BuildPlan` が公開するのはschema source-content hashだけである。これをsemantic builder
cache keyとして説明または使用してはならない（MUST NOT）。generated C# output、MasterMemory binary
output、cache directoryは別概念であり、将来のconfiguration shapeはOpen Questionのままである。
binary buildが存在する場合、output replacementはatomicでなければならない（MUST）。

Open Questions: generated projectのownership、cache eviction、Unity asset importがatomicなoutput
replacementをどう観測するか。source discoveryでsymlinkをfollowまたはignoreするproduct-level policyも
未解決であり、current traversal guardはcycle防止のためsymlink entryをfollowしない。
