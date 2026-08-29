# RFC: YAML parser/library selection

Status: Proposed

## Context

YAML is the project Source of Truth. The current implementation uses
`serde_yaml = 0.9` for typed deserialization and a `serde_yaml::Value`
intermediate tree. Parser selection therefore affects diagnostics, future GUI
editing, and the interpretation of source files. This RFC records an
investigation and does not authorize a dependency migration.

## Problem

The current parser is convenient for Serde mapping, but its upstream repository
is archived and the `0.9.34` release states that the crate is no longer
maintained. A future replacement must be evaluated against both machine
validation and human-preserving editor requirements; a parser that only loads
values is not automatically suitable for round-trip editing.

## Goals

- compare realistic Rust options against the product's actual YAML needs;
- make unsupported YAML behavior explicit as product Open Questions; and
- avoid a large parser migration before a human-approved decision.

## Non-Goals

This RFC does not choose a YAML subset, change `serde_yaml`, implement a
round-trip editor, or define schema/domain semantics for anchors, tags, or
multiple documents.

## Options

### Current `serde_yaml 0.9`

Strong Serde integration and a small migration surface. The upstream GitHub
repository is archived and the latest `0.9.34` release is marked as no longer
maintained. The value-oriented API is not a lossless syntax tree, so comments,
whitespace, quote style, and exact formatting are not preserved by the current
model. Error objects expose parser marks, but the repository still needs to
map them consistently into the core Diagnostic.

### `yaml_serde`

The YAML organization publishes an actively maintained fork intended as a
minimal migration from `serde_yaml`; its documentation describes package
renaming as an option. It is a promising Serde-compatible candidate, but this
RFC does not assume behavioral equivalence for duplicate keys, tags, scalar
resolution, or error locations without a compatibility test corpus.

### `yaml-rust2` / `saphyr`

These are pure-Rust YAML 1.2 parser/document APIs. `yaml-rust2` documents a
stable/basic-maintenance posture, while `saphyr` is the newer, more actively
developed API family. They provide document/event access and multiple-document
loading, but are not drop-in Serde replacements for the current typed model;
conversion, duplicate-key policy, source spans, and serialization behavior
would need explicit engineering.

### `yaml-edit`

This option is a lossless syntax-tree editor focused on preserving comments,
whitespace, and formatting while editing. It is attractive for a future GUI
write-back path, but it is not a drop-in replacement for the current Serde
model and would require a deliberate bridge between lossless syntax and the
typed domain AST.

### `serde-saphyr`

This is a Serde-oriented YAML deserializer built on the Saphyr ecosystem. Its
documented feature set includes error reporting and comment-aware wrappers,
but its maturity, duplicate-key behavior, round-trip model, and compatibility
with this repository's typed AST require a repository-specific evaluation.

## Comparison matrix

The matrix distinguishes capabilities reported by the candidate projects from
requirements that still need a local test. `Unknown` means “not established by
this RFC”, not “allowed by the product”.

