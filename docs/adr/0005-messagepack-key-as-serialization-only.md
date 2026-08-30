# ADR 0005: MessagePack keyをfield identityから分離する

Status: Proposed

## 背景（Context）

0003適用前のApproved Field Identity仕様は、TableとCustom Typeのpersisted fieldに、renameと削除後のreserved identityを追跡する
numeric Field IDを与えていた。specification change 0003のApplied後のTable/Key設計では、persisted fieldへ必要なnumeric valueをMessagePack serialization layoutのためだけに
明示し、logical field identityやschema migration identityとは分離する。

これはApproved specificationへの変更であったため、採用理由とdeltaは
[Field IDからMessagePack keyへのspecification change](../spec-changes/0003-field-identity-to-messagepack-key.md)に記録され、human approvalと
atomic applicationが完了した。現在のcontractは更新済みcanonical specificationが所有する。

## 決定（Decision）

提案するarchitectureでは、TableおよびCustom Typeのpersisted fieldの`key`をMessagePack `[Key(n)]`へ対応付ける。fieldのlogical name、
Primary/Secondary Keyのresolved field symbol、Reference identity、schema migration identityは別のconceptとして扱う。
具体的なobservable contractは[Table / Primary Key / Secondary Key仕様](../specs/table-and-keys.md)が所有し、既存Approved Field Identityと
Custom Typeへのdeltaはspecification changeが所有する。

## 結果（Consequences）

MessagePack keyの変更はserialization layoutの変更になり得るが、field rename、deletion、addition、secondary-key identity、reference identityを
意味しない。Field ID tombstoneとreuse prohibitionをv1のpersisted field modelに残さないため、cross-schema binary compatibilityは別途設計する
必要がある。schema revision、generated C#、binaryはcoherent artifact setとして扱う。

Table / Primary Key / Secondary Key仕様はApprovedであり、specification change 0003はAppliedである。このADRはarchitecture rationaleを
所有するが、implementation authorityまたはnormative requirementのcanonical ownerではない。

## 代替案（Alternatives）

- MessagePack keyをlogical field identityとしても再利用し、serializationとschema migrationを同じnumeric valueで追跡する。
- MessagePackから独立したpersistent Field IDを維持し、そのmappingを別途MessagePack keyへ保存する。
- field nameまたはsource orderからMessagePack keyを暗黙に導出する。
