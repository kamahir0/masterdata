# Project Config Edit仕様

Status: Approved

Domain: Project Configuration

この仕様はDesktop制作v1のProject Settingsから`masterdata.toml`をsource-preservingに編集するcontractを定義する。Project config shapeは[Project layout](project-layout.md)、Build Profile semanticsは[Build Selection](build-selection.md)、publish target semanticsは[Build pipeline](build-pipeline.md)が所有する。適用記録は[仕様変更0017](../spec-changes/0017-desktop-workspace-settings.md)を参照する。

## 規範要件

### CONFIG-EDIT-001

設定editorはexisting `masterdata.toml`のexact base bytes、project binding、config buffer revisionを使用し、次のtyped operationだけをv1で提供しなければならない（MUST）。

- named Profile追加、existing Profileのinclude_tags / exclude_tags更新。
- Publish target追加、existing targetのpath更新。existing targetはconfig snapshot内のarray occurrenceで指定する。

Profile rename/delete、target remove/reorder/kind変更、project id / roots / artifact path等のgeneral config編集はv1外。target追加時のkindは`csharp`または`binary`を明示入力する。source rootやdirectoryからkindを推測してはならない（MUST NOT）。

### CONFIG-EDIT-002

shared layerはTOML syntaxと編集targetをlosslessに定位し、対象propertyと必要syntaxだけをpatchしなければならない（MUST）。unrelated section、unknown section、comment、quote、newline、array要素順をpreserveする。config全体の通常serializer出力へfallbackしてはならない（MUST NOT）。
最低対応は`[build.profiles.<name>]` table内のstring array、`[[publish.targets]]`内のstring path、両方の追加とする。quoted key、multiline array、inline commentを含むvalidな最低形を扱う。inline table等の別TOML表現を安全に定位できなければ理由付きunsupportedとし、黙ってnormalizationしない。
既存array更新はentry単位でpreserveする。新entryは末尾、削除entryは必要separatorとともに除去し、外側standalone commentを所有推測で削除しない。明示empty listは`[]`、既存未指定から変更後元に戻した場合は未指定の元bytesへ戻す。新Profile/targetはconfig末尾へ必要な区切り付きで追加し、無関係な既存bytesを変えない。

### CONFIG-EDIT-003

candidateはvalid TOMLと一意なtargetを維持し、変えた入力をlosslessに表現できなければならない（MUST）。malformed TOML、duplicate table/property、target ambiguousはSave candidate failure。範囲を安全に特定できないconfig全体はtyped編集不可とする。
一方tag entryのgrammar、duplicate/overlap tag、既存unknown profile key、target path等のdomain diagnosticだけでSaveを禁止してはならない（MUST NOT）。この区別はsource-invalid状態を保持して直せるためのcontractであり、Project validatorのacceptanceを緩和しない。
新Profile名はBuild Selectionのgrammarを満たし既存名とcollisionしないことをcreate operationのpreconditionとする（MUST）。入力は修正できるformに保持し、silent repairしない。Profile rename/deleteを持たないv1で、修正不能な新しいidentityを生成しないためである。外部source由来のinvalid name / unknown propertyは保持し、本editorで修復対象外ならlocationと理由を示す。
保存済みconfigのdomain-invalidによりProject serviceが利用不能なら、そのstateとconfig editorの修正導線を保持し、古い正常configでBuild等を継続してはならない（MUST NOT）。config editorは既に確立したrootとexact config identityから再取得し、domain-valid Projectの再解決を編集開始の必須条件にしない。権限・root binding・構造の安全性を再確認できなければ停止する。

### CONFIG-EDIT-004

config Saveは一fileの明示commitとし、mutation直前にexact content identityを検査しなければならない（MUST）。mismatchはConflictで無変更、safe I/Oを満たす成功だけSuccessとする。FailureとOutcome Unknownを区別し、未保存入力を保持する。Outcome Unknownはactual bytes取得前にretryしてはならない（MUST NOT）。partial/truncated contentを成功扱いしない。crash/global transaction保証は追加しない。
v1 config ConflictにはCompare / Reload / Cancelを提供し、Overwrite / automatic mergeは提供しない。Reloadによる入力破棄は明示確認する（MUST）。新しいbase上で利用者が再編集してSaveする。
config保存はsource YAML、artifact、target filesystem、Gitを変更してはならない（MUST NOT）。path保存はそのdestinationへのpublish実行認可ではない。

## 受け入れ証拠

quoted table key、multiline array、comment/CRLF、unknown section、existing/new profile、target occurrence、unsupported inline table、domain-invalid保存、外部変更を検証する。
