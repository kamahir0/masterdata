# Development State

Stage: decision-required
Candidate: none
Work base: b17ce4ee0c2186edbaf69c56a031355312601ee4

## Active work

In progress: Released Compatibility v1のscopeをrefineし、current-schema semantics、generated C# API、coherent artifact/binary、external long-lived contractの境界を仕様変更0024 Draftへ整理。
Remaining: Human decision後のcanonical specification application、shared compatibility analyzer、必要なproduct surface、verification。

## Blocking findings

None.

## Human decision needed

Released Compatibility v1で「何に対する互換性を保証・判定するか」はmaterial product choiceであり、既存authorityから一意に決まらない。

推奨は仕様変更0024のOption A: explicitなold/new canonical source snapshotを比較し、generated C# API compatibilityとMasterdata source/migration impactをmulti-axisで報告する。MasterMemory cross-schema binary compatibility、save/network/database等のexternal wire contractはv1非対象とし、artifact receiptやMessagePack keyをrelease identityへ昇格させない。

Option BはOption Aに加えてcross-schema MasterMemory binary compatibilityまで保証対象にする。既存のcoherent artifact-set modelを越えるbinary/version contractが必要になりscopeが大きい。

Option Cは比較機能より先にpersistent release manifest / schema version identity / stable member IDsを導入する。長期versioning基盤にはなり得るが、現在retire済みField ID等を別形で再導入する危険があり最も侵襲的。
