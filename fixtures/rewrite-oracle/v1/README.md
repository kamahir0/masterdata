# Independent Rewrite Oracle v1

このdata-only corpusは旧production module / private helper / transportを参照せず、**何が成功か**を記述する。scenarioのoperationはdomain vocabulary、expectedは独立に指定したbytes / state / public consumer behavior。adapterが期待値を生成しない。

`manifest.json`のbyteScenarios / saveScenarios / structuralScenariosは各directoryの`input/`、`scenario.json`、`expected/`へrouteする。sourceはUTF-8 bytes、CRLF caseは実CRLF。比較時に改行・quote・空白をnormalizeしない。occurrenceはbase capture時のphysical source内1-based順序で、PK valueではない。content identityのhash algorithmやinternal representationは固定しない。

`meaning: text / integer / null / mapping`はoracle vocabulary。整数はdecimal stringでlosslessに保持する。新実装のDTOではない。現在のadapterは別test codeにあり、このformatへtranslationする。

`workflows.json`はUI / lifecycle / injected failureのportable event-and-state oracle。pixel / CSS / component treeはrequiredではない。inputは共通小fixture、domain scenarioに応じたexternal change / failure injectionを行う。断定的なUI結果とfault injectionの実行coverageを混同しない。

`interpretation.json`はschema-directed scalar expectation。`capacity.json`は100k inputと全candidate cellの数式oracle。`consumer/`は既存`fixtures/full` + 二つのprobe sourceからcanonical Buildし、独立C# consumerでpublic query / binary semanticsを検証する。compileとloadは別phase。formattingやprivate codegen functionは固定しない。

## Adapter coverage / known gaps

byte adapter: 16 pass、`new-mapping-block`はD6 targetに対する既知legacy flow writer gap。これをexpectedへ取り込まない。Structural adapter: Rename / Add / Dropの3 multi-source byte cases。Interpretation adapter: 9 cases。Save adapter: 7 scenarios、inline / separate / mixed、dirty scope、cached base後のexternal Conflictを実行する。

workflow oracleはcontractであり、全23 workflowsが新adapterで自動実行済みという主張ではない。current behavior testとDesktop evidenceへの対応は[acceptance matrix](../../../docs/rewrite-preparation/acceptance-matrix.md)。未抽出 / 未実行coverageはreadiness reportで明示する。

known gapの判定はlegacy adapter側にだけ置く。corpusの期待値はrewrite pass exemptionを持たない。clean implementationは全target expectationを満たす。known-gap tracking testのgreenをclean implementation conformanceと呼ばない。
