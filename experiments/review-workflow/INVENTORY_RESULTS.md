# Portable review inventory validation

The first comparison slice classifies supplied immutable observations in `vizzi-core`.
It provides sorted inventory iteration, per-path lookup and side-specific retained
source. The [technical reference](../../docs/prototype/COMPARISON_TECH.md#delivered-portable-inventory-interface)
documents the delivered interface. No Git acquisition, hunk computation, pair admission
or comparison UI is delivered by this slice.

## Behavioural evidence

Thirteen integration tests exercise the public interface with independently specified
outcomes and in-memory source, without a filesystem or SCM provider:

- added/removed empty files, verified content changes, executable-only and type changes;
- complete sorted union, lookup and rename-as-add/delete without copying source bodies;
- explicit unknown presence versus known presence with unavailable bytes; excluded,
  unreadable, sparse and unsupported entries cannot become false equality/deletion;
- symlink-target/reference bytes and binary data without invented source documents;
- lossless non-UTF-8/backslash/control-character paths and compatible normal file IDs;
- duplicate/missing inventory, incorrect document identity/length/type and incompatible
  workspace/review policy/analysis policy/capture-plan marker rejection;
- stable order-independent observation identity, ordered comparison identity, and
  identity changes for metadata, availability, inventory and analysis/source differences;
- source-only documentation on either side, deleted source retained on the base, and
  analysis-coverage changes that do not invent raw-content changes.

The initial classification test failed against the unimplemented interface before
implementation. After the implementation, all 315 all-features workspace tests
(including the 13 comparison tests), doctest checks, formatting and Clippy passed. The recorded
[mutation summary](results/inventory-mutation.json) binds the source/test hashes and
commands used for the final run.

## Mutation scope and limitations

The final run mutates the complete `comparison.rs` module, including path validation,
inventory/document binding, policy compatibility and classification. It runs the
comparison integration suite with an unmodified passing baseline and no exclusions.
Of 61 generated mutations, 45 were caught by test failures and 16 were unviable;
there were zero survivors and zero timeouts. Unviable cases do not count as evidence
that the tests caught the corresponding change.

During preparation, a minimal probe showed that the installed cargo-mutants 27.1.0
omits methods named `new`. Descriptive operation names (`parse`, `from_inventory`,
`between`) ensure this run includes the validating constructors; the generated list
was checked before execution. Mutation results cover generated patterns, not every
possible bug. Existing identity-hash implementation, native acquisition/publication,
browser behaviour and text-diff algorithms are outside this module's mutation scope.

This is correctness evidence for the portable inventory interface. There is no new
performance measurement, actual local-change capture, frozen-fixture UI replay or
independent-user evaluation. Next is bounded raw-text hunk computation from the same
captured pair, before native Git acquisition and linked review views.
