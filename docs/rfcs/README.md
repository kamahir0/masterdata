# RFC

RFCは、adoption、alternative、trade-offを検討している段階で使う、比較的大きなdesign proposalである。
承認済みproduct specificationではなく、`implement-spec` のimplementation authorityとして使用してはならない
（MUST NOT）。

## RFCのlifecycle（RFCのライフサイクル）

- `Draft`: proposalを整理中で、未完了のalternativeまたはOpen Questionを含んでもよい。
- `Proposed`: reviewできる状態だが、採用されたdecisionはない。
- `Accepted`: human maintainerがproposalの方向性を選択した状態。採用されたproduct behaviorはcanonicalな
  `docs/specs` documentに属する。
- `Rejected`: proposalが明示的に却下され、historyのために保持される状態。
- `Superseded`: 後続RFCがproposalを置き換えた状態。successorをdocumentからlinkするべきである。

RFC statusはproduct specification statusとは別である。特に、`Accepted` はproduct specificationを
`Approved` にせず、`Implemented` はRFC statusではない。Accepted RFCがimplementation authorityになるには、
approved-specification change workflowを通じてcanonical specificationへ反映する必要がある。

proposalが選択されたら、採用されたbehaviorを関連する `docs/specs` documentへ移す。RFCはrationaleとして
残してもよく、結果のRequirement IDへlinkするべきである。architectural reasonを保持することが
重要なら、normative ruleを重複させず、`docs/adr` に別途記録する。

alternativeがすでに理解されている `Approved` または `Implemented` canonical specificationへのfocusedな
semantic deltaには、`docs/spec-changes/` を使用する。change artifactはdurableなreview inputだが、人間が
承認したatomic mergeによってcanonical specificationが更新されるまではimplementation authorityではない。

新しいRFCには [_template.md](_template.md) を使用し、conversationからreviewへ至る経路には
[仕様ワークフロー（specification workflow）](../contributing/specification-workflow.md)を使用する。
