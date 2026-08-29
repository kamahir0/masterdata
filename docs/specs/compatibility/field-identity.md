# Field identity仕様（Field identity）

Status: Draft

この文書は、refinementと明示的なapprovalがまだ必要なfield-identity ruleを切り出して管理する。
既存scaffoldのbehaviorはreview対象のevidenceであり、これらDraft requirementのauthorityではない。

### COMPAT-FIELD-001

Field IDはMessagePack integer keyに使用することを想定し、table内でuniqueでなければならない（MUST）。

### COMPAT-FIELD-002

Active field IDは削除後に再利用してはならない（MUST NOT）。

### COMPAT-FIELD-003

削除したIDは、former name/type informationを持つ `reservedFields` tombstoneとして保持してもよい（MAY）。

## 非規範のproduct方向性（Non-normative）

GUIは通常のedit中にstable-ID bookkeepingを意識させないことが期待される。一方でadvanced userと
review toolingからは、persisted IDを確認できることが期待される。正確なallocation、visibility、migration、
wire-compatibilityのbehaviorは、まだ仕様化されていない。

## Open Questions（未解決事項）

- すべてのpersisted schema fieldでnumeric MessagePack key generationを必須にするか。
- 新しいfield IDの正確なallocation policyは何か。
- 再利用を永久に禁止するのか、それとも定義したcompatibility window内だけ禁止するのか。
- 必須とoptionalのtombstone metadataは何か。
- nullability、default value、type change、custom-type evolutionはcompatibilityにどう影響するか。
