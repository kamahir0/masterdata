# Development State

Stage: correction-ready
Candidate: 43c99fc5884f908d0061c2b615d0f0f968c82aca

## Blocking findings

[Candidate review](evidence/desktop-v1-review.md)のF01–F17と不足検証を修正scopeとする。

- F01/F15: Publish destination plan変更をstaleとして拒否し、target別結果を保持、失敗後の旧Confirmを失効する。
- F02–F05: TOMLの正しいsource span/comment/occurrence identityを保持し、config Overwriteを除去する。
- F06–F08: config file bufferの変更合成・guard・active Save・保存後binding再解決を実装する。
- F09: clipboard形状からPaste対象を決め、選択範囲へのflatten誤配置を防ぐ。
- F10: Table/Typeのtyped initializer、unset/null、型変更時の失効を実装する。
- F11/F12: shared validationに一致するis-invalidとnullable scalar sortを実装する。
- F13/F14/F16: Build snapshot captureと結果帰属、phase別busy、Project切替/close、Recovery時Publish-onlyを契約へ合わせる。
- F17: Undo履歴廃棄前に通知する。
- 上記focused regression、CI timeoutの原因確認、required checks、Desktop実機制作/failure scenarioの証拠を揃える。
