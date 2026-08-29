# Project layout and discovery

Status: Implemented

## Normative rules

### PROJECT-001

A project MUST be identified by a file named `masterdata.toml`.

### PROJECT-002

An explicit project path MUST take precedence over implicit discovery. The path
MAY be a project directory or the config file itself.

### PROJECT-003

Without an explicit path, discovery MUST inspect the current directory and each
parent directory until the filesystem root.

### PROJECT-004

If no marker is found, the operation MUST return a structured project-not-found
diagnostic.

### PROJECT-005

Unity `Assets/` and `ProjectSettings/` MAY be used as `init` hints, but MUST NOT
be the project identity mechanism.

### PROJECT-006

Source roots are scan boundaries only. A source file's directory MUST NOT
determine its table, type, or index semantics.

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
are required. The detailed configuration and path requirements below make
these rules independently traceable. The implementation note about path APIs
is non-normative: callers should use platform path values rather than assume a
particular shell separator.

### PROJECT-CONFIG-001

`project.id`, `project.name`, and `project.version` MUST each contain a
non-whitespace value.

### PROJECT-CONFIG-002

`sources.roots` MUST contain at least one source root.

### PROJECT-CONFIG-003

Configured source roots MUST NOT be empty strings. `build.output` and
`build.cache` MUST be non-empty; an optional `build.binary_output`, when
present, MUST also be non-empty.

### PROJECT-PATH-001

Relative source and build paths MUST resolve against the project root. Absolute
paths remain absolute.

Open Questions: whether config should later support named source groups,
ignore patterns, an explicit Unity project link, and whether configured source
roots should follow symlinks. The current implementation does not follow
symlink entries as an internal cycle-safety guard; this is not a
product-level permission or prohibition.

## Acceptance matrix

This matrix is non-normative implementation evidence for the requirements
above. It is kept beside the canonical rules so `implement-spec` can verify the
same observable behavior without making test numbers look like requirement
definitions.

| Requirement | Observable behavior | Implementation owner | Success case | Failure case | Test | Fixture |
| --- | --- | --- | --- | --- | --- | --- |
| PROJECT-001 | A marker named `masterdata.toml` identifies the project. | `masterdata-core::Project` | Explicit directory resolves its marker. | A directory without the marker is not accepted as a project. | `project_001_explicit_directory_uses_masterdata_marker` | `fixtures/minimal` |
| PROJECT-002 | Explicit directory/file path wins over parent discovery. | `masterdata-core::Project::discover` | Inner explicit project is selected. | Parent project is not selected when explicit path is valid. | `project_002_explicit_path_has_priority_over_parent_search` | Temporary project |
| PROJECT-003 | Search starts at the current directory and walks parents. | `masterdata-core::find_config_upwards` | Nested directory finds the nearest ancestor marker. | Search stops at filesystem root. | `project_003_discovers_config_from_parent_directory` | Temporary project |
| PROJECT-004 | Missing marker is returned as structured diagnostic data. | `masterdata-core::Project::discover` | Error has diagnostic code/kind and optional structured context. | Callers do not need a string-only error to identify the condition. | `project_004_returns_structured_not_found_diagnostic` | Temporary directory |
| PROJECT-005 | Unity folders do not establish identity. | `masterdata-core::Project::discover` | `Assets/` and `ProjectSettings/` alone do not resolve a project. | Missing `masterdata.toml` remains not found. | `project_005_unity_folders_do_not_define_identity` | Temporary directory |
| PROJECT-006 | Declared YAML `kind` and `table` own document semantics. | `masterdata-core::Project::load_documents` | Multiple files in one root retain their declared tables. | A file path or directory name cannot relabel a document. | `project_006_source_directory_does_not_define_table_identity` | `fixtures/minimal` |
| PROJECT-CONFIG-001 | Project metadata fields contain non-whitespace values. | `masterdata-core::ProjectConfig::validate` | A complete metadata block is accepted. | An empty `id`, `name`, or `version` returns a structured config diagnostic. | `project_config_001_requires_non_empty_metadata` | Temporary project |
| PROJECT-CONFIG-002 | At least one source root is configured. | `masterdata-core::ProjectConfig::validate` | A project with a source root is accepted. | An empty `sources.roots` list returns a structured config diagnostic. | `project_config_002_requires_a_source_root` | Temporary project |
| PROJECT-CONFIG-003 | Configured source and build paths are non-empty. | `masterdata-core::ProjectConfig::validate` | Non-empty paths are accepted. | An empty source, output, cache, or optional binary path returns a structured config diagnostic. | `project_config_003_rejects_empty_source_or_build_paths` | Temporary project |
| PROJECT-PATH-001 | Relative paths resolve against the project root and absolute paths remain absolute. | `masterdata-core::Project::info` | Project info exposes resolved source and build paths. | A relative path is not resolved against the process working directory. | `project_path_001_resolves_relative_paths_against_project_root` | Temporary project |

All rows must remain valid when path separators differ by platform. Symlink
traversal safety is an implementation guard currently recorded as an Open
Question in the source-discovery documentation; it is not an additional
project identity rule.
