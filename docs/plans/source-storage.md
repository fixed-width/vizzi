# Measure captured-source ownership before introducing storage limits

[Input admission](../prototype/ADMISSION.md) does not bound snapshot source/index storage.
The current source interface returns borrowed whole-file bytes and text, so bounded
paging needs an explicit read/error contract. Establish ownership and allocation evidence
before selecting that change. Keep current source semantics and publication retention
unchanged during this investigation.

## Accounting and controlled workload

Expose source byte length/capacity and line-index length/capacity/bytes without mutating
or copying source. These cover owned buffers, not Arc headers, documents/facts, allocator
metadata, parser state or process memory. Identify shared allocations by Arc identity,
not equal content: two separately allocated identical buffers still consume storage.
Report per-current-snapshot, publication-retained union, and additional external-pin union.
Never sum per-snapshot totals and call shared bytes distinct allocations.

Use source-only storage snapshots over the pinned Linux/PyTorch scopes, omitting analysis
facts and parsers intentionally. Measure initial capture, verified unchanged capture,
four generations of three-header edits or whole-scope edits, release of an expired
external handle, and final source restoration. Actual publication keeps current plus
three previous snapshots; record expiration through its normal pinned interface. A held
query handle must preserve its immutable source after publication expires that identity.

All edits affect marked owned copies and use unique prefixes. Verify source identities,
classification, unchanged sharing, old pins and restored bytes independently. Record
capture time, buffer capacities, source/index ownership, source counts, point RSS and
sampled process peaks. Use child-only virtual-address-space limits with enough room for
the controlled history case; these are not physical-memory/laptop qualification.

## Design decision

Distinguish a single current observation's storage from version history, external query
handles, per-file temporary reading and allocator retention. Shrinking history cannot
cap a current source set or externally held handles. Compact line indexes or shared
chunks can reduce one part of storage, but a bounded resident store needs fallible range
reads, explicit full analysis loading, immutable backing independent of mutable project
files, and well-defined old-pin/error lifetimes. Keep native I/O outside portable model.

Preserve verified-byte hashing, encoding/line semantics, declaration-range validation,
source windows, full source reachability, context validity and unsafe-code restrictions.
Choose the next source interface from evidence; do not silently stop capturing large
files or discard a valid query handle to claim a process cap.

## Validation

Use TDD for source accounting, spare capacity, UTF-8/unsupported/empty text, unchanged
sharing, distinct equal-content allocations, source history expiry, external handle
release and restoration. Run normal workspace/doctest/fmt/Clippy/browser checks and
complete captured-source mutation scope. Keep timing separate from validation jobs.

## Delivery evidence

[The completed investigation](../../experiments/source-storage/RESULTS.md) measures all
four conditions and 36 ownership phases, verifies no-op sharing, expired external pins,
restoration and final weak-handle release. Source buffers dominate whole-version memory;
line-index capacity is also material. Fixed history count does not bound a current source
set or externally owned data, and released buffers need not lower RSS immediately.

The next step is [fallible bounded source reads](bounded-source-reads.md) over immutable
backing, plus explicit verified full-input loading. Storage layout/index compression
and disk reclamation remain separate measured choices. Source/accounting defaults are
unchanged; normal and complete affected mutation checks are recorded in the report.
