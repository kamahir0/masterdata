# ADR 0003: .NETをMasterMemory bridgeとする

Status: Accepted

## 背景（Context）

MasterMemory v3 Source Generatorとbinary detailは、.NET library ecosystemに属する。これらのinternalを
Rustで再実装すると、compatibilityとmaintenanceのriskが生じる。

## 決定（Decision）

Rustはvalidated input、generated C# scaffold、schema source-content hash、およびvalidated valueを含むinternal
builder requestを準備する。`masterdata-dotnet` は.NET process invocationを所有し、stagedなrepository builderへ
requestを渡してC#をcompileし、MasterMemory binaryを生成・reload validationする。application layerはbuilder成功後に
binaryをpublishする。

## 結果（Consequences）

process boundaryは明示的かつtest可能でなければならない。repositoryにはbridge smoke test、staged requestを使う
production builder、およびSource Generator、binary build、reload、lookupを実行する独立したhand-written MasterMemory v3
technical spikeを含める。production pathはsource YAMLを.NET側で再parseせず、Rustのvalidated semanticsをinternal
protocolで受け渡す。将来のcache reuseでは、schema source-content hashをsemantic identityとして扱わず、semantic
schema/cache-key designを使用する必要がある。
