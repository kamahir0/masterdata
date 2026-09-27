# Field Declaration Mutation

Status: Approved

Table fieldの`type`、`nullable`、`array`宣言を変更する追加operationのsource safety contract。本operationは[Schema Migration v1](schema-migration.md)の`AddField` / `RenameField` / `DropField`の意味を変更しない。Type System、Table/Key、Reference、YAML subsetは各ownerへ委譲する。

### FIELD-DECL-001

入力はlogical Table identity、current field name、次のcomplete field declarationを表す。pathをTable identityとして使ってはならない（MUST NOT）。field name / MessagePack keyの変更は本operationへ混ぜない（MUST NOT）。

### FIELD-DECL-002

shared Coreはcanonical source snapshotと必要なresolution closureからdeterministic Planを作り、新宣言を既存のType System / Table schema規則で検証しなければならない（MUST）。既存record valueを暗黙に変換してはならない（MUST NOT）。inline recordsとすべての分離Data sourceについて対象fieldの各値を新宣言で検証し、妥当性を証明できないvalue、unknown source、unresolved dependencyがあればsource mutation前に拒否する（MUST）。

### FIELD-DECL-003

KeyまたはReferenceのcomponentへ影響する変更は、それらの整合性をshared semanticsで証明できる場合だけ許可する（MUST）。証明できない場合は依存名を示して拒否する。既存fieldのkey、name、record value、他field、他sourceを暗黙に変更してはならない（MUST NOT）。

### FIELD-DECL-004

source-preservingな最小patchをin-memoryで適用し、canonical parserによる再parseと期待semantic結果とのpostcondition比較を行わなければならない（MUST）。source commitは[Schema Migration v1](schema-migration.md)のlost-update、multi-file rollback、Recovery Requiredと同じ安全境界を使用する。commit failureまたはtransport outcome不明をSuccessとしてはならない（MUST NOT）。

### FIELD-DECL-005

GUIの高水準intentは正常時に内部Planをcommitしてよい（MAY）。affected sourceのdirty bufferを上書きせず、stale時にsilent retryしない。異常時はtarget field、affected source / record、理由を返す。GUIはsource formatと型規則を再実装してはならない（MUST NOT）。

## Compatibility

既存YAML shape、logical Table identity、Migration v1のcommand semantics、CLI grammarは維持する。valuesのcoercionやarray wrapping、null埋めは本operationの範囲外。

## Evidence

empty Table、inline records、複数Data source、scalar/nullable/array値、Key/Reference依存、コメント/quote保持、stale commit、rollbackをfocused testで確認する。
