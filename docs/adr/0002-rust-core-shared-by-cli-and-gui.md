# ADR 0002: Rust core is shared by CLI and GUI

Status: Accepted

## Context

The CLI and GUI need identical project discovery, parsing, validation, and
build semantics. A GUI subprocess call to the CLI would make error handling,
testing, and lifecycle behavior unnecessarily indirect.

## Decision

`masterdata-core` owns shared domain/application operations. The CLI invokes it
as a Rust library; Tauri commands invoke the same library directly. Neither
frontend may reimplement domain logic.

## Consequences

Core APIs need structured serializable results and diagnostics. UI-specific
formatting belongs in adapters. A change to project semantics is made once and
tested once at the core boundary.

