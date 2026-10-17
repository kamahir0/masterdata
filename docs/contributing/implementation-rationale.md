# Implementation Rationale

Status: Active

straightforwardな実装から外れ、理由を知らない将来の変更で不具合を再発させる箇所だけ、近くにWHYを残す。

対象の例はworkaround、ordering / concurrency、意図的な状態の複製、特殊なエラー処理、計測に基づく最適化。通常のbranchやhelperごとにcommentを要求しない。

必要な範囲で、理由、簡略化すると壊れるbehavior、根拠となるtest / spec / 計測、temporaryな場合の撤去条件を説明する。参照番号だけで理由の代わりにしない。

関連するcode、invariant、依存関係、根拠を変えた際は、その理由も再確認する。現在も正しければ保持し、条件が変われば更新し、理由が消えたなら削除する。test passだけでは理由の鮮度を証明できない。

behaviorは仕様、横断的な設計理由はADR、回帰の証拠はtestが所有する。既に十分に復元できる理由を重複して書かない。独自のcomment DSLや、そのためだけの複雑なparserは作らない。
