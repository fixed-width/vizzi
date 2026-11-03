# Measure streamed source capture

Use the pinned Linux/PyTorch qualification archives listed in
[`profiles.json`](../qualification/profiles.json). The driver creates marked owned copies
and restores edited targets; it never runs analysed project tooling. Run from the workspace
root without competing builds, tests, mutation jobs or benchmarks.

```sh
cargo build --release --offline -p vizzi --features qualification \
  --example stream_capture --example source_storage --bin vizzi
python3 experiments/stream-capture/measure.py \
  --fixture-mib 512 --output results/local/stream-capture.json
```

Choose a new output path. The driver preserves per-condition configurations, stderr,
progress and completed reports beside the final JSON. Fixture size accepts 1–1024 MiB.
Only temporary directories made by the driver are edited or removed.

The generated fixture is one large UTF-8 line of spaces. Separate sequential child
processes compare whole-vector capture plus persistence with streaming staging, using
the same current metadata builder. Each captures the file twice and verifies content
identity, encoding, line count, old-source sharing and complete bytes through bounded
reads. Memory is sampled about every 50 ms; capture checkpoints also include process
RSS and lifetime high-water memory. A 2 GiB child virtual address-space limit is applied.
This fixture isolates capture; it performs no parsing, fact extraction or UI work.

Each corpus repeats the existing source-storage whole-scope history workflow with
streaming enabled and a 16 GiB committed spool allowance. It holds an expired baseline
pin, creates four changed source generations, releases publications/pins, restores source
and checks snapshots and bytes. The child virtual address-space limit is 12 GiB. See the
[storage procedure](../source-storage/README.md) for phase definitions. The preceding
[whole-vector backing observations](../source-reads/RESULTS.md) are a historical comparison;
the driver does not repeat that implementation in the corpus conditions.

Staging has a separate allowance equal to committed capacity. Their combined file
lengths may reach twice the configured allowance. Address-space limits are not RSS or
physical RAM limits. Metadata, probe verification, allocator retention and operating-system
file caching remain distinct from transient capture buffers. Single sequential conditions
do not estimate timing tails, randomised speedups or baseline-laptop performance.

See [results](RESULTS.md) for interpretation and the
[source reference](../../docs/prototype/SOURCE_READS.md) for the implemented contract.
