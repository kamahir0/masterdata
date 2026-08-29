# 完全な参照fixture（Full reference fixture）

このdirectoryは、completeなschema modelをdogfoodするためのplanned projectである。initial parserとbuilderがまだ
拡張中のため、現時点では意図的に説明用としている。将来のfixtureでは次を検証する。

- value objectとimmutable custom type
- 通常のenumとflags enum
- primary keyとcomposite key
- unique、non-unique、composite secondary index
- one-to-oneとone-to-manyの `MasterReference`

current fileは、coreがschema ASTに保持できるdeclarationを使用している。Type resolution、index materialization、
reference helper、MasterMemory generationは引き続き未解決のimplementation workである。
