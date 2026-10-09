# 仕様変更: Table declaration authoring

Status: Applied

## 採用したdecision

Approved Unified Tableが要求するadvanced MessagePack key / Key / Reference authoringを独立operation familyとして具体化した。Migration v1のCLI operation setとordinary schema draftを変えず、existing declaration semantics / source preservation / native structural safetyを共有する。これはHuman-selected Clean-room Rewrite内のAgent Decisionであり、Humanの新semantic decisionとして扱わない。

Canonical owner: [Table Declaration Mutation](../specs/table-declaration-mutation.md)、`TABLE-DECL-001..006`。

## Approval / application

Approval mode: Agent-autonomous。Basis: Clean-room Rewrite / Approved `GUI-UNIFIED-004..005`。別passのreview-specでBlocking / material ambiguityなし、Approved as Proposed: Yes、Eligible: Yes、Human gate: None。

Canonical application: `3d54825e60720bdef40bed6525e9e48ffd695185`。詳細proposal、compatibility / acceptance、review記録は同commitの履歴に残す。現在のimplementation authorityは上記canonical ownerである。
