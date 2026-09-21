# Development State

Stage: decision-required
Candidate: none
Work base: 7bcef041c982c4cf07910d68e22408221ec41b5e

## Active work

In progress: Reference v1のcanonical semanticsをrefineし、既存Table/Key identityとMasterMemory query APIに整合する仕様変更0023 Draftを作成。
Remaining: Human decision後のspec review / application、shared core validation、C# helper、Table Editor authoring、verification。

## Blocking findings

None.

## Human decision needed

Referenceのtarget identityは既存Approved contractから `table + ordered target fields` に一意化できるが、persisted source surfaceとgenerated public helper APIには複数の合理的choiceが残る。

推奨は仕様変更0023のOption A: schema-level `references`、explicit relation `name`、ordered source `fields`、target `table + fields`、v1はRequired scalar sourceのみ、generated `Get<Name>(MemoryDatabase database)` helper。composite keyを自然に扱え、nullableのpartial-null policyを先送りせずv1 scopeから明示除外できる。

Option Bは同じsurfaceでNullable Referenceもv1に含め、全component null=参照なし、全component non-null=lookup、partial null=validation errorとする。便利だがsource/runtime/codegen semanticsが広がる。

Option Cはbuild-time integrityだけを先行しgenerated helperを後続にするが、既存REF-003の方向とReference v1の利用価値を分断するため非推奨。
