# ADR 0003: .NETをMasterMemory bridgeとする

Status: Accepted

## 背景（Context）

MasterMemory v3 Source Generatorとbinary detailは、.NET library ecosystemに属する。これらのinternalを
Rustで再実装すると、compatibilityとmaintenanceのriskが生じる。

## 決定（Decision）

Rustはvalidated input、generated C# scaffold、schema source-content hashを準備する。
`masterdata-dotnet` は.NET process invocationを所有し、将来はrepository builderを呼び出してC#をcompileし、
MasterMemory binaryを生成・validateする。

## 結果（Consequences）

process boundaryは明示的かつtest可能でなければならない。repositoryにはdependency-freeなbuilder
smoke testと、Source Generator、binary build、reload、lookupを実行する独立したhand-written MasterMemory v3
technical spikeを含める。production schema-driven generationは明示的なnot-implemented stageとして残る。
将来のcache reuseでは、schema source-content hashをsemantic identityとして扱わず、semantic schema/cache-key
designを使用する必要がある。
