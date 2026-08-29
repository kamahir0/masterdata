# ADR 0001: YAMLをSource of Truthとする

Status: Accepted

## 背景（Context）

Generated C#とMasterMemory binary artifactはderived outputである。generated artifactをauthorityとして
扱うと、review、regeneration、compatibility trackingが信頼できなくなる。

## 決定（Decision）

YAML schemaとdata documentをcanonicalなSource of Truthとする。Generated C#、builder output、cache、
MasterMemory binaryは再現可能なartifactであり、editのauthorityとして扱ってはならない（MUST NOT）。

## 結果（Consequences）

変更をtextとしてreviewでき、CIでregenerateできる。YAMLにはstable IDとcompatibility validationが必要である。
GUI editはYAML（または将来明示するtransaction format）へwriteしなければならず、generated outputを
黙ってpatchしてはならない。
