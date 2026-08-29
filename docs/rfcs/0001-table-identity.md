# RFC: Table identityとgenerated C# name

Status: Accepted

## 背景（Context）

初期schema exampleには `table: item` と `tableId: catalog.item` の両方が含まれていた。一方、Rust
implementationはschemaとdataのassociationに `table` を使用し、C# scaffoldはgenerated type nameに
`csharpName` を使用していた。2つ目のidentityはmodel、validation、generatorでconsumeされていないため、
exampleに残すと、indexとcompatibilityを実装する前にambiguityを生む。

## 課題（Problem）

将来のworkには、schema/data associationのためのstable table identityと、rename可能なgenerated C# presentation
nameが必要である。identityはfileまたはdirectory pathから推論してはならず、global namespaceまたは
migration promiseを黙って導入しない。

## 目標（Goals）

- current scaffoldのidentity boundaryを明示する。
- `table` とgenerated `csharpName` の目的を分離する。
- 後続proposalが必要性を示した場合に、future project/global identityの余地を残す。

## 非目標（Non-Goals）

このRFCは、index identity、MessagePack compatibility、table rename、migration、generated API naming policy、
global identity formatを決めない。

## 選択肢（Options）

1. `table` をproject-localなstable identityとし、`csharpName` をgenerated C# type-name overrideとする。
2. 別のrequired `tableId` を維持し、そのためのmigration/namespace modelを定義する。
3. source pathまたはgenerated C# nameからidentityを導出する。

## トレードオフ（Trade-offs）

Option 1はcurrent core behaviorと一致し、未使用のduplicate conceptを取り除くが、projectがglobalにstableな
identityを必要とする場合は後のmigrationが必要になる可能性がある。Option 2は区別を明示するが、semanticsが
必要になる前に2つ目のvalueを追加する。Option 3は既存のfile location boundaryと衝突し、moveまたはrenameを
domain changeにする。

## 提案（Proposal）

current scaffoldでは `table` をproject-localなstable table identityとし、`csharpName` をrename可能なgenerated
C# presentation nameとして扱い、未使用の `tableId` fieldをtyped schema modelとfixturesから削除する。
`table`/`csharpName` の区別はterminologyとschema languageに記録する。このrepository hardening taskで記録された
明示的なhuman decisionは、current scaffoldについてこの方向を受け入れる。このacceptanceはglobal identity、
rename migration、released-schema compatibility、legacy `tableId` migration、cross-project identityを決めない。

## 互換性（Compatibility）

このscaffoldにはreleased schema formatがない。未使用でignoredだったmodel fieldを削除することはcurrent fixtureの
source-shape cleanupであるが、released parserでlegacy `tableId` inputをacceptまたはrejectするかはOpen Questionの
ままである。global identityとrename migrationはここでは定義しない。

## Open Questions（未解決事項）

- projectは将来globally stableなtable identityを必要とするか。
- 必要な場合、それは `table` と同じvalueか、それとも別にversion管理するidentityか。
- `table` が変更された場合、どのcompatibilityとmigration behaviorを適用するか。
- parserはlegacy `tableId` をrejectするか、non-semantic metadataとして保持するか、migration window中にサポートするか。

## 決定（Decision）

repository hardening taskにおける明示的なhuman decisionにより、current scaffoldについてAcceptedとする。
canonical Draft specificationは採用された `table`/`csharpName` の区別を記録し、このRFCはrationaleとoption comparison
として残す。RFCの `Accepted` はproduct specificationの `Approved` と同じlifecycle stateではなく、未解決の
compatibility questionは引き続きOpenである。
