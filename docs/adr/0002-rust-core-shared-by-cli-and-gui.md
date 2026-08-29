# ADR 0002: Rust coreをCLIとGUIで共有する

Status: Accepted

## 背景（Context）

CLIとGUIは、project discovery、parsing、validation、build semanticsを同一にする必要がある。
GUIからCLIをsubprocess callすると、error handling、testing、lifecycle behaviorが不要に間接的になる。

## 決定（Decision）

`masterdata-app` はproject info、validation、build preparation、C# generation、.NET boundaryなどの
shared application orchestrationを所有する。`masterdata-core` はshared domain/document operationを
所有する。CLIとTauriはRust libraryとしてapplication serviceを呼び出し、どちらのfrontendもdomain
logicを再実装したり、CLI subprocessを呼び出したりしてはならない。

## 結果（Consequences）

Core/application APIにはstructuredでserializableなresultとdiagnosticが必要になる。UI固有のformattingは
adapterに置く。project semanticsへの変更はcore boundaryで一度だけ行い、一度だけtestする。Tauriは
diagnostic code、kind、source location、schema path、record identity、suggestion、related requirement
referenceをerror DTOに保持する。
