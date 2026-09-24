# Development State

Stage: correction-ready
Candidate: 582043fca6282456e319aba279bd85b76da80116
Work base: 6b0d92cbe8c6b73ea514ebf5d061253afdc8c92d

## Active work

- In progress: CandidateのTable Overview切替時の表示修正。
- Remaining: corrective candidateのlocal checks、fresh verification、required remote CI reconciliation。

## Blocking findings

- Blocking: `582043fca6282456e319aba279bd85b76da80116`でOverview表示中にTableを切り替えると、新Tableのrequest完了前に旧Tableのsnapshotが新Table見出しの下へ残る。Table identityごとに表示stateを分離し、回帰テストで確認する。
