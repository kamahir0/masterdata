# 完全な参照fixture（Full reference fixture）

このdirectoryは、completeなschema modelをdogfoodするためのplanned projectである。initial parserとbuilderがまだ
拡張中のため、現時点では意図的に説明用としている。将来のfixtureでは次を検証する。

- value objectとimmutable custom type
- 通常のenumとflags enum
- primary keyとcomposite key
- unique、non-unique、composite secondary index
- one-to-oneとone-to-manyの `MasterReference`

current fileは、coreがtyped ASTへ保持し、Type System resolver、Table resolver、C# generatorが扱うdeclarationを使用している。
Table-level validationとMasterMemoryのC# loweringは実装済みである。Reference helper、production binary orchestration、cache、
および最終artifact出力は引き続き次のimplementation workである。
