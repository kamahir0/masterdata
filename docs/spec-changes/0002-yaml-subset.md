# 仕様変更: Masterdata YAML subsetのprimitive scalar境界（Specification change）

Status: Applied

<!-- Lifecycle: Draft -> Proposed -> Approved -> Applied、またはRejected。canonical
     specificationを変更する前にhuman approvalが必要である。 -->

## Affected Specifications

- [`docs/specs/type-system/primitives.md`](../specs/type-system/primitives.md)、`TYPE-PRIMITIVE-003` および
  `TYPE-PRIMITIVE-007`。
- [`docs/specs/yaml-subset.md`](../specs/yaml-subset.md)、`YAML-SUBSET-009` から `YAML-SUBSET-014`。

## 根拠と分類（Source Evidence and Classification）

Record Tags、Build Profiles / Build Selection、およびMasterdata YAML subsetに関する人間のDecisionを、既存の
canonical specificationへ整理する作業である。YAML subsetの新しいdocumentは、parser/libraryのdefault behaviorから独立した
boolean、null、integer、floating-point、stringのsource classificationを提案する。

既存のApproved Primitive Types仕様は、target typeのstrict scalar validation、fixed-width range、finite floating-point valueを
すでに定義している。Masterdata YAML subset proposalは、source側のscalar classificationとsyntax boundaryを定義し、YAML 1.2.2を
normative syntax reference baselineとしている。このartifactはそのApproved documentを直接編集せず、YAML subsetが承認された場合にのみ、
source boundaryをPrimitive Typesのcanonical ownerへ接続するdeltaを記録する。YAML subsetが未承認の間は、既存Approved contractを変更しない。

## 提案する差分（Proposed Delta）

`Masterdata YAML subset仕様`がApprovedになった後、Primitive Types仕様について次のdeltaをatomicに適用する。

1. `TYPE-PRIMITIVE-003` のparser scalar classificationとのboundary referenceを、`YAML-SUBSET-009` から
   `YAML-SUBSET-014` が定めるMasterdata subsetへ接続する。target primitiveのcategory一致、representable value、range、strict
   validation、およびimplicit coercion禁止はPrimitive Types仕様が所有し、source scalarのsubset classificationはYAML subset仕様が
   所有する。
2. `TYPE-PRIMITIVE-007` のfinite-only ruleを維持し、Masterdata YAML subsetが `NaN`、`Infinity`、`-Infinity` をunsupportedとすることを
   cross-referenceする。finite-onlyのtype-system semanticsはPrimitive Types仕様のownerのままとする。
3. Primitive Types仕様のOpen Questionsから、YAML subsetで解決されたboolean、null、ordinary base-10 integer、floating-point lexical
   form、およびnon-finite tokenのparser-boundary questionを削除またはYAML subsetへのreferenceへ置き換える。timestamp-looking plain
   scalar、将来のPrimitive Type、diagnostic/source-span、未選択parser/library policyに関するquestionは解決しない。

このdeltaは新しいPrimitive Type、implicit conversion、numeric range、key compatibility、comparison capability、またはparser
library implementationを追加しない。

## 互換性（Compatibility）

delta適用後、同じsource textに対するscalar classificationとunsupported syntaxの扱いが明確になるため、既存parser defaultに依存した
入力の受理結果が変わる可能性がある。target primitiveのfixed-width range、finite-only value domain、empty string、key capability、
comparison semanticsは変更しない。timestamp-looking plain scalar、Date/DateTime、released schema migrationは引き続き未解決である。

YAML subsetがApprovedになる前にこのdeltaをAppliedまたはimplementation authorityとして扱ってはならない（MUST NOT）。

## 受け入れと実装への影響（Acceptance and Implementation Impact）

delta適用後、Primitive Typesのvalidation evidenceはYAML subsetのscalar categoryをsource boundaryとして扱い、次を確認できなければ
ならない。

- `true`、`false`、`null`、ordinary base-10 integer、defined floating-point form、quoted/plain stringのclassificationがsubsetと一致する。
- `1`をfloat/doubleへ、`1.0`をintegerへimplicit coerceしない。
- unsupportedなnumeric-looking token、`~`、`NaN`、`Infinity`、`+Infinity`、および`-Infinity`を通常のstringまたはfinite valueとして受理しない。
- `TYPE-PRIMITIVE-003`、`TYPE-PRIMITIVE-007`、および対応するYAML subset requirementが同じ実装境界を参照する。

このartifact自体はparser、validator、fixtures、またはproduction implementationを変更しない。implementationはdeltaがApprovedかつ
Appliedになった後に、canonical merged specificationから開始する。

## 未解決事項（Open Questions）

- timestamp-looking plain scalar、diagnostic/source-span、およびparser/libraryの選択・migration・maintenance policyの扱い。

## レビュー（Review）

このdeltaはApproved Primitive Types仕様を直接編集せず、YAML subsetのhuman approval後にのみ適用する境界を明示していた。Human approval後、
deltaはcanonical Primitive Types仕様へatomicに適用済みである。YAML subsetのtimestamp-looking scalar、diagnostic/source-span、および
parser/library policyは意図的に未解決であり、このartifactで先取りしない。YAML 1.2.2のsyntax reference baselineと、subsetが定義する
literal blockの制限を未解決事項として扱わない。Human approval前のcanonical mergeおよびimplementation authority化は行わない。

## 承認記録（Approval Record）

このtaskでhuman maintainerが、現行のMasterdata YAML subset semanticsと、このartifactが定めるPrimitive Typesとの接続deltaを明示的に
承認した。deltaは `docs/specs/yaml-subset.md` のApproved化後、`docs/specs/type-system/primitives.md` へatomicに適用済みである。
このartifactは承認済みdeltaのaudit recordとして保持し、`Status: Applied` とする。
