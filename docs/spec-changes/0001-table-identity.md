# 仕様変更: 現在のtable identity boundaryを採用（Specification change）

Status: Applied

## Affected Specifications

- [`docs/specs/compatibility/table-identity.md`](../specs/compatibility/table-identity.md),
  `COMPAT-TABLE-001`。
- [`docs/specs/schema-language.md`](../specs/schema-language.md)、current scaffoldのidentity explanation。
- [`docs/product/terminology.md`](../product/terminology.md)、table identityのglossary entry。

## Source Evidence and Classification（source evidenceと分類）

repository-hardening taskでは、current-scaffoldの方向を明示的に決定した。`table` はproject-localなstable
logical table identity、`csharpName` はgenerated C# type-name override/presentation name、`tableId` は
現段階では不要である。これは既存implementationからのinferenceではなく、人間の `Decision` である。

Global table identity、table rename migration、released-schema compatibility、legacy `tableId` migration、
cross-project identityは `Open Question` のままである。

## Proposed Delta（提案delta）

current scaffoldでは、canonical specificationに採用済みの `table` / `csharpName` の区別を記録し、2つ目の
`tableId` identityを定義しない。未解決のcompatibility questionをrequirementへ昇格させない。

## 互換性（Compatibility）

current repositoryにはreleased schema formatがない。採用した方向はmigration promiseもglobal identity
namespaceも確立しない。

## Acceptance and Implementation Impact（受け入れとimplementation impact）

typed Rust schema model、fixtures、terminology、generated C# identity commentは、canonical directionと一致
しなければならない。既存testはsource locationがtable identityを決めないことを確認する。このchangeは
index、reference、type resolution、rename migrationを実装しない。

## Open Questions（未解決事項）

- Projectは将来globally stableなtable identityを必要とするか。
- `table` が変更された場合、どのmigration behaviorを適用するか。
- legacy `tableId` inputをrejectするか、non-semantic metadataとして保持するか、migration window中にsupportするか。
- cross-project table identityは必要か。

## レビュー（Review）

current-scaffold decisionに対する `review-spec` のconcernは解消された。残りのcompatibility questionは上記に
意図的に残している。

## 承認記録（Approval Record）

repository-hardening taskでのhuman decisionがcurrent-scaffold directionを明示的に承認した。deltaはcanonical
documentationへatomicに適用され、RFC 0001は `Accepted` へ移された。このartifactはdurable audit recordとして
保持し、`Applied` とする。
