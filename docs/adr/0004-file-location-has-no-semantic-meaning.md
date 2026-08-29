# ADR 0004: File location has no semantic meaning

Status: Accepted

## Context

Projects may split a table across files, keep all YAML in one directory, or
organize files by feature/team. Inferring table identity from a directory
would make harmless moves into semantic changes.

## Decision

Filesystem location is only a discovery boundary. Each YAML file declares its
`kind` and `table`; schema fields and future index/reference declarations carry
the remaining meaning.

## Consequences

Discovery scans configured roots recursively and sorts paths only for
deterministic processing/hash input. Moving a file inside a source root does
not change its table identity. Source roots themselves are explicit project
configuration, not hard-coded directory conventions. The current traversal
does not follow symlink entries as an internal cycle-safety guard; whether
source discovery follows symlinks is a separate product Open Question.
