# Measure automatically scheduled source maintenance

Use the pinned archives and compatible fact cache from the
[foreground procedure](../foreground-maintenance/README.md). Run from the workspace root:

```sh
cargo build --release --offline -p vizzi --features qualification \
  --example idle_maintenance --bin vizzi
python3 experiments/idle-maintenance/measure.py \
  --cache /tmp/vizzi-foreground-facts \
  --output /tmp/vizzi-idle-measurements/observations.json
```

Choose a new output path and finish builds, tests, mutation jobs and cache preparation
before timing. The driver creates marked owned copies, restores original targets and
runs no analysed project tooling. Original archives remain read-only.

The real Host idle scheduler admits cleanup. An observed maintenance provider delegates
the same spool work hint and cleanup operation, reporting actual generated-tail cuts.
A minimum-removal filter excludes unrelated small cleanup. No manual cleanup thread or
artificial truncate delay is used. Host options enable maintenance, disable native watches
and set an hour-long recovery interval after initial verification. Explicit path hints
use the ordinary coordinator/capture/analysis/publication path.

Three alternating condition pairs compare edits after completed cleanup with edits after
observed truncate entry. Generated tails are 4 GiB Linux / 768 MiB PyTorch. Tail preparation
can satisfy the quiet interval, so retirement-to-entry mainly observes polling/admission;
it is not a fresh user-idle timeout measurement. Raw data records admission, cut, write,
capture and publication intervals plus native maintenance counters and shutdown time.
Actual overlap is checked rather than assumed. Source/facts/declarations/old pins and
restored clean observations must match; final backing length is zero after host/pins close
and explicit verification cleanup runs.

Shared backing allowances are 8 / 1 GiB, with separate equal staging allowances and a
12 GiB child virtual-address-space limit. These do not qualify RSS/physical RAM or baseline
hardware. Pressure setup and independent verification are outside edit latency but enter
process peaks. Warm native samples do not establish watcher/browser tails, cold-storage
performance, blocked shutdown deadlines or representative retirement frequencies.

See [results](RESULTS.md), [scheduler design](../../docs/plans/idle-maintenance.md) and
[native recovery reference](../../docs/prototype/RECOVERY.md) for interpretation.
