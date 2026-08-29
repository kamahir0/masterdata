# GUI app shell

Status: Draft

The Tauri v2 application is a thin desktop adapter. On startup it invokes the
`project_info` Tauri command. The command calls the shared `masterdata-app`
service, which delegates domain work to `masterdata-core`, and returns a
serializable `ProjectInfo`; the React frontend does not inspect the filesystem,
parse YAML, or spawn the CLI.

The initial shell contains:

- a project identity card with root/config/source paths;
- placeholder navigation for tables and types;
- Reload project action;
- placeholder Validate and Build controls. Their backend commands use the
  shared application service; richer interaction remains future GUI scope.

The planned layout is left navigation, central record/editor area, right
inspector, and top Save/Validate/Build actions. GUI errors preserve the
structured diagnostic code, kind, path, line/column, schema path, record
identity, suggestion, and related requirement references. The frontend may
choose how much of that data to render without flattening it at the Tauri
boundary.

Open Questions: project picker UX, unsaved changes policy, and native file
watcher strategy.
