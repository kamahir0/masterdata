# Custom Type仕様（Custom Type）

Status: Draft

Custom Typeは、supported schema typeを組み合わせたimmutableなgenerated C# valueとして使う
ことを意図している。正確なshapeとrecursion ruleは、まだ承認されていない。

### SCHEMA-CUSTOM-001

Generated custom typeはimmutableでなければならない（MUST）。

## Open Questions（未解決事項）

- Custom Typeにはどのfield typeを含められるか。
- `record` と `struct` の選択をYAMLでどう表現するか。
- recursive custom typeを完全に禁止するのか、それともC# value-type recursionが不正になる場合だけ禁止するのか。
- Custom Type内部のfield IDをどのように割り当て、serializeするか。
- どのnullability/default-value ruleを適用するか。
- どのCustom Typeをkey-compatibleにできるか（該当する場合）。
