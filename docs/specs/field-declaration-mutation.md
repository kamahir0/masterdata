# Field Declaration Mutation

Status: Approved

Table fieldの`type`、`nullable`、`array`宣言を変更するsource safety contract。`FIELD-DECL-001..005`は明示的なstrict Plan/Migration operation、`FIELD-DECL-006..010`は通常Table authoringのdraftとfile Saveを所有する。前者を後者の通常header操作へ暗黙適用してはならない。Type System、Table/Key、Reference、YAML subsetは各ownerへ委譲する。

### FIELD-DECL-001

入力はlogical Table identity、current field name、次のcomplete field declarationを表す。pathをTable identityとして使ってはならない（MUST NOT）。field name / MessagePack keyの変更は本operationへ混ぜない（MUST NOT）。

### FIELD-DECL-002

shared Coreはcanonical source snapshotと必要なresolution closureからdeterministic Planを作り、新宣言を既存のType System / Table schema規則で検証しなければならない（MUST）。既存record valueを暗黙に変換してはならない（MUST NOT）。inline recordsとすべての分離Data sourceについて対象fieldの各値を新宣言で検証し、妥当性を証明できないvalue、unknown source、unresolved dependencyがあればsource mutation前に拒否する（MUST）。

### FIELD-DECL-003

KeyまたはReferenceのcomponentへ影響する変更は、それらの整合性をshared semanticsで証明できる場合だけ許可する（MUST）。証明できない場合は依存名を示して拒否する。既存fieldのkey、name、record value、他field、他sourceを暗黙に変更してはならない（MUST NOT）。

### FIELD-DECL-004

source-preservingな最小patchをin-memoryで適用し、canonical parserによる再parseと期待semantic結果とのpostcondition比較を行わなければならない（MUST）。source commitは[Schema Migration v1](schema-migration.md)のlost-update、multi-file rollback、Recovery Requiredと同じ安全境界を使用する。commit failureまたはtransport outcome不明をSuccessとしてはならない（MUST NOT）。

### FIELD-DECL-005

明示的なstrict Plan operationを使用するcallerは、affected sourceのdirty bufferを上書きせず、stale時にsilent retryしない。異常時はtarget field、affected source / record、理由を返す。GUIはsource formatと型規則を再実装してはならない（MUST NOT）。通常のtype / Nullable / Array header操作は`FIELD-DECL-006..010`へ進む。

### FIELD-DECL-006

通常のtype / Nullable / Array header変更はschema source fileのauthoring draftとして保持しなければならない（MUST）。操作時にdisk mutationまたは全record validityの証明を要求してはならず（MUST NOT）、record source valueを変換・canonicalizeしてはならない（MUST NOT）。draftのtype/modifierは編集面に直ちに表示し、Undo/Redoで未保存の変更を可逆的に戻せなければならない（MUST）。NullableとArrayの同時trueなどsource schemaとして成立しないshapeは、GUIで単一操作として有効な宣言へ切り替える。

### FIELD-DECL-007

shared Coreはschema draftとすべてのinline / 分離record sourceのcurrent draftをcomposeし、`YAML-SUBSET-018`のSourceValueをnew declarationで解釈しなければならない（MUST）。解釈不能なrecord valueまたはkey/reference dependencyは、操作失敗ではなくsource occurrence / field / nested value pathに対応するdiagnosticにしなければならない（MUST）。schemaを元へ戻せば、sourceを変更せずに再解釈し、原因が消えたdiagnosticを消さなければならない（MUST）。GUIとCLI Validate / Build / Migrationはshared interpreterの意味を分岐させてはならない（MUST NOT）。

### FIELD-DECL-008

schema draftのsource mutation targetはschema physical source fileであり、record source textを解釈やdiagnosticだけを理由に変換してはならない（MUST NOT）。Unified Tableの通常Save commandが同時に選択record sourceのactual dirty candidateを対象にする場合も、file別のsource-preserving candidate、base exact content identity、commit resultを保持しなければならない（MUST）。inline recordsとschema declarationが同じfileで両方dirtyなら、一つのcandidateにcomposeして一回だけcommitする（MUST）。source locationを安全に再特定できない、candidateがsubset syntax/structural shapeと期待patch postconditionを満たさない、I/O failure、Outcome Unknown、Recovery RequiredではSuccessを返してはならない（MUST NOT）。write safetyと失敗後のbuffer保持は`SOURCE-EDIT-005..012`と同等の境界を維持する。

### FIELD-DECL-009

schema draftのSave可否をrecord/domain validation errorの有無へ依存させてはならない（MUST NOT）。Save successはsemantic validityやBuildabilityを意味しない。Validateはpersisted sourceまたは明示されたin-memory draftに対するdiagnosticを返し、Buildは既存のprofile-independent / selected-dataset validationに必要なresolved modelがinvalidなら失敗しなければならない（MUST）。SaveだけでBuild/Publish/Gitを実行してはならない（MUST NOT）。

### FIELD-DECL-010

schemaと各separate record sourceはphysical fileごとに独立したbase identity・dirty state・historyを持たなければならない（MUST）。schema declarationsとinline recordsは同一schema fileのbase identity・dirty lifecycleを共有する。通常Saveで複数fileを対象にしても成功fileだけをnew baseへ進め、他fileのdraftを保持してdiagnosticを再評価する。別fileのexternal changeはそのfileのdirty bufferを黙って上書き・rebaseしてはならず（MUST NOT）、Conflict/Failure/Outcome Unknownの区別とRecovery Required gateを維持する。Save AllはProject-wideな明示的上位workflowであり、途中失敗を全成功と報告してはならない（MUST NOT）。

### FIELD-DECL-011

Table field declarationの順序変更はschema physical sourceの可逆的authoring draftとして保持し、Saveまでdiskへ書いてはならない（MUST NOT）。field identity、name、type、modifier、MessagePack key、PK/SK、Reference、record valueを順序変更だけで変えてはならない（MUST NOT）。source-preserving candidateとsemantic postconditionをshared Coreが検証し、inline record draftがあれば同じphysical candidateへcomposeする。no-opは履歴を増やさない。

## Compatibility

logical Table identity、Migration v1の明示Plan operation、CLI grammarは維持する。通常header authoringは即時Migrationからdraft Saveへ移り、既存のinvalid valueで操作が拒否されずdiagnosticになる。record scalarのaccepted setは仕様変更0042に従って変わる。valuesの暗黙変換やarray wrapping、null埋めは行わない。

## Evidence

empty Table、inline records、複数Data source、scalar/nullable/array値、Key/Reference依存、コメント/quote保持、stale commit、rollbackをfocused testで確認する。
