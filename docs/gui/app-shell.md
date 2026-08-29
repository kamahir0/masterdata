# GUI app shell

Status: Draft

The Tauri v2 application is a thin desktop adapter. On startup it invokes the
`project_info` Tauri command. The command calls `masterdata-core` and returns a
serializable `ProjectInfo`; the React frontend does not inspect the filesystem,
parse YAML, or spawn the CLI.

The initial shell contains:

- a project identity card with root/config/source paths;
- placeholder navigation for tables and types;
- Reload project action;
- disabled Validate and Build buttons, pending shared application commands.

The planned layout is left navigation, central record/editor area, right
inspector, and top Save/Validate/Build actions. GUI errors should eventually
render the core diagnostic code, path, schema path, record identity, and
suggestion separately.

Open Questions: project picker UX, unsaved changes policy, and native file
watcher strategy.