| Criterion | serde_yaml 0.9 | yaml_serde | yaml-rust2 / saphyr | yaml-edit | serde-saphyr |
| --- | --- | --- | --- | --- | --- |
| Maintenance status | Upstream archived/deprecated | Maintained fork candidate | Pure-Rust family; yaml-rust2 favors stability, saphyr moves faster | Focused editor candidate | Active candidate; local maturity check needed |
| Serde integration | Native | Intended minimal migration | No drop-in native mapping | None as current model | Native Serde path |
| Error line/column | Parser mark available; local mapping needed | Local compatibility test needed | Event/parser spans need local mapping | Position tracking documented | Error reporting documented |
| Duplicate mapping keys | Local behavior test required | Local behavior test required | Local behavior test required | Syntax retained; semantic policy local | Local behavior test required |
| Anchors / aliases | Local behavior test required | Local behavior test required | YAML document/event support; local semantic test required | Syntax can be preserved; semantic policy local | Documented support; product policy local |
| Merge keys | Local behavior test required | Local behavior test required | Product policy local | Syntax can be preserved; product policy local | Documented support; product policy local |
| Multiple `---` documents | API behavior must be tested | API behavior must be tested | Documented document loading | Document model behavior must be tested | API behavior must be tested |
| Custom tags | Local behavior test required | Local behavior test required | Event/document support; product policy local | Syntax preservation possible; policy local | Documented support path; policy local |
| Numeric interpretation | Compatibility corpus required | Compatibility corpus required | YAML 1.2-oriented; corpus required | Syntax retained; conversion local | Compatibility corpus required |
| Timestamp interpretation | Compatibility corpus required | Compatibility corpus required | Conversion policy local | Syntax retained; conversion local | Compatibility corpus required |
| Unknown fields | Serde attributes/local policy | Serde attributes/local policy | Conversion layer policy | Syntax can preserve them | Serde attributes/local policy |
| Round-trip editing | Not lossless in current model | Not established | Emitter, not lossless editor | Lossless editing is the primary goal | Comment-aware values, not full format guarantee |
| Comment preservation | Not in current typed model | Not established | Not the current typed model's guarantee | Explicitly supported | Comment wrappers documented |
| Format/quote preservation | No | Not established | No guarantee from emitter | Explicitly supported | Not established as full format preservation |
| Performance | Existing baseline | Baseline needed | Candidate benchmark needed | Editor-oriented benchmark needed | Candidate benchmark needed |
| Cross-platform | Rust/native dependency behavior to test | Test required | Pure Rust | Pure Rust | Pure Rust ecosystem |
| Ecosystem maturity | Historically mature, now unmaintained | Newer fork | Established lineage, differing maturity | Narrower/newer focus | Newer candidate |
| License | MIT/Apache-2.0 lineage; verify package metadata | Verify package metadata | MIT/Apache-2.0 family | Verify package metadata | Verify package metadata |

## Trade-offs

Keeping `serde_yaml` avoids immediate migration risk and preserves the current
typed implementation, but carries maintenance risk and does not solve future
lossless editing. A Serde-compatible fork reduces code churn but still needs a
corpus. A YAML 1.2 document API improves control over syntax and documents but
requires an explicit typed conversion layer. A lossless editor best serves GUI
write-back but should likely complement, rather than silently replace, the
semantic parser.

## Proposal

Keep the current `serde_yaml 0.9` dependency unchanged until a human-approved
decision is made. Before selecting a replacement, build a compatibility corpus
covering the matrix above and decide whether the architecture needs two
layers: a semantic parser for validation and a lossless syntax representation
for GUI edits. Do not infer the product YAML subset from whichever candidate is
most convenient.

## Compatibility

Changing parser libraries can alter scalar types, duplicate-key acceptance,
anchors/aliases, tags, error locations, and serialization output. Any migration
requires fixture and golden-output comparison plus an explicit compatibility
decision. No migration is performed by this RFC.

## Open Questions

- Which YAML version/dialect is the product contract?
- Are anchors, aliases, merge keys, multiple documents, and custom tags
  allowed?
- Are duplicate mapping keys an error, a first-value rule, a last-value rule,
  or preserved for a later diagnostic?
- Which numeric and timestamp forms become typed values?
- Are unknown fields rejected, ignored, or preserved?
- Must GUI edits preserve comments, quote style, whitespace, and ordering?
- Is a two-layer semantic/lossless representation justified?
- Which license combination and maintenance policy will be accepted for the
  selected parser stack?

## Decision

Pending explicit human approval. Current recommendation: defer migration, keep
the dependency pinned at `serde_yaml 0.9`, and resolve the YAML subset and
editor requirements through a reviewed compatibility corpus.

## References

- [`serde-yaml`](https://github.com/dtolnay/serde-yaml) and its
  [`0.9.34` release](https://github.com/dtolnay/serde-yaml/releases/tag/0.9.34)
- [`yaml_serde`](https://github.com/yaml/yaml-serde)
- [`yaml-rust2`](https://github.com/Ethiraric/yaml-rust2) and
  [`saphyr`](https://github.com/saphyr-rs/saphyr)
- [`yaml-edit`](https://github.com/jelmer/yaml-edit)
- [`serde-saphyr`](https://github.com/bourumir-wyngs/serde-saphyr)
