# 仕様変更（Specification changes）

このdirectoryは、Approved / Implemented canonical specificationへのsemantic changeをapproval前に隔離し、Applied後はcompact audit recordを保持する。

current implementation authorityは`docs/specs/**` / GUI canonical specであり、`Applied` artifactではない。

## Lifecycle

`Draft -> Proposed -> Approved -> Applied` または `Rejected`

1. `refine-spec`がsource evidence、proposed delta、compatibility、Open Questionsを作る。
2. `review-spec`がsemantic readiness、Human gate、autonomous approval eligibilityを確認する。
3. 有効なapproval後、deltaをcanonical ownerへatomicにapplyする。
4. artifactを`Applied`へ移し、canonical owner / Requirement IDとapproval/application provenanceを残す。
5. Applied後は[Documentation Policy](../contributing/documentation-policy.md#specification-change-retention)に従いcompact audit recordへ縮退してよい。詳細Proposal / ReviewはGit historyが保持する。

## Applied record format

Applied artifactは原則として次だけを保持する。

- `Why`: 採用したchangeの短い理由 / decision
- `Canonical result`: canonical ownerとRequirement ID
- `Approval / application`: HumanまたはAgent-autonomous provenance、application commit等

canonical requirement本文、長いreview transcript、implementation planをcopyし続けない。

## Historical packages

- Desktop制作v1 P1–P3: [0016](0016-desktop-daily-editing.md), [0017](0017-desktop-workspace-settings.md), [0018](0018-desktop-build-delivery.md)
- P4: [0019](0019-existing-record-key-edit.md), [0020](0020-source-file-rename-move.md)

新規artifactには[_template.md](_template.md)を使い、monotonicなnumberを再利用しない。
