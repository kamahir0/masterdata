# 仕様変更: Native Hostのzero-terminal Connected lifecycleを定義する（Specification change）

Status: Applied

> Historical audit record. Current implementation authority is the canonical specification listed below. Full proposal / review detail remains in Git history.

## Why

setup/authorization済みの通常利用ではterminal操作なしにNative Hostをdetect/validateしてConnected modeへ移行できるlifecycleを追加した。

## Canonical result

- [Runtime hosts](../specs/runtime-hosts.md) — `RUNTIME-HOST-014..016`
- [Product Vision](../product/vision.md) — zero-terminal normal flow
- [RFC 0004](../rfcs/0004-web-native-host-runtime.md) — traceability

## Approval / application

Human maintainerがautomatic detection/handshake、valid prior authorizationでのConnected transition、explicit reauthorization維持、Standalone fallback、CLI direct composition維持を承認し、canonical ownersへ適用した。
