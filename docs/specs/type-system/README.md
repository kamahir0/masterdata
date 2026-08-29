# Type System仕様（Type-system）

Type-system ruleは、独立してreviewできるcanonical documentへ分割している。これにより、
Primitive、Field Modifier、Value Object、Enum/Flags、Custom Typeのbehaviorを、1つの
document-wide statusに縛られずDraft、Proposed、Approved、Implementedへ進められる。この
directory indexはnon-normativeであり、要件を所有するのはlink先のdocumentである。

- [Primitive Types仕様](primitives.md) — `Status: Proposed`
- [Field Modifiers仕様](field-modifiers.md) — `Status: Proposed`
- [Value Objects仕様](value-objects.md) — `Status: Proposed`
- [Enums and Flags仕様](enums.md) — `Status: Draft`
- [Custom Types仕様](custom-types.md) — `Status: Draft`

これらのfileでは既存のRequirement IDを保持する。documentを分割してもIDをrenameまたは
reassignしない。
