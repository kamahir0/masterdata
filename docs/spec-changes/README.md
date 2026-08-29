# 仕様変更（Specification changes）

このdirectoryには、既存のcanonical specificationを変更するdurableなproposalを保存する。
`docs/specs/` とは意図的に分離している。人間が承認する前に、`Approved` / `Implemented` canonical
documentへsemantic changeを含めてはならない。

## lifecycle（ライフサイクル）

1. `refine-spec` がevidence、affected Requirement ID、proposed delta、compatibility impact、Open Questionsを
   新しいartifactに記録する。
2. `review-spec` がartifactを、source request、canonical specification、ADR、terminology、current implementation、
   testabilityと照合して独立に確認する。
3. review中、artifactは `Proposed` である。human maintainerが明示的に `Approved` または `Rejected` へ移す。
   AI-generated artifactをauto-approveしてはならない。
4. 明示的なapproval後、deltaをcanonical specificationへatomicに適用し、必要に応じてtests/fixturesを更新する。
   その後artifactを `Applied` へ移し、approvalと変更したcanonical requirementを記録する。canonical documentは、
   implementation evidenceによって `Implemented` が正当化されるまで `Approved` のままとする。以前に
   `Implemented` だった場合は、先に `Approved` へ戻す。

`Draft` はartifactが不完全な状態を表す。`Proposed` はreview可能だが未承認の状態を表す。`Approved` はhuman
decisionを記録するが、canonical mergeはまだpendingである。`Applied` は承認済みdeltaがcanonicalに存在することを
記録する。`Rejected` はproposalが却下されたことを記録する。proposalは `Applied` になるまでimplementation
contractではない。`implement-spec` はmerge後のcanonical approved documentを使用し、proposal自体を使用しては
ならない。optionの比較を含む大きなchangeには `docs/rfcs/` を使用し、採用されたbehaviorをこのworkflowへ
routeする。

新規artifactには [_template.md](_template.md) を使用し、`0001-table-identity.md` のようにmonotonically allocate
したfilenameを付ける。proposal numberはhistoryであり、再利用してはならない。
