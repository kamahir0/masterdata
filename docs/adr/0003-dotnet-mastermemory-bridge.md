# ADR 0003: .NETをMasterMemory bridgeとする

Status: Accepted

## 背景（Context）

MasterMemory v3 Source Generatorとbinary detailは、.NET library ecosystemに属する。これらのinternalを
Rustで再実装すると、compatibilityとmaintenanceのriskが生じる。

## 決定（Decision）

Rustはvalidated input、generated C# scaffold、schema source-content hash、およびvalidated valueを含むinternal
builder requestを準備する。native .NET invocation boundary は.NET process invocationを所有し、stagedなrepository builderへ
requestを渡してC#をcompileし、MasterMemory binaryを生成・reload validationする。artifact確定と外部Publishのlifecycleは[Build contract](../specs/build-pipeline.md)が所有する。

## 結果（Consequences）

process boundaryは明示的かつtest可能でなければならない。実consumerによるcompile / binary load / lookupを検証する。production pathはsource YAMLを.NET側で再parseせず、Rustのvalidated semanticsをboundaryで渡す。source-content hashをsemantic identityやwrite authorizationへ代用しない。exact protocol、process名、旧technical spike配置はrewrite constraintではない。
