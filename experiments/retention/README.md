# Measure repeated edits with bounded syntax retention

Compare disabled/enabled retention through production capture, extraction, validation,
and snapshot construction. Original archives stay unchanged; no analysed project runs.
The [recorded results](RESULTS.md) distinguish engine, browser, and memory observations.

```sh
cargo build --release -p vizzi --offline
cargo build --release -p vizzi --features qualification --example retain --offline
python3 experiments/retention/measure.py \
  --cache /tmp/vizzi-retention-cache --output /tmp/vizzi-retention-native.json
```

The default covers both pinned [profiles](../qualification/profiles.json).
Use `--profile linux` or `--profile pytorch` for one. The driver prepares current-context
analysis caches before timing, makes disposable copies, and writes the ownership marker
required by the native probe. Existing outputs/configurations are not overwritten.
Run without competing builds, tests, mutation jobs, or benchmarks.

Each profile selects an ordinary source file and its largest assigned header. Separate
processes compare zero/512 MiB weight. Each file/position has a new retention cache,
one recorded priming edit, and three subsequent edits. Positions are the start, a line
boundary near the middle, and the end. Unique per-condition probe IDs ensure actual
parsing rather than persisted-cache hits.

Every case reads/parses one file, avoids reconciliation and persisted hits, matches an
independent fresh parse bypassing the analysis cache, and matches a complete capture.
The latter additionally checks source scope, document identity, and snapshot construction.
Original targets are restored between positions and verified before removing copies.

Results separate priming, repeated edits, engine times, tree reuse, weights, idle
source/node counts, point RSS before verification, and sampled process peaks. Peaks
include verification and cannot be attributed solely to measured updates. Admission
weight is not a heap measurement or process cap; this host is not the 16 GB laptop.

The prototype watcher/stdio driver also accepts `--syntax-cache-mib 512` and creates
fresh probe IDs. Recovery stays active there; sampled metadata may describe a later
attempt. For browser measurement, append `--syntax-cache-mib`, `512` to a temporary
profile's `source_arguments` and set its `cache` to the prepared cache. See
[browser validation](../browser/README.md). These timings exclude GPU completion and
physical presentation.
