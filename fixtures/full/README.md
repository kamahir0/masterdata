# 完全な参照fixture（Full reference fixture）

このdirectoryは、Type System、Table/Key、およびproduction MasterMemory builderを一つのprojectで検証するfixtureである。
`masterdata-app` のend-to-end testはこのfixtureをpathに空白を含むtemporary projectへコピーし、Rustのvalidated modelから
実際の.NET builderへhandoffしてbinaryをreloadする。

- value objectとimmutable custom type
- 通常のenumとflags enum
- primary keyとcomposite key
- unique、non-unique、composite secondary index
- one-to-oneとone-to-manyの `MasterReference`

current fileは、coreがtyped ASTへ保持し、Type System resolver、Table resolver、C# generatorが扱うdeclarationを使用している。
Table-level validation、MasterMemoryのC# lowering、staged production binary build、およびbinary reload validationを検証する。
Reference helper、builder cache、released binary compatibility、Unityへの最終配置は別scopeである。
