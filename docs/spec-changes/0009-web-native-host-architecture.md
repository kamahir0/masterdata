# 仕様変更: Web / Native Host capability architectureを記録する（Specification change）

Status: Applied

> Historical audit record. Full proposal / review detail remains in Git history.

> 後続の[仕様変更0022](0022-retire-web-product-hosts.md)により、このWeb / Native Host方向はretireされた。以下の適用記録は当時のdecision historyであり、列挙したownerをcurrent authorityとして再開しない。

## Why

Webを正式product hostとし、Standalone / Connected mode、Native Host capability、CLI direct composition、shared Rust semantics、.NET boundaryをarchitectureとして採用した。

## Canonical result

- [Runtime hosts](../specs/runtime-hosts.md) — `RUNTIME-HOST-001..013`
- [Product Vision](../product/vision.md) — Web / local-first direction
- [RFC 0004](../rfcs/0004-web-native-host-runtime.md) — accepted alternatives
- [ADR 0006](../adr/0006-host-capability-composition.md) — host composition WHY

## Approval / application

Human maintainerがWeb / Native Host scopeを承認。Runtime HostsをApproved canonical specとして追加し、Vision / RFC / ADRへ各ownerに応じて適用した。
