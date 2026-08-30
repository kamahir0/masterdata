---
name: review-code
description: Review implementation diffs for specification conformance, regression safety, rationale freshness, reverse traceability, evidence integrity, and architecture boundaries.
---

# review-code

## 目的と責務

このskillは、実装diffが既存のauthorityとevidenceに照らして安全かを確認する、final implementation reviewである。
`review-spec`が「仕様は正しいか、approval可能か」を確認するのに対し、`review-code`は「実装diffはその仕様に
適合し、regressionとrationaleの鮮度が保たれているか」を確認する。

`review-code`はcanonical specificationを変更しない。Open Questionを解決せず、未定義のobservable behaviorを
実装で発明しない。Approved specにbehaviorが定義されているのに実装が違反する場合はbug findingとして扱う。
Approved specからbehaviorを選択しなければならない場合は`Specification Gap`として`refine-spec`へ戻す。

## 必須のinput

変更範囲に応じて、次をdiff中心に読む。

- changed filesとimplementation diff
- 関連する`Approved` / `Implemented` specificationとRequirement ID
- 関連test、fixture、goldenまたはbenchmark
- touched codeのnearby rationale comment
- 参照されたADR、RFC、issue/reference、external/platform constraint
- 必要なarchitecture boundary（core、application、CLI、GUI、codegen、.NET adapter）

repository全体を無制限に探索しない。変更から合理的にdiscoverableな関連範囲を確認する。ただし、
unusual codeの削除・簡略化・移動では、nearby rationale、Requirement ID、regression test、ADR/RFC、benchmark、
platform/library/toolchain constraintを検索してから判断する。

## Rationale-sensitiveな変更

次の変更はrationale freshnessを確認する。

- rationale commentに近接するcodeの変更
- rationaleが保護するfunction、method、moduleのrefactorまたは移動
- protected invariantの変更
- 参照されたregression testの変更または削除
- 参照されたRequirement IDのowner/status変更
- 参照されたADR/RFCのsupersede
- dependency、toolchain、platform assumptionの変更
- optimizationまたはworkaroundの変更
- simplify、deduplicate、replace、ordering、timing、filesystem、concurrencyの変更

whitespace、formatting、mechanical renameなど意味的に無関係な変更を自動的にblockしてはならない（MUST NOT）。
すべてのbranch、clone、allocationにcommentを要求してはならない。対象は、straightforwardな実装から意図的に
外れ、理由を失うと将来の変更判断を誤らせるcodeである。

再検証結果は次のいずれかにする。

- `Still accurate`: current implementation、failure mode、evidenceに一致する。保持する。
- `Invariant/reason changed`: protected invariantまたは理由を更新し、evidenceも更新・再確認する。
- `Reason no longer applies`: invariantが消えた、または別のevidenceで保護されていることを確認して削除する。

testの成功とrationaleの鮮度は別の証拠である。Testはbehaviorを証明し、Commentはimplementation shapeがなぜ
存在するかを説明する。参照先のtestが通っていても、古い理由を残してはならない（MUST NOT）。

## Review checklist

### Specification conformance

- implementationが関連する`Approved` / `Implemented` behaviorに適合しているか。
- `Draft` / `Proposed` spec、conversation、current code behaviorを未承認のimplementation authorityとして
  扱っていないか。
- Requirement ID referenceが実際に対象behaviorを指し、Diagnostic Codeと混同されていないか。
- Approved specで答えられないobservable behaviorを勝手に選択していないか。

### Regression and evidence

- 変更されたbehaviorにfocused regression testがあるか、既存testが実際に保護しているか。
- test nameが`test_bug_1`のような番号だけでなく、保護するbehaviorを表しているか。
- fixture、golden、benchmark、external constraintが変更後の実装と対応しているか。
- `Regression:`、Requirement ID、ADR/RFC、documentation pathなどの明示参照先が存在するか。

### Rationale freshness

