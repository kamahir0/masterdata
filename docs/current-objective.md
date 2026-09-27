# Current Objective

## Role

この文書はHuman-selectedなcurrent priorityとDONE boundaryのownerである。observable semanticsはcanonical specification、resume地点は[Development State](execution-state.md)、implementation realityはcode / tests / Gitで確認する。

## Objective

**Source scalarの意味をschemaから決めるshared semantic modelと、semantic-invalidな途中状態を保持できるTable authoringを設計する。Save、Validate、Buildの責務を分離し、GUIとCLIに同じCore解釈を適用する。**

## Completion slices

- 現在のSource→Type System→authoring / CLI / Build経路、fixture、互換性を調査し、Scalar Interpretation Matrixとauthoring / persistence modelを設計する。
- canonical仕様変更と必要なarchitecture decisionをreviewし、Human gateの要否を明確にする。
- gateを満たせばPrimitive scalar、field type / Nullable / Array draft、GUI diagnostic、CLI validate parity、Save / Build境界をshared Coreからvertical sliceとして実装する。
- source preservation、exact identity、lost-update、rollback / recoveryを維持し、focused regression、repository check、Desktop実操作で検証する。

## Canonical requirements

- [YAML subset](specs/yaml-subset.md)、[Primitive Types](specs/type-system/primitives.md)、[Field Modifiers](specs/type-system/field-modifiers.md)、[Type System](specs/type-system/README.md)
- [Table Editor](gui/table-editor/spec.md)、[Data Editor](gui/data-editor/spec.md)、[Source Record Edit](specs/source-edit.md)、[Field Declaration Mutation](specs/field-declaration-mutation.md)、[Schema Migration](specs/schema-migration.md)、[Build Pipeline](specs/build-pipeline.md)

## Explicit non-scope

- Table identity、MasterMemory binary format、Build / Publish / Gitの暗黙実行の変更。
- source-preserving rewrite、lost-update / dirty protection、rollback / recoveryの弱体化。
- frontend独自のYAML / domain解釈、Settings / Delivery / chrome全体のvisual redesign。
