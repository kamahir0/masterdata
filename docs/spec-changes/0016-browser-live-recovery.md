# Browserの復旧後も未保存入力を継続する

Status: Applied

Basis: Browser Objective内のAgent Decision。実装reviewで、live sessionの復旧後に保存基準を無条件に無効化すると、保持した入力を再保存する道がなくなることを確認。

## 提案

- 同じProjectのメモリdraftがある場合は「保存前へ復旧」を提供し、復旧成功後もdraft / Undoと元の保存基準を維持して再保存できるようにする。各次回保存は通常どおり全変更対象のディスク内容を比較するため、古い基準を無検査で信用しない。別writerのjournal等で基準が一致しない場合は通常の外部変更として停止する。
- 「保存済み内容を採用」は、そのProjectのメモリsessionがない場合に限る。メモリ入力のある状態でdiskだけを採用してdirtyを誤って解除しない。
- 復旧dialogは閉じて既存draftの編集へ戻れる。未解決journalの警告と再表示入口は残し、影響するProjectへの通常保存は引き続き拒否する。破損journalや別Projectのopen失敗が現在の入力へのアクセスを妨げない。
- acceptance: liveの中断保存→復旧→同じdraftを再保存、外部の第三の内容の保護、dialogを閉じた後の入力保持・再表示をGUIで確認する。

Canonical owner: [Browser Authoring](../specs/browser.md)。Desktop / CLI、権限範囲、初回読込後の通常編集I/Oなしは変更しない。

## Review

別pass challenge: 元の保存基準維持は次回の全対象preflightを省略しないことを確認。復旧と保存ackを混同せずdirty / Undoを保持し、復旧dialog退出でもjournalを消さない。Blocking Issues / Non-blocking Issues / Questions: None。Approved as Proposed: Yes。Autonomous approval eligible: Yes（選択済みObjectiveの保存・回復UXの修正、追加権限・外部作用なし）。Human gate: None。

Approval mode: Agent-autonomous
Objective / request basis: 2026-10-22 Browser実装Objective。別pass判定後orchestrationが承認しcanonicalへ適用。
