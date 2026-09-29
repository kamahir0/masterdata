# Current Objective

## Role

この文書はHuman-selectedなcurrent priorityとDONE boundaryのownerである。observable semanticsはcanonical specification、resume地点は[Development State](execution-state.md)、implementation realityはcode / tests / Gitで確認する。

## Objective

**Unified Table EditorのGrid Spatial Authoringを仕上げる。長大・横長Tableでrow / column contextを保ち、Column / Record Row / Array Itemの順序を直接操作できるようにする。position-relativeなInsert / Move / Removeをcontextual actionとして揃え、pointerとkeyboardから到達可能にする。**

## Completion slices

- Desktopでlong / wide / empty gridを観察し、spatial friction inventoryを作る。
- sticky header / row identityと、長いgridでのAdd Row到達性を整える。
- Column / Array Itemのdrag reorderとkeyboard fallback、Columnのposition-relative Insertを実装する。
- source-order Record Rowのreorder / position-relative Insertをshared source-preserving authoringとして仕様化・実装し、filter / sort時は安全に制限する。
- source-local history / Save、focus、virtualization、clipboard / range selectionを維持し、focused regression、Desktop Golden Paths、repository check、required CIで検証する。

## Canonical requirements

- [Table / Keys](specs/table-and-keys.md)、[Source Record Mutation](specs/source-record-mutation.md)、[Field Declaration Mutation](specs/field-declaration-mutation.md)、[Unified Table Editor](gui/table-editor/spec.md)、[Data Editor](gui/data-editor/spec.md)、[Grid Authoring](gui/data-editor/grid-authoring.md)、[Record Mutation](gui/data-editor/record-mutation.md)

## Explicit non-scope

- 0044 Save、0042 scalar semantics、0045 Complex Value commit model、Search / Filter / Sort UI、record source間move、PK / SKとMessagePack key、Explorer、Source Creation、Settings / Delivery、visual themeの再設計。
