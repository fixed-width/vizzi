# Comparison application and transport validation

The local host now exposes create/refresh/close, independent review status/subscription,
and finite token/side/version-bound file/window/diff queries. The
[API reference](../../docs/prototype/COMPARISON_API.md) records delivered schemas and
resource limits. Comparison controls and linked browser views remain next.

## Behavioural evidence

The first native lifecycle test failed against the unimplemented application before
implementation. Application/query tests exercise:

- creating an analysed pair, exact before/after source, baseline preservation after a
  ref move, refresh token replacement and coherent held-query reads;
- invalid/failed requests preserving active state, latest queued intent, wrong-token
  refusal, close/shutdown and subscription closure after application drop;
- literal subtree scope, projection filters preserving complete inventory totals,
  stable 200-row pages, extreme offsets and long path labels;
- 201 independent hunks/edits paged without silent loss, exact side versions, absent
  versus empty source and existing 96-row/32-KiB source bounds;
- executable-only changes, literal symlink values, binary text refusal and sparse
  absence remaining unassessed rather than deleted;
- context/diagnostic UTF-8 bounds, independent truncation flags, hidden counts,
  request length boundaries and captured-time intervals;
- shared HTTP/stdio results, typed error/status distinctions, JSON-only mutations,
  finite-diagnostic stream refusal and independent SSE;
- query saturation refusal without work and one-at-a-time diff admission.

Publication observers update application labels/status while the native store accepts
the same generation. Capture, projection, source work and payload destruction remain
outside those locks. An immutable file index avoids deriving/scanning every file ID
for each selected source lookup. Native request cancellation and shutdown share the
store's authoritative desired ticket.

## Frozen input replay

The [recorded transport replay](results/transport-fixtures.json) uses the normal native
binary in finite stdio mode. Requests execute the same HTTP handlers, without a socket
or browser. Both variants match the frozen two-path change inventory and three/two Rust
hunks. Source rows are compared with the actual pinned/working bytes, and declaration
windows expose `compile_file` and `read_file`. The Markdown change remains added text.

The replay also checks exact version refusal, same-content refresh with a new token,
unchanged pinned baseline/comparison identity, retired-token refusal, close and finite
event-stream rejection. Fixture status/bytes still verify after the host exits. No
analysed-project build/script runs. Native startup uses one worker and lower priority.

Reproduce with:

```sh
CARGO_BUILD_JOBS=1 cargo build -p vizzi --offline
python3 experiments/review-workflow/verify_transport.py \
  --fixture /tmp/vizzi-review-a --output /tmp/vizzi-transport-a
python3 experiments/review-workflow/verify_transport.py \
  --fixture /tmp/vizzi-review-b --output /tmp/vizzi-transport-b
```

Prepare the disposable fixtures via [the guide](README.md). Choose new output directories
outside each fixture workspace. The runner retains `transport.json` and `host.log`;
its temporary external cache is removed. Finite status probes synchronise this diagnostic
replay; browser integration will consume the review SSE stream.

## Verification scope and limitations

The workspace suite passes **397 tests**, including doctests. Formatting and workspace
Clippy with all targets/features and denied warnings pass. Mutation outcomes are:

| Run | Tested | Assertion catches | Survivors | Timeouts | Unviable |
| --- | ---: | ---: | ---: | ---: | ---: |
| Initial new modules | 171 | 71 | 46 | 4 | 50 |
| Final modules and changed native callers | 202 | 135 | 1 | 6 | 60 |
| Executable follow-up | 1 | 1 | 0 | 0 | 0 |

These overlapping runs are not additive. Of the initial 46 survivors, 42 are caught
by the final assertions, one is detected by the timeout, and three mutation sites
were removed by consolidating cancellation and UTF-8 clipping. The final integration
survivor replaced `main` with success: that run selected library/native tests but
omitted the CLI suite. The focused CLI follow-up catches it. No equivalent-mutation
exclusions were needed.

One attempted run stopped at its unmodified baseline, before testing any mutations.
The HTTP error table found source-read errors mapped to 422 after removal of their
explicit 503 arm. Restoring the arm fixed that baseline. It is recorded separately.

The [mutation report](results/transport-mutation.json) records full new application,
projection and HTTP modules, changed native-store/diagnostic caller checks, baselines,
commands, source/test hashes and outcome categories. Runs use one mutant, one compiler/
jobserver task, serial tests and lower priority. Early survivors prompted explicit
limit, clipping, availability, edit-page, error and shutdown assertions. Redundant
service cancellation state was replaced by authoritative ticket invalidation.
Timeouts and unviable cases remain separate from assertion catches.

This is correctness evidence, not a browser walkthrough, comprehension study or tail
latency/RSS qualification. Paged diffs currently recompute within fixed limits; no
diff cache or production-renderer claim is made. The linked review UI and complete
good/bad task are the next milestone output.
