# 仕様変更: Table schemaとrecordsの同居

Status: Applied

## Affected Specifications

- `docs/specs/table-and-keys.md` `SCHEMA-TABLE-001`
- `docs/specs/source-edit.md` `SOURCE-EDIT-001`, `SOURCE-EDIT-002`
- `docs/specs/source-record-mutation.md` `SOURCE-RECORD-001`, `SOURCE-RECORD-012`
- `docs/specs/source-tag-edit.md` `SOURCE-TAG-001`
- `docs/specs/source-creation.md` `SOURCE-CREATE-004`, `SOURCE-CREATE-005`
- `docs/specs/schema-migration.md` `MIGRATION-014`, `MIGRATION-015`
- `docs/gui/data-editor/spec.md` `GUI-DATA-LAYOUT-001`, `GUI-DATA-LAYOUT-003`
- `docs/gui/table-editor/spec.md` `GUI-TABLE-LAYOUT-001`

## 根拠と分類（Source Evidence and Classification）

- Human Requirement: schema YAMLにrecordsをまとめて定義でき、従来の分離形式も引き続き選べること。
- Human Requirement: record YAMLの編集画面からTable schemaを編集できること。Legacy Editorに近い一体的な編集画面とすること。
- Human Decision: 開発期間中なのでversion保守に工数を割かない。
- Agent Decision: `kind: schema`のoptionalなtop-level `records`を同居形式の印とする。省略した既存schemaは分離形式のままとし、混在を許す。明示した`records: []`は同居形式の空Tableである。理由は既存sourceを無変更で解釈でき、file identityだけで編集対象を決定できるため。
- Agent Decision: schema編集は既存Migration / Reference authoring操作を再利用し、recordの未保存変更が影響fileにある時はApplyを止める。理由はsource-preservingとlost-update契約を維持するため。

## 提案する差分（Proposed Delta）

- `SCHEMA-TABLE-001`: schema documentはoptionalな`records` sequenceを持てる。このrecord occurrenceは同じTableのdata documentのrecordsと同じ検証、Build Selection、key/reference制約を受ける。同じTableでinline recordsと0個以上の分離data documentを併用できる。record provenanceは実際のsource fileと、そのfile内のrecord indexを指す。
- `SOURCE-EDIT-001/002`: existing record member value editの対象sourceに、`records`を明示したschema documentを含める。schema declaration自体をrecord保存操作で変更しない。source-preserving patch、stale snapshot、postconditionは分離data documentと同一。`records`のないschema documentをrecord保存対象にしない。
- `SOURCE-RECORD-001/012`と`SOURCE-TAG-001`: record add / deleteと`$tags` editの対象sourceにも、`records`を明示したschema documentを含める。それぞれの独立operationとfile単位commit契約は維持する。
- `SOURCE-CREATE-004/005`: Table作成時に「schemaのみ」または「schemaと空recordsを同居」を選択できる。いずれも1 requestで1 fileだけ生成する。後者は`records: []`を明示し、別data fileは生成しない。Data documentの作成は引き続き可能。
- `MIGRATION-014/015`: schema document自身にrecordsがある場合、Migrationはschema宣言とinline recordsへ同じsource file内で必要なpatchを合成し、1 fileとしてplan、postcondition、atomic commitを扱う。分離data documentにも従来通り適用する。
- `GUI-DATA-LAYOUT-001/003`: record gridは選択中のdata document、または`records`を明示したschema documentのrecordsを表示・編集する。異なるsource fileのrecordsを混ぜない。
- `GUI-TABLE-LAYOUT-001`: record編集画面に対応Table schemaのfield / key / referenceを確認・編集するsectionを設ける。schema編集は既存のPlan / Diff / Applyとdirty file gateを使う。schema fileが選択されinline recordsを持つ場合も、同じ画面で両者に到達できる。`records`のないschemaを選択した場合はschema sectionを表示する。Data Editorの旧non-goal「schema編集」はこの一体化に合わせて除去する。

## 互換性（Compatibility）

既存の分離形式は新versionで同じように読める。新形式を旧binaryで読むこと、および旧binary向けversion維持は対象外。sourceの自動変換は行わない。

## 受け入れと実装への影響（Acceptance and Implementation Impact）

- inlineのみ、分離のみ、両方混在、空recordsでTable解決・Buildが正しく動く。
- inline recordsの編集・追加・削除・tag変更はschema部分と無関係なtextを保持する。
- Add / Rename / Drop Fieldとtype関連Migrationが必要なinline recordsを更新し、同一fileへ重複planを作らず、postconditionを検証する。reference関連Migrationも同じschema fileで重複planを作らない。
- GUIで作成形式を選び、inline fileのrecord編集とdata fileからのschema編集を行える。

## 未解決事項（Open Questions）

なし。

## レビュー（Review）

Blocking Issues: None identified. First challenge passでvalue editとadd/delete/tagのowner混同を修正済み。Non-blocking Issues: None identified. Questions: None identified. Approved as Proposed: Yes. Intent fidelity、cross-spec consistency、testability、compatibility、documentation ownershipを再確認した。

Autonomous approval eligibility: Eligible: Yes. Human gate: None. 既存sourceは引き続き同じ形で読める追加形式であり、旧binaryのversion保守をしない選択はHumanが明示した。

## 承認記録（Approval Record）

Approval mode: Agent-autonomous. Basis: Human-selected Current Objectiveと2026-09-25の実装指示。Review result: Blockingなし、material ambiguityなし。Canonical application: この変更で対象仕様へ反映。
