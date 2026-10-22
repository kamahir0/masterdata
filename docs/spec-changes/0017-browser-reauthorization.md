# Browserのフォルダ再許可

Status: Applied

Basis: Browser Objective内のAgent Decision。macOS / Chrome実機で選択folderのアクセス削除→Save時の再許可を拒否すると、以後のrequestPermissionはpromptを出さずdeniedを返すことを確認。未保存入力は保持できるが、再Saveへ戻る明示的な入口が必要。

## 提案

- Browserの保存方法menuと復旧dialogに「フォルダのアクセスを再許可」を設ける。ユーザー操作でDirectory Pickerを開き、現在のProjectまたは復旧対象と`isSameEntry`で同じfolderであることを確認する。別folder・取消・拒否・失敗は現在のdraft / Undo / 保存基準を保持する。
- 再許可では新Projectとしてloadせずhandleだけを更新し、ファイルの読書きや保存ackを行わない。再Save / 復旧は別の明示操作として通常のpreflightを通す。
- permission拒否の案内からこの入口が分かるようにする。権限範囲は元の選択folderと同一に限定する。
- acceptance: 権限取消→拒否→再保存失敗でもdisk不変 / dirty維持、明示再選択→同じfolderへの再許可→元の入力を保存できる。取消・別folderの選択は置換 / 保存しない。実Chromeのnative permission UIで確認する。

Canonical owner: [Browser Authoring](../specs/browser.md)。Desktop / CLI、Build / Publish非提供、保存protocolは変えない。

## Review

別passでchallenge。明示user activation、同じhandle identityの検査、取消 / 異なるfolder / 拒否時の入力保持、保存基準を再読込しないことと次Saveの競合検査を確認。要求権限は既存authorizationと同じfolderであり境界拡張なし。Blocking / Non-blocking Issues / Questions: None。Approved as Proposed: Yes。Autonomous approval eligible: Yes。Human gate: None。

Approval mode: Agent-autonomous
Objective / request basis: 2026-10-22のBrowser実用化・保存安全性検証の委任。別pass判定後にorchestrationが承認しcanonicalへ適用。
