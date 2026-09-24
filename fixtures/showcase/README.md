# 開発用パターン見本

`cargo xtask dev-reset`、`cargo xtask cli`、`cargo xtask gui` はこの固定入力から
`target/dev-project` を再作成する。編集内容を残したい場合は、`target/dev-project`
を別の場所へコピーして使う。

| 見る対象 | ファイル | パターン |
| --- | --- | --- |
| Table | `catalog-schema.yaml`, `catalog-data.yaml` | primitive 全種、VO、Custom Type、Enum、Flags、Nullable、Array、単一 Primary Key、unique / non-unique / 複合 Secondary Key、record tag |
| Type | `item-id.yaml`, `item-code.yaml`, `reward.yaml`, `rarity.yaml`, `item-tags.yaml`, `long-features.yaml` | 数値・文字列 VO、方向別 implicit conversion、Custom Type、通常 Enum、符号付き・符号なし Flags Enum |
| 複合 Key | `region-schema.yaml`, `region-data.yaml` | 複合 Primary Key、unique Secondary Key |
| Reference | `spawn-schema.yaml`, `spawn-data.yaml` | 必須の単一/複合参照、non-unique 対象への複数件参照、Nullable 参照 |
| 空 Table | `empty-table-schema.yaml` | data document がない Table |
| Build Profile | `masterdata.toml` | `production` と `development` による record selection |

`sources` 内のファイル名は識別子ではない。`kind`、`table`、`name` がソース上の宣言を表す。
不正入力の例は `fixtures/invalid`、最小構成は `fixtures/minimal` を参照する。
