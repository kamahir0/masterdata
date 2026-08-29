# ADR 0002: Rust core is shared by CLI and GUI

Status: Accepted

## Context

The CLI and GUI need identical project discovery, parsing, validation, and
build semantics. A GUI subprocess call to the CLI would make error handling,
testing, and lifecycle behavior unnecessarily indirect.

## Decision

`masterdata-app` owns shared application orchestration such as project info,
validation, build preparation, C# generation, and the .NET boundary.
`masterdata-core` owns shared domain/document operations. The CLI and Tauri
invoke the application service as a Rust library; neither frontend may
reimplement domain logic or invoke the CLI subprocess.

## Consequences

Core/application APIs need structured serializable results and diagnostics.
UI-specific formatting belongs in adapters. A change to project semantics is
made once and tested once at the core boundary. Tauri preserves diagnostic
code, kind, source location, schema path, record identity, suggestion, and
related requirement references in its error DTO.
