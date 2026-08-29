# Project layout and discovery

Status: Approved

## Normative rules

- `PROJECT-001`: A project MUST be identified by a file named
  `masterdata.toml`.
- `PROJECT-002`: An explicit project path MUST take precedence over implicit
  discovery. The path MAY be a project directory or the config file itself.
- `PROJECT-003`: Without an explicit path, discovery MUST inspect the current
  directory and each parent directory until the filesystem root.
- `PROJECT-004`: If no marker is found, the operation MUST return a structured
  project-not-found diagnostic.
- `PROJECT-005`: Unity `Assets/` and `ProjectSettings/` MAY be used as `init`
  hints, but MUST NOT be the project identity mechanism.
- `PROJECT-006`: Source roots are scan boundaries only. A source file's
  directory MUST NOT determine its table, type, or index semantics.

## Configuration shape

```toml
[project]
id = "game.masterdata"
name = "Game Master Data"
version = "0.1.0"

[sources]
roots = ["sources"]

[build]
output = ".masterdata/generated"
cache = ".masterdata/cache"
```

`project.id`, `project.name`, `project.version`, and at least one source root
are required. Paths are resolved relative to the project root unless they are
absolute. Cross-platform code MUST use path APIs rather than shell separators.

Open Questions: whether config should later support named source groups,
ignore patterns, or an explicit Unity project link.
