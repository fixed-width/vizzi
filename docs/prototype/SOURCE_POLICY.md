# Source selection and snapshot policy reference

`vizzi_core::policy` implements portable selection over normalized workspace-relative
paths. It performs no I/O and does not construct parsers. Native `Directory` adapters
apply the same frozen policy during discovery, hint verification, and event filtering.

## Configuration and classification

`PolicyConfig` contains enabled `c_family_roots`, `exclusions`, and explicit C/C++
`assignments`. Rust `.rs` and Python `.py` inputs are selected by default. Family scopes
add the candidate suffixes in [the C-family policy](C_FAMILY_POLICY.md), including
`.c++` and `.h++`. Assigned C/C++ inputs receive [raw declaration extraction](C_FAMILY_SYNTAX.md).
Ambiguous headers, unassigned fragments, and CUDA remain source-only. An unavailable
factory in a custom registry still preserves source without fake analysis.

`SourcePolicy::new(config, available_languages)` validates and freezes the configuration.
`classify(path)` returns no capture for excluded/out-of-scope files, or one immutable
`Classification`. `excludes(path)` also applies to directory pruning. Invalid relative
paths return errors rather than silently behaving like absent source.

`Classification::Assigned` carries a registered language and assignment provenance.
`SourceOnly` carries an optional known/requested language, provenance, and one reason:

| Reason | Meaning |
| --- | --- |
| `language_unassigned` | Ambiguous C/C++ header with no explicit language |
| `dialect_unsupported` | Recognized CUDA source/header without a supported analyzer |
| `fragment_unassigned` | Textual fragment without an explicit language |
| `analyzer_unavailable` | A language was requested but has no registered factory |

Provenance is an exact suffix (`extension`) or normalized explicit rule scope
(`explicit_scope`). Encoding is independent: `utf8` permits text preview; `unsupported`
does not. A non-UTF-8 unassigned header retains both facts. Source-only documents have
no analysis records; `Document::capture` rejects attempts to attach them.

Rules match complete path components. Exclusions override assignments. CUDA cannot be
reclassified by a general C++ assignment. More specific assignments override parent
rules; conflicting rules at the same normalized scope fail construction. Assignments
outside enabled family scopes fail. Root `.git`, `target`, and `node_modules` exclusions
and symlink behavior remain; nested directories with those names are preserved.
Dotfiles such as `.rs` are not suffix matches, while `a.C` is distinct from `a.c`.

Scopes use `.` for the root. They accept redundant separators, `.` components, and
trailing separators, but reject empty input, absolute paths, traversal, backslashes,
and drive-prefixed paths. Normalized duplicate/nested roots and exclusions collapse.
Same-scope duplicate assignments and child assignments redundant with their effective
parent collapse. Rule order is not a priority mechanism. This is documented syntactic
normalization, not exhaustive equivalence checking of arbitrary policy configurations.

## Registration and native construction

`Languages` registers factories by `Context.language`; empty/duplicate language names
and repeated context identities fail registration. Adapters no longer claim extensions.
`Languages::select(language)` looks up a factory; `names()` supplies availability to the
policy. There is one registered raw adapter per language in this host.

`Directory::with_policy(root, config, languages)` constructs policy and factories
together so availability cannot drift. `open` uses default configuration and built-in
Rust/Python/C/C++ factories; `with_languages` supplies a different factory registry with
default capture rules. Removing a factory preserves selected bytes as source-only.

Policies are immutable for a host's lifetime. The CLI exposes repeatable
`--c-family-root SCOPE`, `--exclude-source SCOPE`, and `--syntax-language SCOPE=c|cpp`.
See [running the workspace](../how-to/run-workspace.md) for usage. Compilation-database
import, configured profiles, and dynamic configuration reload are not implemented.

## Identity and incremental retention

`CapturePolicyId` hashes the `source-policy-v1` domain, policy implementation source,
normalized roots/exclusions/assignments, and the sorted set of available languages.
Hashing implementation bytes conservatively invalidates policies after code changes.
Analyzer context/version changes participate in the verified
[capture plan](CAPTURE_PLAN.md) used when retaining documents.

Every snapshot retains its policy identity, including empty snapshots. A mismatch
forces complete discovery before any unread documents can be retained. No caller
precondition is needed after expanding or reducing scope. Snapshot composition v4
includes policy identity; document composition v2 includes classification and analyses.
Verified native snapshots also contain a plan marker covering all analyzer descriptors;
matching that marker avoids per-file reclassification on ordinary hints.
Workspace/file/entity identity rules remain as specified by [IDENTITIES.md](IDENTITIES.md).

Full captures can reuse documents whose bytes, classification, and analysis context
remain equal even if policy changed elsewhere. Analysis caches still store only
path-independent facts keyed by content/context. Rebinding supplies current selection
metadata. Source-only inputs require no parser or persisted fake analysis.

View and pinned-source responses use schema 2 and retain policy/classification metadata.
Status and notification envelopes remain schema 1. `analyzed` counts files with an
analysis record, including records reporting unavailable analysis; `source_only` counts
files with no records. These counts do not assert successful grammar acceptance.
The [API reference](API.md) defines fields and pinning.
