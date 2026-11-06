# Source-aware selection and controlled ablations

Keep the legacy `Corpus::retrieve` behavior available as the live control. Extend captured
file metadata with bounded lexical scopes and source-role bases using the registered
Rust/Python analyses and generic path conventions. Attach roles/scopes to returned
evidence, not inferred call/dependency edges. Keep query-policy code outside the model
adapter and include its version in source/index identity.

The compact selector ranks lexical candidates with repository-independent intent,
authority and symbol/container evidence, then selects literal excerpts, preferring line boundaries
within a total byte allowance. Current production behavior and test/historical evidence
remain distinguishable. Prefer multiple useful scopes over duplicate overlapping windows.
Corpus replacement is atomic and reuses unchanged file metadata/chunking; capture remains
no-follow and does not execute project tools. No new third-party dependency is required.

The native JSON-line protocol gains explicit selector and bounded context options while
retaining legacy defaults. A separate ID-answer validation operation resolves only labels
from the exact native-held context, checks snapshot freshness, and returns canonical
source references. Existing quote validation remains unchanged as the control.

Versioned dataset tools accept explicit paths. Source audit may inspect gold anchors, but
development retrieval reports and tuning must exclude fresh held-outs until a freeze
record hashes code, cases and selection settings. A separate held-out run verifies the
freeze rather than silently accepting changed inputs. Prompts receive no oracle fields.

Use one fixed model with three arms: legacy evidence + quotes, compact evidence + quotes,
and identical compact evidence + IDs. The latter two share prompt text except reference
instructions/schema. Preserve source-role metadata in compact arms. Compare semantics
separately from reference-copy/ID validity and from JSON correctness. Use the existing
safe process-tree observer after tests and serial lower-priority mutations finish.

## Native protocol reference

`retrieve` accepts `selector` (`legacy` by default, or `compact`), `limit` (default 6,
range 1–8) and `budget` (default 8,000, range 1–16,000 literal UTF-8 bytes). The original
operation with no options retains the legacy selector. Invalid selectors, unknown fields
and invalid bounds fail. The compact cohort specifies limit 3 and budget 2,500.

Evidence includes `role`, `role_basis` and optional `scope` (`name`, `kind`, `parents`).
Roles use path conventions and the registered Rust/Python raw syntax. Scope names/kinds
are bounded to 128 UTF-8 bytes, ancestors to 16 and scopes to 4,096 per file. These hints
do not establish configured production inclusion, semantic authority or call resolution.

`validate_ids` takes the native-held context `id` and an answer with `status`, `answer`
and integer `citations`. It rejects stale snapshots, unknown/expired contexts, unavailable
or duplicate labels, empty/overlong answers and an answered status without citations.
Successful responses contain `references`, each with canonical `label`, `snapshot`,
excerpt `id`, `path`, full-file `content`, `start_line` and `end_line`. The legacy
`validate` operation still requires exact source quotations. Both report semantic
correctness as unchecked. The host retains at most 64 contexts and accepts at most
128 KiB per JSON-line request, as in the pilot.
