# Verify streamed source before reusing metadata

The [streaming capture measurements](../../experiments/stream-capture/RESULTS.md) show
an unchanged-pass regression from constructing candidate metadata before comparing
identity. Avoid that work while retaining actual-content verification, bounded scratch
and immutable previous captures.

## Verification seam

The portable captured-source module owns an incremental verifier for an existing
capture. It borrows captured identity/length, accepts ordered byte slices and consumes
itself to compare at logical EOF. It retains only hash state and remaining length;
it never reads backing, allocates line indexes or retains input bytes.

Initialise the existing namespaced, length-prefixed hash with the previous captured
length. That length is a candidate, not proof: reuse requires both the exact observed
byte count and matching actual-byte digest after EOF. Extra or missing bytes always
reject reuse, regardless of chunking. Keep the identity composition inside the portable
module, shared with known-length capture; native IO must not reproduce it.

## Native integration

Continue staging all project bytes so changed inputs need no second read of a mutable
project file. Feed the verifier from the same read slices. Only after actual EOF and
cancellation checks may successful verification return the original source/metadata.
Skip staged metadata scans and committed appends in that case, including at full quota.

Otherwise build metadata and identity from actual staged length using the existing
bounded capture path, then append/publish as before. Remove the redundant later identity
reuse path. Staging cleanup, error handling, commit rollback, old pins, cumulative disk
capacity and separate staging capacity retain their existing contract. Different-length
inputs cannot use the speculative digest; the normal staged path hashes their actual length.

This adds input-pass hashing for changed inputs with a previous capture. Measure that
cost alongside unchanged captures. Staging IO remains; this is not metadata-only change
detection or a whole-process/index/disk reclamation bound. Resident capture stays default.

## Validation

Use TDD for empty/binary/Unicode inputs, arbitrary partitions, exact/short/extra bytes,
same-length changes, backing-independent verification, and irreversible length mismatch.
Native tests must preserve full-quota reuse, EOF/error/cancellation handling and exact
new metadata for changed, shortened or lengthened inputs. Retain existing immutable pin,
rollback, bounded-read and cleanup coverage.

Run workspace/doctest/fmt/Clippy/browser checks plus complete relevant core/native mutation
scopes. Report compiler failures, timeouts and exclusions separately from test catches.
Repeat the optimised 512 MiB fixture and pinned Linux/PyTorch history workloads without
competing validation jobs. Preserve raw observations/source/binary hashes and distinguish
historical comparisons from paired current-code results.

## Delivery evidence

The [before/after measurements](../../experiments/verified-reuse/RESULTS.md) preserve
source manifests, sharing, old pins, disk/owned-buffer accounting and restoration.
Unchanged capture decreases from 3,674 to 2,172 ms Linux and 569 to 263 ms PyTorch;
opposite-order large-file pairs confirm the same direction. Workspace/browser checks
and complete relevant mutation scopes pass with no survivors, with timeouts and unviable
outcomes reported separately. Staging IO and changed-input speculative hashing remain
explicit costs; reclamation and resident/process admission are separate design work.
