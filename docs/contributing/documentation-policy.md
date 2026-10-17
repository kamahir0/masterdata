# Documentation Policy

Status: Active

目的は、開発者・agentが意図、authority、現在地、判断理由を合理的な時間で復元できること。文書量や形式的な網羅性自体を最適化しない。

## One knowledge, one owner

追加前に既存ownerで十分に復元できるか確認する。十分なら追加しない。

| 内容 | 置く場所 |
| --- | --- |
| 製品の方向性 | `docs/product/` |
| 今の目的 / 再開地点 | Current Objective / Development State |
| 観察可能なbehavior | `docs/specs/` |
| 承認済みbehaviorの変更提案 | `docs/spec-changes/` |
| 横断的なarchitectureのWHY | `docs/adr/` |
| 具体的な大きい未決定案の比較 | 必要な場合だけ`docs/rfcs/` |
| 回帰behaviorの証拠 | tests |
| 局所的な実装理由 | codeの近接comment |
| 過去の変更 / 詳細review履歴 | Git |

directory、template、報告書を将来用という理由だけで先に増やさない。入口、skills、Objective、stateへcanonical ruleを複製せずlinkする。

## 保持と更新

- Applied提案はWhy、canonical owner、承認と適用のprovenanceだけを残し、詳細はGitへ任せてよい。
- 重要な実測・manual reviewの結果を保持する場合は、対象SHAと環境を示す。historical evidenceを新Candidateの証明として再利用しない。
- 会話全文、tool output、テストlogを恒久文書へ貼らない。実行していない確認を行ったかのように記録しない。
- タスク完了時に新しい証拠要件を増やさない。実質的な欠陥・不足が見つかった場合は具体的な理由を示す。
- 人間向け文書は原則日本語。identifier、API、path、stable status等はそのまま使う。
