# Rewrite Readiness Gap Closure

Status: In progress evidence / readiness未判定

## Consumer first-loss boundary

旧Candidate a09e37d（main a5da23dのproduction tree）で、full fixtureと2field最小fixtureを再現した。YAML → resolved / normalized request → .NET nested conversion → Custom構築 → Append前後までは2001を保持。MessagePack wireにも2001が存在する。**最初の誤った境界はCustomのMessagePack deserialization**。binary reload後はItemId.Value=0、Amount=0、Array=defaultとなる。

MessagePack3.1.3の生成formatterは、integer keyに対応するconstructor bindingが宣言順constructorへ適合しないとparameterless structを構築し、get-only fieldsをSkipしていた。SerializationConstructor属性だけではtype mismatch診断となり解決しない。単一のsparse fieldだけでは再現せず、異なる型の2fields・keys[9,0]で再現した。[full before trace](consumer-before-full.jsonl)、[minimal before trace](consumer-before-minimal.jsonl)は一時opt-in .NET instrumentationによる実値。診断用Rust example / builder instrumentationはproductionへ残していない。

修正はC# generatorのCustom型に限定し、公開MessagePack formatter extensionを介してpersisted keyで読み、宣言順の既存public constructorへ渡す。readonly / property / constructor API、key metadata、wire slots、Rust正規化、Build / Publish lifecycleは変えない。再ロード後の実値は[after trace](consumer-after-full.jsonl)。この具体formatter構成はrewrite architecture requirementではない。

Focused actual consumer regressionでは、direct / nested / 2records / declaration-key order / Array / nullable null・non-null / uintを検証し、generated C#とbinaryのrepeat Build一致を確認した。全public consumer oracleのfresh Candidate実行とDesktop distribution / CI reconciliationは継続中。
