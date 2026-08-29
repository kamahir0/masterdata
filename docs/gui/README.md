# GUI specifications

GUI behavior is specification-worthy when it affects a user's observable
workflow. GUI specs use the same `Draft`, `Proposed`, `Approved`,
`Implemented`, and `Deprecated` lifecycle as domain specs. Use a `GUI-` prefix
for requirement IDs, followed by the surface and a stable three-digit number.

The GUI specification should describe behavior rather than duplicate
`masterdata-core` semantics. It may cover layout, state, selection, editing,
validation, focus, keyboard and mouse interaction, loading, empty/error/
disabled states, unsaved changes, and build-in-progress behavior. The
Tauri command remains the adapter boundary for shared domain operations.

Use [_template.md](_template.md) when starting a new surface specification.
For a larger surface, keep visual artifacts beside its specification:

```text
docs/gui/table-editor/
├── spec.md
├── default.png
├── validation-error.png
└── empty-state.png
```

Every image listed by a spec MUST be labeled either `Normative` (the reviewed
visual is part of the acceptance contract, with the relevant state and
viewport described) or `Reference-only` (a design aid that does not add
behavior). A reference image never overrides written normative requirements.
If an image is changed, review the affected GUI requirement IDs and update the
compatibility/acceptance notes as needed. This task intentionally adds no
Table Editor images or behavior.
