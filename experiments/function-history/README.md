# Qualify function history and captured calls

Contributor procedure for [issue #5](https://github.com/fixed-width/vizzi/issues/5).
The [results](RESULTS.md) describe observations and limits separately from this procedure.
These tools use production HTTP routes and Python's standard library. They never build,
import or run the inspected projects. Rust correctness fixtures and Chromium checks remain
separate from these optimised-build observations.

## Prepare inputs

Run from the Vizzi checkout. [profiles.json](profiles.json) pins the revisions and functions.
Linux and PyTorch reuse the explicit raw-language assignments and exclusions in
[the full-corpus profiles](../qualification/profiles.json). They do not supply compiler
configuration, include paths or active preprocessor branches. Submodules are not expanded.
LPC source is not assigned C syntax in the smaller lpc-rs reference.

Use a disposable destination outside the originals and keep the original checkouts until
qualification finishes: local clones can borrow their Git objects. Shallow local clones
may copy objects even with `--shared`. Preparation fetches bounded history from upstream
as ordinary, unfiltered Git objects; network access is needed here. It verifies reachable
object completeness and every selected historical blob before timing. Partial/promisor
clones are unsupported history inputs, even when the selected blobs happen to be cached.
The original checkout, index, refs and shallow boundary are left unchanged. No checkout,
fetch or edit occurs during measurement.

```sh
python3 experiments/function-history/prepare.py --profile linux \
  --original /path/to/linux --destination /tmp/vizzi-history-linux
python3 experiments/function-history/prepare.py --profile pytorch \
  --original /path/to/pytorch --destination /tmp/vizzi-history-pytorch
python3 experiments/function-history/prepare.py --profile lpc-rs \
  --original /path/to/lpc-rs --destination /tmp/vizzi-history-lpc
```

The selected revision must exist locally for lpc-rs. Fetch that exact revision into the
disposable checkout if preparing from a different local history. Preparation writes
`DESTINATION.preparation.json` with original before/after identities, commands, selected
file revisions and blob IDs. Destinations and result directories must not already exist.
`history_depth` is Git's ancestry-depth request, not a cap on downloaded commits or bytes;
merge histories can make acquisition substantially larger than that number suggests.
The [C++ supplement](cpp.json) selects a changed function in the same PyTorch file/history;
the main profile retains an unchanged C++ function as a shallow-boundary control.

## Validate the collector and product

```sh
python3 -m unittest discover -s experiments/function-history -p 'test_*.py'
cargo fmt --all -- --check
CARGO_BUILD_JOBS=1 RUST_TEST_THREADS=1 nice -n 10 \
  cargo test --release --workspace --locked --offline
CARGO_BUILD_JOBS=1 nice -n 10 \
  cargo clippy --workspace --all-targets --locked --offline -- -D warnings
CARGO_BUILD_JOBS=1 nice -n 10 cargo build --release --locked --offline -p vizzi
```

The collector checks source pins and literal fragments against independent `git show`
bytes outside each measured interval. Its unit tests reject mismatched source and verify
that refusals never enter successful latency summaries. No product implementation changes
are needed for this experiment; native mutation evidence remains in the feature contracts.

## Collect one condition at a time

Stop builds, tests, other measurements and history acquisition before collection. The
following defaults are three fresh native processes, five samples per function, four
analysis workers and discovery through the available history or the 64-file-revision
boundary. The first process has an empty analysis cache; later processes reuse that cache.
Initial indexing is reported separately. Each process has fresh history/comparison owners.
OS page caches are not flushed; these are process/operation-cold observations, not cold-disk
observations. Inspection and acquisition have already warmed some inputs.

```sh
nice -n 10 python3 experiments/function-history/measure.py --profile linux \
  --root /tmp/vizzi-history-linux --output /tmp/vizzi-history-linux-results --capacity
nice -n 10 python3 experiments/function-history/measure.py --profile pytorch \
  --root /tmp/vizzi-history-pytorch --output /tmp/vizzi-history-pytorch-results --capacity
nice -n 10 python3 experiments/function-history/measure.py --profile pytorch \
  --profiles experiments/function-history/cpp.json \
  --root /tmp/vizzi-history-pytorch --output /tmp/vizzi-history-pytorch-cpp-results
nice -n 10 python3 experiments/function-history/measure.py --profile lpc-rs \
  --root /tmp/vizzi-history-lpc --output /tmp/vizzi-history-lpc-results --capacity
nice -n 10 python3 experiments/function-history/limits.py \
  --output /tmp/vizzi-history-limits
```

`--runs`, `--samples`, `--workers` and `--depth` allow an explicitly reported alternative.
`--cache DIR` reuses a prior analysis cache and records each process's cache as seeded.
History and comparison owners are still new in every process. The recorded Linux trials
use this option after a separate cold index; their first operation timings include that
distinction.
`--capacity` adds 25 history admissions and 13 comparison admissions for the first function
of the first process, then checks that old windows and captured-call bindings expire.
The independent limit fixture uses one worker and checks inclusive 2 MiB files, 32 MiB
history and pair allowances, 24 sessions, 12 pairs, depth and shallow boundaries. Its
timings describe fixture operations only and do not set real-repository latency budgets.

The full-corpus native endpoints can accept captured declaration identities from paged
source outlines even when the bounded live function inventory omits those files. The
collector records inventory coverage before using that path. This qualifies those native
operations; it does not prove that a user can open the same function through Add bubble.

## Read the evidence

Each result directory contains `result.json`, native logs and its derived-data cache.
Keep JSON/logs as evidence; caches and disposable checkouts can be removed afterwards.
The JSON records exact native commands, binary and collector SHA-256, source revisions,
compiler/runtime, hardware, CPU affinity, initial indexing and operation requests. Large
source projections retain line numbers and hashes rather than project source text. Git
byte checks run during collection, before compaction.

Every HTTP observation has its status, duration, response size and phase. Successful
definition responses can still be unresolved or ambiguous; inspect their state, candidates
and coverage separately from successful destination-open timings. Calls are paged fully;
trails choose the first available candidate in native order, explicitly recording that
choice. This is same-file captured evidence, without cross-file or semantic resolution.

RSS is sampled approximately every 20 ms and at operation completion. Sampled maxima are
lower bounds; `VmHWM` covers the whole process lifetime, including indexing and preparation.
Logical source allowances do not include parser trees, metadata, response projections,
temporary captures, allocation overhead or the live repository. Child Git memory and
browser memory/rendering are outside native RSS observations.

During measured work a background probe requests status and a pinned live source window
roughly every 100 ms, checking that the snapshot stays unchanged. Its HTTP latency measures
native service responsiveness, not browser presentation or interactive frame rate. The
probe and sampler add load. Refusals, unsupported analysis and incomplete inventories stay
in the raw evidence and outside successful latency summaries.

Export per-function summaries and a complete operation table:

```sh
python3 experiments/function-history/summarise.py \
  /tmp/vizzi-history-linux-results/result.json \
  /tmp/vizzi-history-pytorch-results/result.json \
  /tmp/vizzi-history-pytorch-cpp-results/result.json \
  /tmp/vizzi-history-lpc-results/result.json \
  --output /tmp/vizzi-history-summary
```

The exporter also accepts `.json.gz` evidence files. Its medians and maxima include only
HTTP 200 observations; refusal counts remain grouped by HTTP status.
