# Share verified source during capture and analysis

Implemented: [contract](../prototype/CAPTURED_SOURCE.md) and
[measurements](../../experiments/shared-source/RESULTS.md).

The [retention measurements](../../experiments/retention/RESULTS.md) still show full
recovery delaying ordinary edits. Before this change, full capture held a byte buffer per file while document construction
copied UTF-8 text and built another line index.
Context reparsing repeats that storage alongside the pinned baseline.

## Ownership change

Introduce a portable immutable `CapturedSource`: content identity, owned UTF-8 or raw
bytes, encoding availability, and the cached line index. Capture accepts an owned byte
vector. Safe `String::from_utf8` keeps that vector's allocation; unsupported encoding
keeps the original raw bytes. No unsafe conversion or global interning is needed.

Documents validate analyses against this source and share its `Arc`. Their existing
borrowed-byte constructor remains a convenience wrapper. Keep document version v2 and
snapshot composition v4; source-storage identity is not observation identity.

Native capture reads one file into a temporary buffer, hashes its actual bytes, then
reuses the matching baseline source or owns the new buffer. It never treats metadata
as proof. Source sharing is independent of classification/context, so changed analysis
can reference verified old bytes without reusing old facts. Jobs and documents share
the same source object; parser ownership and the bounded syntax cache stay separate.

## Capture and interruption

Add a cancellable capture entry point used by the engine, preserving full/partial
discovery, ancestor/symlink safeguards, errors, and conservative fallback. Check before
filesystem work, between entries/components/reads, and after a file is captured. A
cancelled capture returns no candidate and leaves published observations untouched.
Blocking I/O, hashing, UTF-8 validation, and cleanup still have no hard deadline.

The temporary read overlap becomes one file at a time. A cold or wholly changed
workspace still needs all its candidate source, and old/new different contents can
coexist. This does not impose a byte cap, stream analysis through a bounded queue, or
change recovery scheduling. Unsupported-encoding documents now retain captured raw
bytes internally; textual/pinned HTTP responses remain unavailable for that encoding.

## Verification and measurements

Use TDD for buffer adoption, unchanged recapture sharing, same-size changed bytes,
UTF-8/unsupported input, line ranges, document validation/version equivalence, sharing
across analysis contexts, and cancelled/full/partial native capture. Existing tests
must preserve stale-ticket rules, dirty hints, old pins, clean-capture agreement,
source policy, language selection, and syntax retention.

Record fresh-process cached loads, retained full verification, and retained context
reparse on full Linux/PyTorch before and after this change, with identical profiles,
four workers, warm OS caches, and separate timing/correctness jobs. Report RSS samples
and time separately. Recheck the browser's recovery outlier without claiming that
source sharing changes scheduling or bounds all allocations.
