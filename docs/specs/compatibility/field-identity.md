# Field identity仕様（Field identity）

Status: Approved

この文書は、Custom Typeを含むfield-identity ruleをcanonicalな形で管理する。既存scaffoldのbehaviorはreview対象のevidenceで
あって、これらApproved requirementの代替ではない。

### COMPAT-FIELD-001

TableまたはCustom Typeのpersisted fieldはnumeric Field IDを持ち、各container内でuniqueでなければならない（MUST）。
active field IDとreserved field IDは、その同じcontainerに属する1つのused-ID namespaceを共有しなければならない（MUST）。
したがって、containerのused ID setはactive IDとreserved IDのunionであり、active IDとreserved IDは互いにcollisionしては
ならない（MUST NOT）。Field IDはfield nameから独立し、table、各Custom Type、およびMasterMemoryのindex numberとは別の
namespaceとidentityを持たなければならない（MUST）。Field IDを将来MessagePack integer keyへ使用することは想定するが、
exact wire shapeはこのdocumentでは定義しない。

### COMPAT-FIELD-002

Tableまたは同じCustom Typeで一度used ID setに入ったField IDは、active fieldの削除後も同じcontainer内で再利用しては
ならない（MUST NOT）。active fieldとして再宣言する前にreserved identityを削除することも、IDの再利用として扱う。

### COMPAT-FIELD-003

persisted fieldを削除した場合、そのField IDは同じcontainerのused-ID namespace内でreserved field identityとして保持しなければ
ならない（MUST）。Tableではtable schemaのtop-level `reservedFields`、Custom TypeではCustom Type schemaの
`custom.reservedFields`をcanonicalな保持場所として使用する。Custom Typeの各reserved entryは、少なくとも `id`、
`formerName`、`formerType` を保持しなければならない（MUST）。このdocumentは、Custom Typeのminimum entryを超える
deletion timestamp、reason、replacement、migration、serialization metadataを要求しない。

### COMPAT-FIELD-004

Field nameのrenameだけではField IDを変更してはならない（MUST NOT）。同じcontainer内でfield identityを維持するrenameは、
既存のField IDを保持しなければならない（MUST）。

## 非規範のproduct方向性（Non-normative）

GUIは通常のedit中にstable-ID bookkeepingを意識させないことが期待される。一方でadvanced userと
review toolingからは、persisted IDを確認できることが期待される。正確なallocation、visibility、migration、
wire-compatibilityのbehaviorは、まだ仕様化されていない。

## 検証ルール

TableまたはCustom Typeの各containerについて、active IDとreserved IDのunionをused ID setとして検査する。active/reserved
collision、used IDの再利用、rename時のID変更を、このdocumentのrequirementに対する個別のvalidation outcomeとして扱う。
Custom Typeのreserved entryのcanonical YAML shapeは[Custom Type仕様](../type-system/custom-types.md)が所有する。

## 受け入れ証拠

| Requirement（要件） | Success observation（成功時の観測） | Failure observation（失敗時の観測） |
| --- | --- | --- |
| `COMPAT-FIELD-001` | container内のactive IDとreserved IDがuniqueで、used ID setに重複がない。table、各Custom Type、index numberのnamespaceが分離される。 | active/reserved collision、同一container内の重複、または別namespaceのIDを同一IDとして扱う。 |
| `COMPAT-FIELD-002` | 一度used ID setに入ったIDが、削除後も同じcontainerのreserved identityとして占有される。 | reserved identityを削除して同じIDを新しいactive fieldへ割り当てる。 |
| `COMPAT-FIELD-003` | 削除したTable fieldはtop-level `reservedFields`、削除したCustom Type fieldは `custom.reservedFields` に保持される。Custom Type entryは `id`、`formerName`、`formerType` を持つ。 | 削除IDがactive/reserved namespaceから消える、Custom Type minimum memberが欠落する、または別containerのreserved entryへ移される。 |
| `COMPAT-FIELD-004` | field nameをrenameしても同じactive Field IDが維持され、reserved identityへ移らない。 | renameだけでField IDが変わる、またはactive fieldがreservedになる。 |

## Open Questions（未解決事項）

- すべてのpersisted schema fieldでnumeric MessagePack key generationを必須にするか。
- 新しいfield IDの正確なallocation policyは何か。
- Custom Typeのminimum `reservedFields` entryを超えて、deletion timestamp、reason、replacement、migration、serialization metadataを保持するか。
- nullability、default value、type change、custom-type evolutionはcompatibilityにどう影響するか。
