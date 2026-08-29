# Table identity仕様（Table identity）

Status: Implemented

## 概要

current scaffoldでは、tableのproject-localなlogical identityは1つだけである。generated C# namingは
別のpresentation concernである。

### COMPAT-TABLE-001

current scaffoldでは、`table` がproject-localなstable table identityであり、generated C# type nameは
別のrename可能なpresentation nameである。

このrequirementは、global identity、table rename migration、released-schema compatibility、legacy
`tableId` migration、cross-project identityを定義しない。

## Implementation evidence（実装evidence）

- Schemaとdata documentは、source pathではなく宣言された `table` valueによって関連付く。
- C# generatorは `csharpName` をgenerated type-name overrideとしてのみ使用し、source `table` identityは別に保持する。
- Project discovery testは、source directoryの配置によってdocumentのtable identityを変更できないことを確認する。

Accepted designのrationaleは [RFC 0001](../../rfcs/0001-table-identity.md) に記録し、適用済みapprovalの
recordは [specification change 0001](../../spec-changes/0001-table-identity.md) である。

## Open Questions（未解決事項）

- Projectは将来globally stableなtable identityを必要とするか。
- `table` が変更されたとき、どのcompatibilityとmigration behaviorを適用するか。
- legacy `tableId` inputをrejectするか、non-semantic metadataとして保持するか、migration windowの間だけサポートするか。
- cross-project table identityは必要か。
