# Review ownership validation

Review capture now has bounded pair/source ownership before publication. The
[reference](../../docs/prototype/REVIEW_OWNERSHIP.md) defines the native store, candidate,
query lease and accounting interfaces. No analysis scheduler, host route or comparison
UI is introduced by this slice.

## Behavioural checks

The initial retention test failed against the unimplemented store before implementation.
Public-interface tests now exercise independently specified ownership/byte outcomes:

- atomic replacement, continued reads through an existing retired lease, refusal of
  new retired-token lookups and refusal of a third pair;
- baseline-owner sharing across refresh, distinct storage charged for equal-content
  captures, and release after the final query clone;
- byte reservations observed inside an input read, unused-capacity release and capture
  refusal preserving the active pair;
- abandonment, incomplete stages, foreign/stale tickets, cancellation during IO,
  scope preservation and incompatible-workspace rejection;
- closed tokens, restarted-store token namespaces, shutdown with held queries, and
  late requests that cannot revive old source ownership;
- source-only/empty inputs, literal target charges, unrepresentable bodies that are
  not retained, and bounded window/diff results that do not retain source leases;
- input panic/unwinding without leaked byte reservations or poisoned publication locks.

Additional native validation checks shared `CapturedSource` owner accounting,
request-sequence exhaustion and shutdown while an internal metadata guard is alive.
The shutdown case was added after mutation testing exposed the missing assertion;
metadata bookkeeping must not extend closed-store source lifetime.

All 367 all-features workspace tests, doctest checks, formatting and Clippy pass.

## Frozen review fixture

The [recorded admitted captures](results/ownership-fixtures.json) exercise both prepared
variants through `ReviewStore`, native Git inputs and query leases. They retain the
frozen two-path change inventory, three/two Rust hunk coordinates and added-document
absence/text distinctions. Exactly one active pair remains charged after publication,
without a private byte reservation. Separate store lifetimes use distinct publication
tokens while unchanged captured content preserves comparison identity. Fixture
status/bytes still verify after capture.

Reproduce using [the native fixture procedure](README.md#check-native-git-capture) with
`--require-admission`. No analysed-project scripts or builds run. These are correctness
checks, not timing/RSS measurements, an operator study or a linked-UI walkthrough.

## Mutation and integration scope

The [mutation summary](results/ownership-mutation.json) records the complete new
`review.rs` module, an unmodified baseline, commands and final source/test hashes.
Runs use one mutant, one compiler/jobserver task, serial tests and lower priority.
The final full-module run caught 52 of 70 generated cases, with 18 unviable,
zero survivors, zero timeouts and no exclusions. An earlier shutdown survivor prompted
the metadata-lifetime regression. Redundant publication state checks were simplified
to one authoritative ticket check under the lock before the final run.
Unviable mutations and timeouts are distinct from cases caught by test assertions.
Existing acquisition, portable hashing/comparison, analysis, HTTP/browser code and
source backing are outside this module's mutation scope.

The default source allowance is logical content, not heap/RSS. Current capture makes
new storage per read; cross-capture source reuse beyond the held baseline and integration
with charged analysis rebinding remain future work. The next slice connects capture
to the existing analysis scheduler while preserving this ownership, before host/UI exposure.
