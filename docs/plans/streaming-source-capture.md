# Stream private-backed capture with bounded transient buffers

The [read/backing stage](../../experiments/source-reads/RESULTS.md) removes permanent
resident bodies but still reads a whole project file before spooling. Replace that
opt-in path while preserving resident capture as the default.

## Capture and identity

Read project input into a private reusable staging file in at most 64 KiB requests,
checking cancellation between operations. Observe EOF and actual length; metadata alone
is never content proof. Build content identity, UTF-8 availability, line starts, CRLF bits
and fixed-block integrity digests over those staged bytes with bounded scratch. Keep
ContentId's existing namespace/length-prefix composition. New metadata capture hashes
using the final actual length. The subsequent [verified reuse](verified-source-reuse.md)
path hashes speculatively with the previous captured length and accepts it only after EOF
proves that exact observed count and digest. Never treat a guessed length as proof.

Share metadata construction between resident and staged capture. UTF-8 validation must
carry incomplete code points across blocks, including final truncation and invalid input.
CRLF across blocks and bare final CR retain existing display semantics. Metadata remains
input-size-dependent; this change bounds transient byte/validation buffers, not indexes.

After verification, matching previous content returns its existing immutable source and
backing without appending. Changed content is copied into the committed spool in bounded
chunks and published only after completion. A writer gate serializes capture commits and
staging reuse, while the committed-file mutex is held only for individual chunk IO;
existing source readers can continue between chunks. No IO or backing work occurs under
the publication lock. Old descriptors are never overwritten.

## Limits and cancellation

The existing configured capacity bounds committed session appends. One serialized staging
file is separately bounded by that same capacity. Combined file lengths are therefore
at most twice the configured capacity; this is disk space, not resident memory. Staging
is truncated on observed cancellation, error, unchanged reuse and successful completion
(cleanup is best effort if truncation itself fails).
Uncommitted append tails are rolled back without changing earlier committed ranges.
Cancellation observed before commit does not consume committed quota. Late cancellation
after commit can still leave an unreferenced committed capture; session reclamation is
separate work.

A full committed spool can still stage and verify unchanged inputs. A new changed capture
that cannot commit returns a capacity error and preserves the old publication/hints.
An individual input larger than staging capacity is refused without growing staging
beyond its bound. Blocking IO, mutex acquisition and destructor work have no hard deadline.
No unsafe code, additional dependencies or new user-visible configuration are required.

## Verification and measurement

Use TDD for all chunk sizes, Unicode boundaries, invalid/truncated UTF-8, CRLF splits,
empty source, exact capacities, oversized input, read/write failure, interrupted reads,
cancellation at staging/metadata/commit, unchanged reuse at full committed capacity,
concurrent readers/capture, old pins and cleanup. Compare complete metadata/windows and
identity with resident capture. Run workspace/doctest/fmt/Clippy/browser checks and broad
relevant mutation scopes, recording unviable/timeouts separately.

Compare current streamed capture with the preceding whole-file backed capture using
representative Linux/PyTorch sources plus a generated large single-file case. Record
read request/scratch bounds, staging/commit quota, input bytes, time, process memory and
independent source/encoding/line/pin checks. Keep benchmarks separate from validation.
Metadata, parser/fact/response allocations, allocator retention, disk reclamation and
whole-process enforcement remain outside this increment.

## Delivery evidence

The implementation and [measurements](../../experiments/stream-capture/RESULTS.md) preserve
old pins and actual-content verification while bounding transient byte buffers. Streaming
reduces the generated 512 MiB file's sampled peak RSS from 515.0 to 3.25 MiB. Representative
history checks pass, but unchanged captures are slower because candidate metadata is
rebuilt before identity comparison. Investigate verified unchanged-source metadata reuse
through the [next stage](verified-source-reuse.md); disk reclamation and
resident/index/process limits remain separate design work.
