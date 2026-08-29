# Field identity仕様（Field identity）

Status: Draft

この文書は、refinementと明示的なapprovalがまだ必要なfield-identity ruleを切り出して管理する。
既存scaffoldのbehaviorはreview対象のevidenceであり、これらDraft requirementのauthorityではない。

### COMPAT-FIELD-001

TableまたはCustom Typeのpersisted fieldはnumeric Field IDを持ち、各container内でuniqueでなければならない（MUST）。
Field IDはfield nameから独立し、table、各Custom Type、およびMasterMemoryのindex numberとは別のnamespaceとidentityを
持たなければならない（MUST）。Field IDを将来MessagePack integer keyへ使用することは想定するが、exact wire shapeはこの
documentでは定義しない。

### COMPAT-FIELD-002

Tableまたは同じCustom Typeで一度使用されたField IDは、active fieldの削除後も同じcontainer内で再利用してはならない
（MUST NOT）。

### COMPAT-FIELD-003

削除したtable field IDは、former name/type informationを持つ `reservedFields` tombstoneとして保持してもよい（MAY）。
Custom Typeで削除したField IDを保持するtombstoneのexact representationは、まだ定義しない。

### COMPAT-FIELD-004

Field nameのrenameだけではField IDを変更してはならない（MUST NOT）。同じcontainer内でfield identityを維持するrenameは、
既存のField IDを保持しなければならない（MUST）。

## 非規範のproduct方向性（Non-normative）

GUIは通常のedit中にstable-ID bookkeepingを意識させないことが期待される。一方でadvanced userと
review toolingからは、persisted IDを確認できることが期待される。正確なallocation、visibility、migration、
wire-compatibilityのbehaviorは、まだ仕様化されていない。

## Open Questions（未解決事項）

- すべてのpersisted schema fieldでnumeric MessagePack key generationを必須にするか。
- 新しいfield IDの正確なallocation policyは何か。
- 必須とoptionalのtombstone metadataは何か。
- nullability、default value、type change、custom-type evolutionはcompatibilityにどう影響するか。
