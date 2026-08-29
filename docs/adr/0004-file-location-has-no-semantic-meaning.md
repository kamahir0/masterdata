# ADR 0004: File locationはsemantic meaningを持たない

Status: Accepted

## 背景（Context）

Projectはtableを複数fileに分割しても、すべてのYAMLを1つのdirectoryに置いても、feature/teamごとに
fileを整理してもよい。directoryからtable identityを推測すると、無害なfile移動がsemantic changeになる。

## 決定（Decision）

Filesystem locationはdiscovery boundaryに過ぎない。各YAML fileが `kind` と `table` を宣言し、schema fieldと
将来のindex/reference declarationが残りの意味を担う。

## 結果（Consequences）

Discoveryはconfigured rootをrecursiveにscanし、deterministicなprocessing/hash inputのためにpathをsortする。
source root内でfileを移動してもtable identityは変わらない。source root自体は明示的なproject configurationで
あり、hard-coded directory conventionではない。current traversalはcycle-safetyのinternal guardとして
symlink entryをfollowしない。source discoveryがsymlinkをfollowするかどうかは、別のproduct Open Questionである。