- non-obvious codeに必要な範囲のrecoverableなWHYがあるか。
- commentがWHATの説明ではなく、protected invariantとfailure modeを説明しているか。
- current implementationがcommentの理由と一致しているか。
- 参照testが主張されたbehaviorを実際に保護しているか。
- workaroundのaffected dependency/platform、protected behavior、removal conditionが古くなっていないか。
- refactorでrationaleが旧locationに残る、または新locationへ移動されずorphanになる状態がないか。
- optimizationにbenchmark、profile、allocation evidence、またはknown hot pathがあるか。
- unusual codeをsimplify・deleteする前に、そのinvariantを確認した証拠があるか。

### Authority and architecture

- local commentをproduct specificationの代わりにしていないか。
- local rationaleを新しいproduct requirementへ自動昇格していないか。
- Specはobservable behavior、ADRはcross-cutting architecture、Testはregression evidence、Commentはlocal
  implementation rationaleというowner分離を守っているか。
- core、application、CLI、GUI、codegen、.NET adapterのboundaryを越えてdomain semanticsを重複させていないか。

## 構造参照checkとの分離

`cargo xtask check-rationale`は、機械的に高い確度で検証できる次の参照だけを対象とする。

- comment内のRequirement IDがcanonical specificationで定義されていること
- `ADR-NNNN` / `RFC-NNNN`参照に対応する番号付きdocumentが存在すること
- `Regression: test_identifier`がsource内のcomment外に存在すること
- `docs/...`として明示されたrepository-relative documentation pathが存在すること

このcheckは自由記述commentの意味、WHYの正しさ、failure modeの鮮度、workaroundの必要性、benchmarkの妥当性を
判定しない。独自の`@rationale`等のcomment parser DSLを導入しない。構造参照が曖昧なら自動checkを拡張せず、
このskillのreview findingとして扱う。

## Finding classification

- `Blocking`: mergeするとknown correctness、spec、compatibility、architecture、またはprotected invariantに
  違反する。
- `Non-blocking`: approvalを妨げないeditorialまたはmaintainability concern。
- `Rationale Gap`: non-obvious implementationがあるが、理由を安全に復元できない。
- `Stale Rationale`: commentがcurrent implementation、failure mode、またはcurrent evidenceと一致しない。
- `Evidence Gap`: rationaleの主張を支えるtest、spec、ADR、benchmark、またはexternal referenceが不足・破損している。
- `Specification Gap`: implementationに必要なobservable behaviorをApproved specから選べない。

`Stale Rationale`が将来のsimplificationや削除を誤らせる場合は`Blocking`でも報告する。単なる誤字や軽微な
editorial issueは`Non-blocking`とする。理由が不明なままcommentを書き換えて新しいreasonを発明してはならない。

## 必須のreview report

次の構造で短く報告する。

### Scope

reviewしたdiff、implementation boundary、関連spec/test/evidence。

### Specification Conformance

`Pass` または具体的なissue。

### Tests and Regression Evidence

関連test、fixture、benchmarkの十分性と参照整合性。

### Rationale Freshness

`Fresh`、`Stale`、`Missing`、`Not applicable` を、対象ごとに理由付きで記録する。

### Evidence Integrity

- Requirement references:
- ADR/RFC references:
- Regression test references:
- Benchmark/external references:

### Architecture

boundary violationの有無。

### Findings

severity付きで`Blocking`、`Non-blocking`、`Rationale Gap`、`Stale Rationale`、`Evidence Gap`、
`Specification Gap`を分類する。空なら`None identified`と書く。

### Verdict

`Ready to merge: Yes` または `Ready to merge: No`。ただしこのskillはspec statusを変更せず、semantic gapを
自動修正しない。

## 通常のcompletion flow

`implement-spec`またはApproved specに対するbug fixでは、次の順で実行する。

```text
implementation completed
        -> tests / regression evidence
        -> re-scan touched rationale
        -> review-code
        -> fixes, if needed
        -> cargo xtask check-rationale
        -> cargo xtask check-all
```

tests passだけ、またはreferenceが存在するだけでreviewを省略してはならない。rationaleが不要になった場合も、
protected invariantが消えたか別のevidenceへ移ったことを確認してから削除する。
