# 仕様変更0031: Computed View v1退役

Status: Applied

## Why

将来のProgrammable View要件は保持するが、現行Computed View v1の独自DSLを将来方向として固定しない。早期実装されたpersisted view、
expression runtime、Overview/query projection、migration integrationを除去し、将来ゼロベースで再設計できる境界へ戻した。

## Canonical result

- `kind: view` document、View CRUD、expression parser/AST/type checker/evaluator、View-specific diagnostics、Overview integrationをcurrent contractから除去した。
- Schema / Type Migration、source creation、validation、Build、project discovery、CLI/Tauri、frontendからComputed View専用のsurfaceを除去した。
- Programmable Viewは将来意図としてのみ保持し、DSL、runtime、persisted shape、互換性、優先時期を定義しない。
- retained authoring/query/Build/Publish/Unity semanticsは変更しない。

## Approval / application

2026-09-23 JSTのHuman decisionを適用済み。0027はretired audit recordへ縮退し、実装・docs routing・testsを削除した。
