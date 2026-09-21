# Development State

Stage: decision-required
Candidate: none
Work base: cba90fb1a7ea5f3d62e25a78e3f83659f57987dd

## Active work

Completed: 仕様変更0023へ2026-09-21 JSTのHuman decision（Option B）を記録し、Reference core semanticsをApproved [Index / Reference](specs/index-and-reference.md)へ適用。
Completed: typed AST、cross-table target resolution、selected dataset integrity、nullable semantics、source-preserving Reference mutation、Table Editor snapshot/Plan surface、Reference dependency fail-closedを実装し、focused testsを通過。
In progress: Reference v1のHuman-gated public C# helper contractを確定待ち。
Remaining: helper method naming、Optional non-unique absence representationのdecision後にC# helper lowering/compile evidenceを実装し、final verificationへ進む。

## Blocking findings

None.

## Human decision needed

Option B自体は承認済みであり、再確認不要。残るdecisionは次の2点だけ。

1. generated helperのpublic method naming。候補例は `GetCategory(MemoryDatabase database)`、`FindCategory(MemoryDatabase database)`、または別の既存命名規則。`Get<Name>`は旧proposalの推奨であり、今回のOption B選択から承認済みとは推定しない。
2. Optional + non-unique helperのabsence representation。actual MasterMemory queryは`RangeView<Target>`を返す。候補は全nullをempty `RangeView<Target>`として返す、`RangeView<Target>?`/wrapperでabsenceを明示する、または別の既存API contractを採る、のいずれか。unique optionalはtarget row nullableが候補となる。

推奨案: helper nameは既存C# generated member naming ownerから明示承認された一つを選び、Optional non-uniqueはMasterMemoryのactual `RangeView<T>`をmaterializeせずempty resultでabsenceを表現する。ただしこれはHuman Approvalなしに適用しない。
